//! ZIP files: make one from files and folders, look inside one, and unpack it into a new folder.
//! Unpacking never writes outside its folder, never replaces a file, and refuses an archive that
//! would unpack to far more than it weighs. Passwords and other formats (7z, RAR) are not handled.

use crate::state::AppState;
use nomnom_core::archive as rules;
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

type CmdResult<T> = Result<T, String>;

static CANCEL: AtomicBool = AtomicBool::new(false);
const MAX_SOURCES: usize = 1000;
const MAX_LISTED: usize = 2000;

#[derive(Debug, Serialize)]
pub struct Item {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Serialize)]
pub struct Listing {
    pub items: Vec<Item>,
    pub count: usize,
    pub unpacked: u64,
    pub packed: u64,
    pub encrypted: bool,
}

fn open_zip(path: &str) -> CmdResult<zip::ZipArchive<std::fs::File>> {
    let p = Path::new(path);
    if p.extension().is_none_or(|e| !e.eq_ignore_ascii_case("zip")) {
        return Err("Only .zip files can be opened here. 7z and RAR are not supported yet.".into());
    }
    let f = std::fs::File::open(p).map_err(|_| "That file isn't there.".to_string())?;
    zip::ZipArchive::new(f).map_err(|_| "That is not a ZIP file, or it is damaged.".to_string())
}

#[tauri::command]
pub async fn ar_list(path: String) -> CmdResult<Listing> {
    tokio::task::spawn_blocking(move || {
        let mut z = open_zip(&path)?;
        let mut out = Listing {
            items: Vec::new(),
            count: z.len(),
            unpacked: 0,
            packed: 0,
            encrypted: false,
        };
        for i in 0..z.len() {
            let Ok(f) = z.by_index_raw(i) else { continue };
            out.unpacked += f.size();
            out.packed += f.compressed_size();
            out.encrypted |= f.encrypted();
            if out.items.len() < MAX_LISTED {
                out.items.push(Item {
                    name: f.name().to_string(),
                    size: f.size(),
                    is_dir: f.is_dir(),
                });
            }
        }
        rules::check_limits(out.count, out.unpacked, out.packed)?;
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Clone, Serialize)]
struct Progress {
    done: u64,
    total: u64,
}

struct Pace {
    last: Instant,
}
impl Pace {
    fn new() -> Self {
        Pace {
            last: Instant::now() - Duration::from_secs(1),
        }
    }
    fn tell(&mut self, app: &AppHandle, done: u64, total: u64, force: bool) {
        if force || self.last.elapsed() > Duration::from_millis(250) {
            self.last = Instant::now();
            let _ = app.emit("archive://progress", Progress { done, total });
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Extracted {
    pub folder: String,
    pub files: usize,
    pub bytes: u64,
}

fn free_folder(parent: &Path, name: &str) -> PathBuf {
    for n in 1..10_000u32 {
        let p = if n == 1 {
            parent.join(name)
        } else {
            parent.join(format!("{name} ({n})"))
        };
        if !p.exists() {
            return p;
        }
    }
    parent.join(format!("{name} {}", uuid::Uuid::new_v4().simple()))
}

fn unpack(
    path: &str,
    dest: &Path,
    mut tell: impl FnMut(u64, u64, bool),
) -> CmdResult<(usize, u64)> {
    let mut z = open_zip(path)?;
    let (mut unpacked, mut packed, mut encrypted) = (0u64, 0u64, false);
    for i in 0..z.len() {
        if let Ok(f) = z.by_index_raw(i) {
            unpacked += f.size();
            packed += f.compressed_size();
            encrypted |= f.encrypted();
        }
    }
    rules::check_limits(z.len(), unpacked, packed)?;
    if encrypted {
        return Err(
            "That archive has a password. Archives with passwords are not supported yet.".into(),
        );
    }
    std::fs::create_dir(dest).map_err(|e| format!("Couldn't make the folder: {e}"))?;
    let (mut files, mut bytes) = (0usize, 0u64);
    let mut buf = vec![0u8; 256 * 1024];
    for i in 0..z.len() {
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Stopped.".into());
        }
        let mut entry = z
            .by_index(i)
            .map_err(|_| "A file inside is damaged.".to_string())?;
        // A link inside an archive is not recreated: it could point anywhere.
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            continue;
        }
        let Some(rel) = rules::safe_entry(entry.name()) else {
            return Err(format!("\"{}\" has a name that would be written outside the folder, so the archive was not unpacked.", entry.name()));
        };
        let target = dest.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| format!("Couldn't make a folder: {e}"))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Couldn't make a folder: {e}"))?;
        }
        // `create_new`: nothing that exists is ever overwritten.
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| format!("Couldn't write \"{}\": {e}", entry.name()))?;
        loop {
            let n = entry
                .read(&mut buf)
                .map_err(|_| "A file inside is damaged.".to_string())?;
            if n == 0 {
                break;
            }
            bytes += n as u64;
            // The sizes in the headers can lie; what is really written is counted too.
            if bytes > rules::MAX_UNPACKED {
                return Err("That archive unpacks to more than 16 GB, so it was stopped.".into());
            }
            out.write_all(&buf[..n])
                .map_err(|e| format!("Couldn't write the file: {e}"))?;
            tell(bytes, unpacked, false);
        }
        files += 1;
    }
    tell(bytes, unpacked, true);
    Ok((files, bytes))
}

/// Unpacks into a new folder named after the archive, inside `into`.
#[tauri::command]
pub async fn ar_extract(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    into: String,
) -> CmdResult<Extracted> {
    let parent = PathBuf::from(into.trim());
    crate::folder_rules::allowed(&state, &parent)?;
    if !parent.is_dir() {
        return Err("The folder to unpack into isn't there.".into());
    }
    let name = rules::folder_name_for(
        &Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );
    let dest = free_folder(&parent, &name);
    CANCEL.store(false, Ordering::Relaxed);
    let d = dest.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut pace = Pace::new();
        unpack(&path, &d, |done, total, force| {
            pace.tell(&app, done, total, force)
        })
    })
    .await
    .map_err(|e| e.to_string())?;
    match result {
        Ok((files, bytes)) => Ok(Extracted {
            folder: dest.to_string_lossy().into_owned(),
            files,
            bytes,
        }),
        Err(e) => {
            // The folder is one this call just made, so what is in it is its own half-finished work.
            let _ = std::fs::remove_dir_all(&dest);
            Err(e)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Created {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

fn gather(source: &Path, out: &mut Vec<(PathBuf, String)>) -> CmdResult<()> {
    let meta = std::fs::symlink_metadata(source)
        .map_err(|_| format!("\"{}\" isn't there.", source.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "\"{}\" is a link, which can't be added.",
            source.display()
        ));
    }
    let base = source.parent().unwrap_or(Path::new(""));
    let name_of = |p: &Path| {
        p.strip_prefix(base)
            .unwrap_or(p)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    };
    if meta.is_file() {
        out.push((source.to_path_buf(), name_of(source)));
        return Ok(());
    }
    let mut stack = vec![source.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut empty = true;
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            empty = false;
            if ft.is_dir() {
                stack.push(e.path());
            } else if ft.is_file() {
                out.push((e.path(), name_of(&e.path())));
            }
            if out.len() > 100_000 {
                return Err("That is too many files for one archive here.".into());
            }
        }
        if empty {
            out.push((dir.clone(), format!("{}/", name_of(&dir))));
        }
    }
    Ok(())
}

fn build(
    sources: &[PathBuf],
    tmp: &Path,
    mut tell: impl FnMut(u64, u64, bool),
) -> CmdResult<(usize, u64)> {
    let mut entries: Vec<(PathBuf, String)> = Vec::new();
    for s in sources {
        gather(s, &mut entries)?;
    }
    let total: u64 = entries
        .iter()
        .filter(|(p, n)| !n.ends_with('/') && p.is_file())
        .map(|(p, _)| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();
    if total > rules::MAX_UNPACKED {
        return Err("That is more than 16 GB, too much for one archive here.".into());
    }
    let file =
        std::fs::File::create(tmp).map_err(|e| format!("Couldn't start the archive: {e}"))?;
    let mut w = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .large_file(total > 3_900_000_000);
    let (mut files, mut done) = (0usize, 0u64);
    let mut buf = vec![0u8; 256 * 1024];
    for (path, name) in &entries {
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Stopped.".into());
        }
        if name.ends_with('/') {
            w.add_directory(name.trim_end_matches('/'), opts)
                .map_err(|e| e.to_string())?;
            continue;
        }
        let mut f =
            std::fs::File::open(path).map_err(|e| format!("Couldn't read \"{name}\": {e}"))?;
        w.start_file(name.as_str(), opts)
            .map_err(|e| e.to_string())?;
        loop {
            let n = f
                .read(&mut buf)
                .map_err(|e| format!("Couldn't read \"{name}\": {e}"))?;
            if n == 0 {
                break;
            }
            w.write_all(&buf[..n])
                .map_err(|e| format!("Couldn't write the archive: {e}"))?;
            done += n as u64;
            tell(done, total, false);
        }
        files += 1;
    }
    w.finish()
        .map_err(|e| e.to_string())?
        .flush()
        .map_err(|e| e.to_string())?;
    tell(done, total, true);
    Ok((files, done))
}

/// Makes a ZIP from files and folders. The name must be new: an existing file is never replaced.
#[tauri::command]
pub async fn ar_create(
    app: AppHandle,
    state: State<'_, AppState>,
    sources: Vec<String>,
    dest: String,
) -> CmdResult<Created> {
    if sources.is_empty() || sources.len() > MAX_SOURCES {
        return Err("Choose what to put in the archive.".into());
    }
    let dest = PathBuf::from(dest.trim());
    if dest
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("zip"))
    {
        return Err("Choose a name that ends in .zip.".into());
    }
    if dest.exists() {
        return Err("A file with that name is already there. Choose another name.".into());
    }
    let dir = dest
        .parent()
        .ok_or("Choose where to save it.")?
        .to_path_buf();
    crate::folder_rules::allowed(&state, &dir)?;
    if !dir.is_dir() {
        return Err("The folder to save in isn't there.".into());
    }
    let sources: Vec<PathBuf> = sources.iter().map(PathBuf::from).collect();
    for s in &sources {
        if !s.is_absolute() {
            return Err("Use full paths.".into());
        }
        if nomnom_organizer::pathutil::is_inside(&dest, s) {
            return Err("The archive can't be saved inside something it is made from.".into());
        }
    }
    let tmp = dir.join(format!(".nomnom-{}.tmp", uuid::Uuid::new_v4().simple()));
    CANCEL.store(false, Ordering::Relaxed);
    let (t, d) = (tmp.clone(), dest.clone());
    let result = tokio::task::spawn_blocking(move || {
        let mut pace = Pace::new();
        let (files, bytes) = build(&sources, &t, |done, total, force| {
            pace.tell(&app, done, total, force)
        })?;
        nomnom_core::fs::move_no_replace(&t, &d)
            .map_err(|e| format!("Couldn't save the archive: {e}"))?;
        Ok::<_, String>((files, bytes))
    })
    .await
    .map_err(|e| e.to_string())?;
    match result {
        Ok((files, bytes)) => Ok(Created {
            path: dest.to_string_lossy().into_owned(),
            files,
            bytes,
        }),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn ar_cancel() {
    CANCEL.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let f = std::fs::File::create(path).unwrap();
        let mut w = zip::ZipWriter::new(f);
        let o = zip::write::SimpleFileOptions::default();
        for (n, d) in entries {
            w.start_file(*n, o).unwrap();
            w.write_all(d).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn gathering_names_files_by_their_place_inside_the_chosen_folder() {
        let d = tempfile::tempdir().unwrap();
        let top = d.path().join("Photos");
        std::fs::create_dir_all(top.join("trip")).unwrap();
        std::fs::write(top.join("a.jpg"), "a").unwrap();
        std::fs::write(top.join("trip/b.jpg"), "b").unwrap();
        std::fs::create_dir_all(top.join("empty")).unwrap();
        let mut out = Vec::new();
        gather(&top, &mut out).unwrap();
        let mut names: Vec<_> = out.iter().map(|(_, n)| n.clone()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["Photos/a.jpg", "Photos/empty/", "Photos/trip/b.jpg"]
        );
    }

    #[test]
    fn a_missing_source_is_an_error() {
        let mut out = Vec::new();
        assert!(gather(Path::new("Z:\\nope\\nothing"), &mut out).is_err());
    }

    #[test]
    fn listing_reads_sizes_and_flags_a_normal_zip() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("t.zip");
        write_zip(&z, &[("a.txt", b"hello"), ("dir/b.txt", b"worlds")]);
        let l = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(ar_list(z.to_string_lossy().into_owned()))
            .unwrap();
        assert_eq!((l.count, l.unpacked, l.encrypted), (2, 11, false));
        assert_eq!(l.items[0].name, "a.txt");
    }

    #[test]
    fn a_zip_made_here_unpacks_to_the_same_files_in_a_new_folder() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("Stuff");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "alpha").unwrap();
        std::fs::write(src.join("sub/b.txt"), "beta").unwrap();
        std::fs::create_dir_all(src.join("empty")).unwrap();
        let z = d.path().join("stuff.zip");
        let (files, bytes) = build(std::slice::from_ref(&src), &z, |_, _, _| {}).unwrap();
        assert_eq!((files, bytes), (2, 9));
        let out = d.path().join("out");
        let (n, b) = unpack(z.to_str().unwrap(), &out, |_, _, _| {}).unwrap();
        assert_eq!((n, b), (2, 9));
        assert_eq!(
            std::fs::read_to_string(out.join("Stuff/a.txt")).unwrap(),
            "alpha"
        );
        assert_eq!(
            std::fs::read_to_string(out.join("Stuff/sub/b.txt")).unwrap(),
            "beta"
        );
        assert!(out.join("Stuff/empty").is_dir());
    }

    #[test]
    fn an_entry_that_escapes_stops_the_whole_unpacking() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("evil.zip");
        write_zip(&z, &[("fine.txt", b"ok"), ("../escaped.txt", b"bad")]);
        let out = d.path().join("out");
        let err = unpack(z.to_str().unwrap(), &out, |_, _, _| {}).unwrap_err();
        assert!(err.contains("outside"), "{err}");
        assert!(!d.path().join("escaped.txt").exists());
    }

    #[test]
    fn nothing_that_is_already_there_is_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let z = d.path().join("a.zip");
        write_zip(&z, &[("same.txt", b"from the archive")]);
        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("same.txt"), "mine").unwrap();
        // The folder already exists, so unpacking refuses before it writes anything.
        assert!(unpack(z.to_str().unwrap(), &out, |_, _, _| {}).is_err());
        assert_eq!(
            std::fs::read_to_string(out.join("same.txt")).unwrap(),
            "mine"
        );
    }

    #[test]
    fn only_zip_files_are_opened() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("x.rar");
        std::fs::write(&f, "not a zip").unwrap();
        assert!(open_zip(f.to_str().unwrap()).is_err());
        let g = d.path().join("y.zip");
        std::fs::write(&g, "not a zip either").unwrap();
        assert!(open_zip(g.to_str().unwrap()).is_err());
    }

    #[test]
    fn the_unpack_folder_never_reuses_a_name() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(free_folder(d.path(), "X"), d.path().join("X"));
        std::fs::create_dir(d.path().join("X")).unwrap();
        assert_eq!(free_folder(d.path(), "X"), d.path().join("X (2)"));
    }
}
