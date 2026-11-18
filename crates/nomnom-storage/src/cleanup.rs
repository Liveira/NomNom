//! Places where programs keep files they can make again: temporary folders, browser caches, crash
//! reports. The list is fixed and short; NomNom only measures them. Clearing one goes through the
//! same preview and confirmation as any other delete in the Disk Analyzer.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Where to look. Built from the person's own folders, so tests can point it at a temp folder.
#[derive(Debug, Clone)]
pub struct Env {
    pub home: PathBuf,
    /// `%LOCALAPPDATA%` on Windows; `None` elsewhere.
    pub local_app_data: Option<PathBuf>,
    pub temp: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub id: String,
    pub title: String,
    pub why: String,
    pub path: PathBuf,
}

fn spot(id: &str, title: &str, why: &str, path: PathBuf) -> Spot {
    Spot {
        id: id.into(),
        title: title.into(),
        why: why.into(),
        path,
    }
}

/// Folders inside `dir` (one level), for things like Firefox profiles.
fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    v.sort();
    v
}

const BROWSER_WHY: &str =
    "Pages and pictures the browser saved to load faster. It fetches them again when needed.";

/// The known places that exist on this computer.
pub fn spots(env: &Env) -> Vec<Spot> {
    let mut out: Vec<Spot> = Vec::new();
    let mut add = |s: Spot| {
        if s.path.is_dir() {
            out.push(s);
        }
    };

    add(spot(
        "temp",
        "Temporary files",
        "Files programs left in your temporary folder. A file that is in use is skipped and the rest can go.",
        env.temp.clone(),
    ));
    add(spot(
        "nomnom-temp",
        "NomNom's own leftovers",
        "Installers NomNom downloaded for updates and other short-lived files.",
        env.temp.join("NomNom-update"),
    ));

    if let Some(local) = &env.local_app_data {
        add(spot(
            "crash",
            "Crash reports",
            "Records Windows kept when a program crashed.",
            local.join("CrashDumps"),
        ));
        add(spot(
            "inet",
            "Internet cache",
            "Web files Windows and some programs saved for speed.",
            local.join("Microsoft").join("Windows").join("INetCache"),
        ));
        add(spot(
            "shader",
            "Graphics shader cache",
            "Compiled graphics code that games and apps rebuild when needed.",
            local.join("D3DSCache"),
        ));
        for (id, title, rel) in [
            (
                "chrome",
                "Google Chrome cache",
                ["Google", "Chrome", "User Data", "Default", "Cache"],
            ),
            (
                "edge",
                "Microsoft Edge cache",
                ["Microsoft", "Edge", "User Data", "Default", "Cache"],
            ),
            (
                "brave",
                "Brave cache",
                [
                    "BraveSoftware",
                    "Brave-Browser",
                    "User Data",
                    "Default",
                    "Cache",
                ],
            ),
        ] {
            add(spot(
                id,
                title,
                BROWSER_WHY,
                rel.iter().fold(local.clone(), |p, s| p.join(s)),
            ));
        }
        for p in subfolders(&local.join("Mozilla").join("Firefox").join("Profiles")) {
            add(spot(
                "firefox",
                "Firefox cache",
                BROWSER_WHY,
                p.join("cache2"),
            ));
        }
        add(spot(
            "npm",
            "npm download cache",
            "Packages npm downloaded before. It downloads them again if a project needs them.",
            local.join("npm-cache"),
        ));
        add(spot(
            "pip",
            "pip download cache",
            "Python packages pip downloaded before.",
            local.join("pip").join("Cache"),
        ));
    } else {
        let cache = env.home.join(".cache");
        add(spot(
            "thumbs",
            "Picture thumbnails",
            "Small previews your file manager made. It makes them again when you open a folder.",
            cache.join("thumbnails"),
        ));
        for (id, title, rel) in [
            (
                "chrome",
                "Google Chrome cache",
                &["google-chrome", "Default", "Cache"][..],
            ),
            (
                "chromium",
                "Chromium cache",
                &["chromium", "Default", "Cache"][..],
            ),
            (
                "brave",
                "Brave cache",
                &["BraveSoftware", "Brave-Browser", "Default", "Cache"][..],
            ),
        ] {
            add(spot(
                id,
                title,
                BROWSER_WHY,
                rel.iter().fold(cache.clone(), |p, s| p.join(s)),
            ));
        }
        for p in subfolders(&cache.join("mozilla").join("firefox")) {
            add(spot(
                "firefox",
                "Firefox cache",
                BROWSER_WHY,
                p.join("cache2"),
            ));
        }
        add(spot(
            "pip",
            "pip download cache",
            "Python packages pip downloaded before.",
            cache.join("pip"),
        ));
        add(spot(
            "npm",
            "npm download cache",
            "Packages npm downloaded before. It downloads them again if a project needs them.",
            env.home.join(".npm").join("_cacache"),
        ));
        add(spot(
            "mesa",
            "Graphics shader cache",
            "Compiled graphics code that games and apps rebuild when needed.",
            cache.join("mesa_shader_cache"),
        ));
    }
    out
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Measured {
    pub size: u64,
    pub files: u64,
    /// True when the walk stopped early (too many entries, or asked to stop), so the numbers are a floor.
    pub partial: bool,
}

/// Adds up the files under `dir`. Links are counted as themselves and never followed.
pub fn measure(dir: &Path, cancel: &AtomicBool, max_entries: u64) -> Measured {
    let mut m = Measured::default();
    let mut seen = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            seen += 1;
            if seen > max_entries || cancel.load(Ordering::Relaxed) {
                m.partial = true;
                return m;
            }
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(e.path());
            } else {
                m.files += 1;
                if let Ok(md) = e.metadata() {
                    m.size += md.len();
                }
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(p: &Path, n: usize) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, vec![7u8; n]).unwrap();
    }

    #[test]
    fn measures_files_in_nested_folders() {
        let d = tempfile::tempdir().unwrap();
        put(&d.path().join("a.bin"), 100);
        put(&d.path().join("x/b.bin"), 250);
        put(&d.path().join("x/y/c.bin"), 50);
        let m = measure(d.path(), &AtomicBool::new(false), 1000);
        assert_eq!((m.size, m.files, m.partial), (400, 3, false));
    }

    #[test]
    fn stops_when_there_are_too_many_entries_or_when_asked() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..20 {
            put(&d.path().join(format!("f{i}")), 1);
        }
        let m = measure(d.path(), &AtomicBool::new(false), 5);
        assert!(m.partial && m.files <= 5);
        let stop = AtomicBool::new(true);
        assert!(measure(d.path(), &stop, 1000).partial);
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_a_link_out_of_the_folder() {
        let outside = tempfile::tempdir().unwrap();
        put(&outside.path().join("big.bin"), 10_000);
        let d = tempfile::tempdir().unwrap();
        put(&d.path().join("small.bin"), 10);
        std::os::unix::fs::symlink(outside.path(), d.path().join("link")).unwrap();
        let m = measure(d.path(), &AtomicBool::new(false), 1000);
        assert!(m.size < 10_000, "followed the link: {}", m.size);
    }

    #[test]
    fn lists_only_places_that_exist() {
        let home = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".cache/thumbnails")).unwrap();
        std::fs::create_dir_all(
            home.path()
                .join(".cache/mozilla/firefox/abc.default/cache2"),
        )
        .unwrap();
        let env = Env {
            home: home.path().into(),
            local_app_data: None,
            temp: temp.path().into(),
        };
        let ids: Vec<String> = spots(&env).into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["temp", "thumbs", "firefox"]);
    }

    #[test]
    fn windows_places_come_from_local_app_data() {
        let local = tempfile::tempdir().unwrap();
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(local.path().join("CrashDumps")).unwrap();
        std::fs::create_dir_all(local.path().join("Google/Chrome/User Data/Default/Cache"))
            .unwrap();
        let env = Env {
            home: local.path().into(),
            local_app_data: Some(local.path().into()),
            temp: temp.path().into(),
        };
        let ids: Vec<String> = spots(&env).into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["temp", "crash", "chrome"]);
    }
}
