//! Runs a list of conversions one after another.

use crate::error::ConvertError;
use crate::model::{self, backend_for, Backend, Kind, Options, Target};
use crate::{ffmpeg, image_conv, naming, office, pdf, Backends};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct Request {
    pub files: Vec<PathBuf>,
    pub target: Target,
    pub options: Options,
    /// None writes next to each source.
    pub out_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub src: PathBuf,
    pub status: Status,
    pub dst: Option<PathBuf>,
    pub message: Option<String>,
    pub notes: Vec<String>,
    pub in_bytes: u64,
    pub out_bytes: u64,
    /// More than one source went into this result (pictures combined into one PDF).
    pub sources: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Started { index: usize, src: PathBuf },
    Progress { index: usize, fraction: f32 },
    Finished { index: usize, outcome: Outcome },
}

fn failed(src: &Path, e: ConvertError) -> Outcome {
    Outcome {
        src: src.to_path_buf(),
        status: if matches!(e, ConvertError::Cancelled) {
            Status::Cancelled
        } else {
            Status::Failed
        },
        dst: None,
        message: Some(e.to_string()),
        notes: vec![],
        in_bytes: 0,
        out_bytes: 0,
        sources: 1,
    }
}

/// Checks a request before anything runs, so a bad one fails as a whole with one sentence.
pub fn check(req: &Request, backends: &Backends) -> Result<(), ConvertError> {
    if req.files.is_empty() {
        return Err(ConvertError::Other("Add some files first.".into()));
    }
    if req.files.len() > 5000 {
        return Err(ConvertError::Other(
            "That's too many files at once. Try 5000 or fewer.".into(),
        ));
    }
    if let Some(dir) = &req.out_dir {
        if !crate::output_dir_ok(dir) {
            return Err(ConvertError::Other(
                "That output folder isn't usable.".into(),
            ));
        }
    }
    for f in &req.files {
        if !f.is_absolute() {
            return Err(ConvertError::Other("File paths must be complete.".into()));
        }
        let Some(kind) = Kind::of(f) else {
            return Err(ConvertError::Other(format!(
                "{} is a type NOMNOM can't convert.",
                name(f)
            )));
        };
        if !model::valid_for(f, req.target) {
            return Err(ConvertError::Other(format!(
                "{} can't be turned into {}.",
                name(f),
                req.target.label()
            )));
        }
        let backend = backend_for(kind, req.target);
        if !backends.has(backend) {
            return Err(ConvertError::MissingTool(match backend {
                Backend::Ffmpeg => "FFmpeg",
                Backend::Office => "LibreOffice",
                _ => "A converter",
            }));
        }
    }
    if req.options.combine
        && !(req.target == Target::Pdf
            && req.files.iter().all(|f| Kind::of(f) == Some(Kind::Image)))
    {
        return Err(ConvertError::Other(
            "Only pictures can be combined into one PDF.".into(),
        ));
    }
    Ok(())
}

fn name(p: &Path) -> String {
    p.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string()
}

pub async fn run(
    req: Request,
    backends: Backends,
    cancel: CancellationToken,
    on_event: Arc<dyn Fn(Event) + Send + Sync>,
) -> Vec<Outcome> {
    let options = req.options.clone().sanitized();
    let mut results = Vec::new();

    if options.combine {
        on_event(Event::Started {
            index: 0,
            src: req.files[0].clone(),
        });
        let outcome = combine_images(&req, &options, &cancel).await;
        on_event(Event::Finished {
            index: 0,
            outcome: outcome.clone(),
        });
        results.push(outcome);
        return results;
    }

    for (index, src) in req.files.iter().enumerate() {
        if cancel.is_cancelled() {
            let mut o = failed(src, ConvertError::Cancelled);
            o.message = Some("Not started.".into());
            on_event(Event::Finished {
                index,
                outcome: o.clone(),
            });
            results.push(o);
            continue;
        }
        on_event(Event::Started {
            index,
            src: src.clone(),
        });
        let progress_cb = {
            let on_event = on_event.clone();
            move |fraction: f32| on_event(Event::Progress { index, fraction })
        };
        let outcome = match convert_one(src, &req, &options, &backends, &cancel, &progress_cb).await
        {
            Ok(o) => o,
            Err(e) => failed(src, e),
        };
        on_event(Event::Finished {
            index,
            outcome: outcome.clone(),
        });
        results.push(outcome);
    }
    results
}

fn dest_dir(src: &Path, out_dir: &Option<PathBuf>) -> Result<PathBuf, ConvertError> {
    let dir = match out_dir {
        Some(d) => d.clone(),
        None => src
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| ConvertError::Other("This file has no folder.".into()))?,
    };
    std::fs::create_dir_all(&dir).map_err(|e| ConvertError::io(&dir, e))?;
    Ok(dir)
}

async fn convert_one(
    src: &Path,
    req: &Request,
    options: &Options,
    backends: &Backends,
    cancel: &CancellationToken,
    progress: &(dyn Fn(f32) + Send + Sync),
) -> Result<Outcome, ConvertError> {
    let meta = std::fs::symlink_metadata(src).map_err(|e| ConvertError::io(src, e))?;
    if !meta.is_file() {
        return Err(ConvertError::Other("This isn't an ordinary file.".into()));
    }
    let kind = Kind::of(src)
        .ok_or_else(|| ConvertError::Other("NOMNOM can't convert this type of file.".into()))?;
    let dir = dest_dir(src, &req.out_dir)?;
    let ext = naming::out_ext(src, req.target);
    let tmp = naming::temp_for(&dir, &ext);
    let _cleanup = TempGuard(tmp.clone());

    let notes = match backend_for(kind, req.target) {
        Backend::Image => {
            let (s, t, target, o) = (src.to_path_buf(), tmp.clone(), req.target, options.clone());
            tokio::task::spawn_blocking(move || image_conv::convert(&s, &t, target, &o))
                .await
                .map_err(|_| ConvertError::Other("The conversion stopped unexpectedly.".into()))??
        }
        Backend::Pdf => write_text_like(src, &tmp, req.target).await?,
        Backend::Ffmpeg => {
            let ffmpeg_exe = backends
                .ffmpeg
                .as_deref()
                .ok_or(ConvertError::MissingTool("FFmpeg"))?;
            let ffprobe_exe = backends
                .ffprobe
                .as_deref()
                .ok_or(ConvertError::MissingTool("FFmpeg"))?;
            let probe = ffmpeg::probe(ffprobe_exe, src).await?;
            let plan = ffmpeg::plan(req.target, options, &probe, src, &tmp)?;
            ffmpeg::run(ffmpeg_exe, &plan, cancel, progress).await?;
            plan.notes
        }
        Backend::Office => {
            let soffice = backends
                .soffice
                .as_deref()
                .ok_or(ConvertError::MissingTool("LibreOffice"))?;
            office::to_pdf(soffice, src, &tmp, cancel).await?;
            vec![]
        }
    };

    validate(&tmp, kind, req.target, backends, 1).await?;
    let out_bytes = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    let dst = naming::place(&tmp, &dir, &naming::stem_of(src), &ext)
        .map_err(|e| ConvertError::io(&tmp, e))?;
    progress(1.0);
    Ok(Outcome {
        src: src.to_path_buf(),
        status: Status::Done,
        dst: Some(dst),
        message: None,
        notes,
        in_bytes: meta.len(),
        out_bytes,
        sources: 1,
    })
}

/// Removes our own temporary file when a conversion ends any way other than a successful move.
struct TempGuard(PathBuf);
impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn write_text_like(
    src: &Path,
    tmp: &Path,
    target: Target,
) -> Result<Vec<String>, ConvertError> {
    let (s, t) = (src.to_path_buf(), tmp.to_path_buf());
    tokio::task::spawn_blocking(move || -> Result<Vec<String>, ConvertError> {
        let md = model::is_markdown(&s);
        match target {
            Target::Pdf if model::Kind::of(&s) == Some(Kind::Image) => {
                let (bytes, notes) =
                    pdf::images_to_pdf(&[s.as_path()], &Options::default(), &|| false)?;
                std::fs::write(&t, bytes).map_err(|e| ConvertError::io(&t, e))?;
                Ok(notes)
            }
            Target::Pdf => {
                let text = pdf::read_text(&s)?;
                let (bytes, notes) = if md {
                    pdf::markdown_to_pdf(&text)
                } else {
                    pdf::text_to_pdf(&text)
                };
                std::fs::write(&t, bytes).map_err(|e| ConvertError::io(&t, e))?;
                Ok(notes)
            }
            Target::Html => {
                let text = pdf::read_text(&s)?;
                let title = naming::stem_of(&s);
                std::fs::write(&t, pdf::markdown_to_html(&text, &title))
                    .map_err(|e| ConvertError::io(&t, e))?;
                Ok(vec![])
            }
            _ => Err(ConvertError::Other("Not a document format.".into())),
        }
    })
    .await
    .map_err(|_| ConvertError::Other("The conversion stopped unexpectedly.".into()))?
}

async fn combine_images(req: &Request, options: &Options, cancel: &CancellationToken) -> Outcome {
    let first = req.files[0].clone();
    let result: Result<Outcome, ConvertError> = async {
        let dir = dest_dir(&first, &req.out_dir)?;
        let tmp = naming::temp_for(&dir, "pdf");
        let _cleanup = TempGuard(tmp.clone());
        let files = req.files.clone();
        let o = options.clone();
        let flag = cancel.clone();
        let (bytes, notes) = tokio::task::spawn_blocking(move || {
            let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
            pdf::images_to_pdf(&refs, &o, &|| flag.is_cancelled())
        })
        .await
        .map_err(|_| ConvertError::Other("The conversion stopped unexpectedly.".into()))??;
        std::fs::write(&tmp, &bytes).map_err(|e| ConvertError::io(&tmp, e))?;
        let pages = pdf::validate_pdf(&tmp, req.files.len())?;
        if pages != req.files.len() {
            return Err(ConvertError::BadOutput("The PDF doesn't have every page."));
        }
        let in_bytes = req
            .files
            .iter()
            .filter_map(|f| std::fs::metadata(f).ok())
            .map(|m| m.len())
            .sum();
        let stem = format!(
            "{} and {} more",
            naming::stem_of(&first),
            req.files.len() - 1
        );
        let stem = if req.files.len() == 1 {
            naming::stem_of(&first)
        } else {
            stem
        };
        let dst = naming::place(&tmp, &dir, &stem, "pdf").map_err(|e| ConvertError::io(&tmp, e))?;
        Ok(Outcome {
            src: first.clone(),
            status: Status::Done,
            dst: Some(dst),
            message: None,
            notes,
            in_bytes,
            out_bytes: bytes.len() as u64,
            sources: req.files.len(),
        })
    }
    .await;
    result.unwrap_or_else(|e| {
        let mut o = failed(&first, e);
        o.sources = req.files.len();
        o
    })
}

async fn validate(
    path: &Path,
    kind: Kind,
    target: Target,
    backends: &Backends,
    min_pages: usize,
) -> Result<(), ConvertError> {
    match target {
        Target::Pdf => {
            pdf::validate_pdf(path, min_pages)?;
        }
        Target::Html => {
            let ok = std::fs::read_to_string(path)
                .map(|s| s.contains("<body>"))
                .unwrap_or(false);
            if !ok {
                return Err(ConvertError::BadOutput("The result isn't a web page."));
            }
        }
        t if kind == Kind::Image && !t.is_audio() => image_conv::validate(path, t)?,
        t => {
            let probe = backends
                .ffprobe
                .as_deref()
                .ok_or(ConvertError::MissingTool("FFmpeg"))?;
            ffmpeg::validate(probe, path, t).await?;
        }
    }
    Ok(())
}
