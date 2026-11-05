//! Word, Excel and PowerPoint files to PDF, through LibreOffice when it is installed.

use crate::error::ConvertError;
use nomnom_core::process;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub fn find_soffice() -> Option<PathBuf> {
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(var) {
            let p = PathBuf::from(base).join("LibreOffice\\program\\soffice.exe");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join("soffice.exe"))
        .find(|p| p.is_file())
}

/// Converts `src` to PDF at `tmp_out`. Works in a scratch folder next to `tmp_out`, with its own
/// LibreOffice profile so the user's own settings and running copy aren't involved.
pub async fn to_pdf(
    soffice: &Path,
    src: &Path,
    tmp_out: &Path,
    cancel: &CancellationToken,
) -> Result<(), ConvertError> {
    let parent = tmp_out.parent().unwrap_or(Path::new("."));
    let scratch = parent.join(format!(
        ".nomnom-office-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..10]
    ));
    std::fs::create_dir_all(scratch.join("profile")).map_err(|e| ConvertError::io(&scratch, e))?;
    let result = run(soffice, src, &scratch, tmp_out, cancel).await;
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

async fn run(
    soffice: &Path,
    src: &Path,
    scratch: &Path,
    tmp_out: &Path,
    cancel: &CancellationToken,
) -> Result<(), ConvertError> {
    let profile = scratch.join("profile");
    let profile_url = format!(
        "-env:UserInstallation=file:///{}",
        profile.to_string_lossy().replace('\\', "/")
    );
    let args: Vec<OsString> = vec![
        "--headless".into(),
        "--norestore".into(),
        "--nolockcheck".into(),
        profile_url.into(),
        "--convert-to".into(),
        "pdf".into(),
        "--outdir".into(),
        scratch.as_os_str().to_os_string(),
        src.as_os_str().to_os_string(),
    ];
    let mut mc = process::spawn(soffice, args, true)
        .map_err(|e| ConvertError::Tool(format!("couldn't start LibreOffice: {e}")))?;
    let outcome = {
        let work = tokio::time::timeout(Duration::from_secs(300), mc.child.wait());
        tokio::select! {
            _ = cancel.cancelled() => None,
            r = work => Some(r),
        }
    };
    match outcome {
        None => {
            mc.kill_tree().await;
            return Err(ConvertError::Cancelled);
        }
        Some(Err(_)) => {
            mc.kill_tree().await;
            return Err(ConvertError::Tool("LibreOffice took too long.".into()));
        }
        Some(Ok(Ok(status))) if status.success() => {}
        Some(_) => {
            return Err(ConvertError::Tool(
                "LibreOffice couldn't open this file.".into(),
            ))
        }
    }
    let stem = src.file_stem().unwrap_or_default().to_os_string();
    let mut produced = scratch.join(&stem);
    produced.set_extension("pdf");
    if !produced.is_file() {
        return Err(ConvertError::Tool(
            "LibreOffice didn't produce a PDF.".into(),
        ));
    }
    std::fs::rename(&produced, tmp_out).map_err(|e| ConvertError::io(tmp_out, e))?;
    Ok(())
}
