//! Per-user right-click context menu integration for Linux file managers:
//! - **KDE Plasma / Dolphin** (`$XDG_DATA_HOME/kio/servicemenus/*.desktop` — executable `.desktop`
//!   service menus for Plasma 5 & Plasma 6, widely used on Arch Linux).
//! - **GNOME Files / Nautilus** (`$XDG_DATA_HOME/nautilus/scripts/NomNom/*` — executable scripts).
//! - **Cinnamon Nemo** (`$XDG_DATA_HOME/nemo/actions/*.nemo_action`).

use std::path::{Path, PathBuf};

const MANAGED_MARKER: &str = "# NomNomManaged";

pub fn default_data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|h| h.join(".local").join("share"))
        })
}

fn quote_desktop(exe: &Path) -> String {
    let s = exe
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{s}\"")
}

fn quote_sh(exe: &Path) -> String {
    let s = exe.to_string_lossy().replace('\'', "'\\''");
    format!("'{s}'")
}

fn write_executable(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, content).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn remove_managed(path: &Path) -> Result<(), String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(());
    };
    if !text.contains(MANAGED_MARKER) {
        return Ok(());
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn dolphin_analyze_menu(exe: &Path) -> String {
    let q = quote_desktop(exe);
    format!(
        "{MANAGED_MARKER}\n\
[Desktop Entry]\n\
Type=Service\n\
X-KDE-ServiceTypes=KonqPopupMenu/Plugin\n\
MimeType=inode/directory;\n\
Actions=NomNomAnalyze;\n\
\n\
[Desktop Action NomNomAnalyze]\n\
Name=Analyze disk space with NomNom\n\
Icon=nomnom\n\
Exec={q} --shell-action analyze -- %F\n"
    )
}

fn dolphin_convert_menu(exe: &Path) -> String {
    let q = quote_desktop(exe);
    format!(
        "{MANAGED_MARKER}\n\
[Desktop Entry]\n\
Type=Service\n\
X-KDE-ServiceTypes=KonqPopupMenu/Plugin\n\
X-KDE-Submenu=NomNom\n\
MimeType=image/png;image/jpeg;image/webp;image/gif;image/bmp;image/tiff;image/x-icon;video/mp4;video/x-matroska;video/quicktime;video/webm;video/x-msvideo;video/x-flv;video/mpeg;video/3gpp;audio/mpeg;audio/mp4;audio/aac;audio/flac;audio/x-wav;audio/wav;audio/ogg;audio/opus;text/plain;text/markdown;application/msword;application/vnd.openxmlformats-officedocument.wordprocessingml.document;application/vnd.ms-excel;application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;application/vnd.ms-powerpoint;application/vnd.openxmlformats-officedocument.presentationml.presentation;application/vnd.oasis.opendocument.text;application/vnd.oasis.opendocument.spreadsheet;application/vnd.oasis.opendocument.presentation;application/rtf;\n\
Actions=ToWebp;ToPng;ToJpg;ToMp4;ToMp3;ToPdf;MoreOptions;\n\
\n\
[Desktop Action ToWebp]\n\
Name=Convert to WebP\n\
Icon=nomnom\n\
Exec={q} --shell-action convert:webp -- %F\n\
\n\
[Desktop Action ToPng]\n\
Name=Convert to PNG\n\
Icon=nomnom\n\
Exec={q} --shell-action convert:png -- %F\n\
\n\
[Desktop Action ToJpg]\n\
Name=Convert to JPG\n\
Icon=nomnom\n\
Exec={q} --shell-action convert:jpg -- %F\n\
\n\
[Desktop Action ToMp4]\n\
Name=Convert to MP4\n\
Icon=nomnom\n\
Exec={q} --shell-action convert:mp4 -- %F\n\
\n\
[Desktop Action ToMp3]\n\
Name=Convert or extract MP3\n\
Icon=nomnom\n\
Exec={q} --shell-action convert:mp3 -- %F\n\
\n\
[Desktop Action ToPdf]\n\
Name=Convert to PDF\n\
Icon=nomnom\n\
Exec={q} --shell-action convert:pdf -- %F\n\
\n\
[Desktop Action MoreOptions]\n\
Name=More options in NomNom…\n\
Icon=nomnom\n\
Exec={q} --shell-action choose -- %F\n"
    )
}

fn nautilus_script(exe: &Path, action: &str) -> String {
    let q = quote_sh(exe);
    format!(
        "#!/bin/sh\n\
{MANAGED_MARKER}\n\
if [ -n \"$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS\" ]; then\n\
  IFS='\n'\n\
  set -f\n\
  exec {q} --shell-action {action} -- $NAUTILUS_SCRIPT_SELECTED_FILE_PATHS\n\
else\n\
  exec {q} --shell-action {action} -- \"$@\"\n\
fi\n"
    )
}

fn nemo_analyze_action(exe: &Path) -> String {
    let q = quote_desktop(exe);
    format!(
        "{MANAGED_MARKER}\n\
[Nemo Action]\n\
Active=true\n\
Name=Analyze disk space with NomNom\n\
Comment=Inspect folder usage in NomNom Disk Analyzer\n\
Exec={q} --shell-action analyze -- %F\n\
Icon-Name=nomnom\n\
Selection=notnone\n\
Extensions=dir;\n\
Quote=double\n"
    )
}

fn nemo_convert_action(exe: &Path) -> String {
    let q = quote_desktop(exe);
    format!(
        "{MANAGED_MARKER}\n\
[Nemo Action]\n\
Active=true\n\
Name=Convert with NomNom…\n\
Comment=Open selected files in NomNom File Tools\n\
Exec={q} --shell-action choose -- %F\n\
Icon-Name=nomnom\n\
Selection=notnone\n\
Extensions=png;jpg;jpeg;webp;gif;bmp;tif;tiff;ico;mp4;m4v;mkv;mov;webm;avi;wmv;flv;mpg;mpeg;3gp;mp3;m4a;aac;wav;flac;ogg;oga;opus;wma;aiff;aif;md;markdown;txt;doc;docx;xls;xlsx;ppt;pptx;odt;ods;odp;rtf;\n\
Quote=double\n"
    )
}

fn managed_paths(data_home: &Path) -> [PathBuf; 6] {
    [
        data_home
            .join("kio/servicemenus")
            .join("nomnom-analyze.desktop"),
        data_home
            .join("kio/servicemenus")
            .join("nomnom-convert.desktop"),
        data_home
            .join("nautilus/scripts/NomNom")
            .join("Analyze disk space"),
        data_home
            .join("nautilus/scripts/NomNom")
            .join("Convert files"),
        data_home
            .join("nemo/actions")
            .join("nomnom-analyze.nemo_action"),
        data_home
            .join("nemo/actions")
            .join("nomnom-convert.nemo_action"),
    ]
}

pub fn install(exe: &Path, data_home: &Path) -> Result<(), String> {
    let [kde_analyze, kde_convert, nau_analyze, nau_convert, nemo_analyze, nemo_convert] =
        managed_paths(data_home);
    write_executable(&kde_analyze, &dolphin_analyze_menu(exe))?;
    write_executable(&kde_convert, &dolphin_convert_menu(exe))?;
    write_executable(&nau_analyze, &nautilus_script(exe, "analyze"))?;
    write_executable(&nau_convert, &nautilus_script(exe, "choose"))?;
    write_executable(&nemo_analyze, &nemo_analyze_action(exe))?;
    write_executable(&nemo_convert, &nemo_convert_action(exe))?;
    Ok(())
}

pub fn uninstall(data_home: &Path) -> Result<(), String> {
    for path in managed_paths(data_home) {
        remove_managed(&path)?;
    }
    let nau_dir = data_home.join("nautilus/scripts/NomNom");
    if nau_dir.is_dir() {
        let _ = std::fs::remove_dir(&nau_dir); // succeeds only when empty
    }
    Ok(())
}

pub fn installed(exe: &Path, data_home: &Path) -> bool {
    let [kde_analyze, kde_convert, ..] = managed_paths(data_home);
    let q = quote_desktop(exe);
    std::fs::read_to_string(kde_analyze)
        .ok()
        .is_some_and(|t| t.contains(&q))
        && std::fs::read_to_string(kde_convert)
            .ok()
            .is_some_and(|t| t.contains(&q))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_dolphin_nautilus_and_nemo_entries_and_removes_only_ours() {
        let dir = tempfile::tempdir().unwrap();
        let exe = Path::new("/usr/bin/nomnom");
        let foreign = dir.path().join("kio/servicemenus/other.desktop");
        std::fs::create_dir_all(foreign.parent().unwrap()).unwrap();
        std::fs::write(&foreign, "[Desktop Entry]\nName=Other\n").unwrap();

        assert!(!installed(exe, dir.path()));
        install(exe, dir.path()).unwrap();
        assert!(installed(exe, dir.path()));
        assert!(!installed(Path::new("/opt/nomnom"), dir.path()));

        for p in managed_paths(dir.path()) {
            assert!(p.is_file(), "missing {}", p.display());
        }

        uninstall(dir.path()).unwrap();
        assert!(!installed(exe, dir.path()));
        assert!(
            foreign.exists(),
            "foreign service menu must remain untouched"
        );
    }
}
