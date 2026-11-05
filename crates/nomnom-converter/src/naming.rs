//! Output names. Nothing is ever replaced: a taken name gets "(converted)" and then a number.

use crate::model::Target;
use std::io;
use std::path::{Path, PathBuf};

/// Extension to use for a file written for `target` from `src`.
pub fn out_ext(src: &Path, target: Target) -> String {
    match target.ext() {
        Some(e) => e.to_string(),
        None => src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_ascii_lowercase(),
    }
}

/// The `n`th candidate name: 0 is the plain name, 1 is "name (converted)", 2 is "name (converted 2)".
pub fn candidate(dir: &Path, stem: &str, ext: &str, n: u32) -> PathBuf {
    let name = match n {
        0 => format!("{stem}.{ext}"),
        1 => format!("{stem} (converted).{ext}"),
        n => format!("{stem} (converted {n}).{ext}"),
    };
    dir.join(name)
}

/// First candidate that doesn't exist (Windows compares names without regard to case).
pub fn first_free(dir: &Path, stem: &str, ext: &str, start: u32) -> PathBuf {
    let mut n = start;
    loop {
        let p = candidate(dir, stem, ext, n);
        if !p.exists() {
            return p;
        }
        n += 1;
        if n > 9999 {
            return candidate(
                dir,
                &format!("{stem} {}", uuid::Uuid::new_v4().simple()),
                ext,
                0,
            );
        }
    }
}

/// Temporary file next to where the result will go, so the final step is a rename on one volume.
pub fn temp_for(dir: &Path, ext: &str) -> PathBuf {
    let id = uuid::Uuid::new_v4().simple().to_string();
    dir.join(format!(".nomnom-{}.tmp.{ext}", &id[..12]))
}

/// Puts `tmp` at the first free name. Another program could take the name between the check and
/// the move, so a refusal moves on to the next name instead of replacing anything.
pub fn place(tmp: &Path, dir: &Path, stem: &str, ext: &str) -> io::Result<PathBuf> {
    let mut n = 0;
    loop {
        let dst = first_free(dir, stem, ext, n);
        match nomnom_core::fs::move_no_replace(tmp, &dst) {
            Ok(()) => return Ok(dst),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                n += 1;
                if n > 9999 {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// File name without extension, made safe to reuse as a stem.
pub fn stem_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("converted")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_name_first_then_converted_then_numbers() {
        let dir = tempfile::tempdir().unwrap();
        let p0 = first_free(dir.path(), "photo", "webp", 0);
        assert_eq!(p0.file_name().unwrap(), "photo.webp");
        std::fs::write(&p0, b"x").unwrap();
        let p1 = first_free(dir.path(), "photo", "webp", 0);
        assert_eq!(p1.file_name().unwrap(), "photo (converted).webp");
        std::fs::write(&p1, b"x").unwrap();
        let p2 = first_free(dir.path(), "photo", "webp", 0);
        assert_eq!(p2.file_name().unwrap(), "photo (converted 2).webp");
    }

    #[test]
    fn a_taken_name_is_matched_without_regard_to_case() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Photo.WEBP"), b"x").unwrap();
        let p = first_free(dir.path(), "photo", "webp", 0);
        assert_eq!(p.file_name().unwrap(), "photo (converted).webp");
    }

    #[test]
    fn place_never_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.png"), b"keep me").unwrap();
        let tmp = temp_for(dir.path(), "png");
        std::fs::write(&tmp, b"new").unwrap();
        let dst = place(&tmp, dir.path(), "a", "png").unwrap();
        assert_eq!(dst.file_name().unwrap(), "a (converted).png");
        assert_eq!(std::fs::read(dir.path().join("a.png")).unwrap(), b"keep me");
        assert_eq!(std::fs::read(dst).unwrap(), b"new");
        assert!(!tmp.exists());
    }

    #[test]
    fn no_audio_keeps_the_source_extension() {
        assert_eq!(out_ext(Path::new("a.MOV"), Target::NoAudio), "mov");
        assert_eq!(out_ext(Path::new("a.mov"), Target::Mp4), "mp4");
    }
}
