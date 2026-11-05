//! Looking at a folder and deciding what could move. Nothing is changed here: the result is a
//! plan, with every file that won't move and the reason why.

use crate::classify::check_content;
use crate::model::{Fingerprint, Move, Options, OrganizerError, Plan, Skip, SkipCode};
use crate::pathutil::{has_traversal, is_drive_root, is_inside, is_same, norm};
use crate::safety::{self, DirVerdict, Limits};
use crate::winfs;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FILETIME_UNIX_EPOCH: i64 = 116_444_736_000_000_000;

fn refuse(msg: impl Into<String>) -> OrganizerError {
    OrganizerError::Refused(msg.into())
}

fn age_secs(mtime_ticks: i64) -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    now - (mtime_ticks - FILETIME_UNIX_EPOCH) / 10_000_000
}

/// The checks on the folders themselves. A refusal here means nothing is scanned at all.
pub fn validate_roots(o: &Options) -> Result<(), OrganizerError> {
    if has_traversal(&o.source) || has_traversal(&o.dest_root) {
        return Err(refuse("Folder paths must be complete paths without '..'."));
    }
    if is_drive_root(&o.source) {
        return Err(refuse(
            "A whole drive can't be organized. Pick a folder inside it.",
        ));
    }
    let info = winfs::info(&o.source).map_err(|_| refuse("That folder can't be opened."))?;
    if !info.is_dir() {
        return Err(refuse("That isn't a folder."));
    }
    if info.is_reparse() {
        return Err(refuse(
            "That folder is a link to somewhere else, so it was left alone.",
        ));
    }
    if o.protected_exact.iter().any(|p| is_same(&o.source, p)) {
        return Err(refuse(
            "That folder holds your whole account. Pick a folder inside it instead.",
        ));
    }
    if let Some(zone) = o.protected_zones.iter().find(|z| is_inside(&o.source, z)) {
        return Err(refuse(format!(
            "{} is protected. NomNom never reorganizes system or program folders.",
            zone.display()
        )));
    }
    if is_inside(&o.source, &o.dest_root) {
        return Err(refuse(
            "That folder is already inside the destination, so it is organized already.",
        ));
    }
    if let Some(zone) = o
        .protected_zones
        .iter()
        .find(|z| is_inside(&o.dest_root, z))
    {
        return Err(refuse(format!(
            "Files can't be moved into {}.",
            zone.display()
        )));
    }
    if is_drive_root(&o.dest_root) {
        return Err(refuse(
            "Pick a folder for the destination, not a whole drive.",
        ));
    }
    if let Ok(d) = winfs::info(&o.dest_root) {
        if d.is_reparse() {
            return Err(refuse(
                "The destination is a link to somewhere else, so it was left alone.",
            ));
        }
        if !d.is_dir() {
            return Err(refuse("The destination isn't a folder."));
        }
    }
    Ok(())
}

fn known_exts(o: &Options) -> HashSet<String> {
    o.categories
        .iter()
        .flat_map(|c| c.exts.iter().cloned())
        .collect()
}

fn category_for<'a>(o: &'a Options, ext: &str) -> Option<&'a str> {
    o.categories
        .iter()
        .find(|c| c.exts.iter().any(|e| e == ext))
        .map(|c| c.name.as_str())
}

struct Candidate {
    path: PathBuf,
    name: String,
    category: String,
    fp: Fingerprint,
    volume: u32,
}

/// Judges one file. Returns the candidate or the reason it stays.
fn judge_file(o: &Options, path: &Path, info: &winfs::FileInfo) -> Result<Candidate, Skip> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let skip = |code, msg: &str| Skip::new(path, false, code, msg);

    if safety::is_in_progress_name(&name) {
        return Err(skip(
            SkipCode::InProgress,
            "It looks like it's still downloading.",
        ));
    }
    let Some(ext) = safety::ext_of(&name) else {
        return Err(skip(
            SkipCode::Unknown,
            "It has no file extension, so NomNom can't tell what it is.",
        ));
    };
    if o.excluded_exts.iter().any(|e| e.eq_ignore_ascii_case(&ext)) {
        return Err(skip(
            SkipCode::Excluded,
            "You chose to leave this type of file alone.",
        ));
    }
    if safety::is_program_ext(&ext) {
        return Err(skip(
            SkipCode::Program,
            "Programs, scripts and settings files are never moved.",
        ));
    }
    if safety::is_archive_ext(&ext) {
        return Err(skip(
            SkipCode::Archive,
            "Archives are left alone because they can hold software.",
        ));
    }
    let Some(category) = category_for(o, &ext) else {
        return Err(skip(
            SkipCode::Unknown,
            &format!("NomNom doesn't sort .{ext} files."),
        ));
    };
    if !o.only_categories.is_empty() && !o.only_categories.iter().any(|c| c == category) {
        return Err(skip(
            SkipCode::Excluded,
            "It isn't in a category you picked.",
        ));
    }
    if age_secs(info.mtime) < o.settle_secs as i64 {
        return Err(skip(
            SkipCode::RecentlyChanged,
            "It changed a moment ago, so it may still be in use.",
        ));
    }
    if winfs::is_locked(path) {
        return Err(skip(SkipCode::Locked, "Another program is using it."));
    }
    let head = match winfs::read_head(path, 8192) {
        Ok(h) => h,
        Err(_) => return Err(skip(SkipCode::Unreadable, "It couldn't be read.")),
    };
    if let Err(why) = check_content(&ext, &head) {
        let code = if head.is_empty() {
            SkipCode::Empty
        } else if why.contains("program") {
            SkipCode::Program
        } else {
            SkipCode::Mismatch
        };
        return Err(skip(code, &why));
    }
    Ok(Candidate {
        path: path.to_path_buf(),
        name,
        category: category.to_string(),
        fp: Fingerprint {
            size: info.size,
            mtime: info.mtime,
            volume: info.id.volume,
            index: info.id.index,
        },
        volume: info.id.volume,
    })
}

/// The volume the destination folder lives on, found by walking up to the first folder that exists.
fn dest_volume(dest_root: &Path) -> Option<u32> {
    dest_root
        .ancestors()
        .find_map(|a| winfs::info(a).ok().map(|i| i.id.volume))
}

/// "name (2).ext" style alternative that isn't taken.
fn numbered(dir: &Path, name: &str, taken: &HashSet<String>) -> PathBuf {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for n in 2..10_000 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !taken.contains(&norm(&candidate)) && winfs::info(&candidate).is_err() {
            return candidate;
        }
    }
    dir.join(format!("{stem} (many){ext}"))
}

/// Builds the plan. Reads the folder; changes nothing.
pub fn plan(o: &Options) -> Result<Plan, OrganizerError> {
    validate_roots(o)?;
    let known = known_exts(o);
    let limits = Limits::default();
    let mut skips: Vec<Skip> = Vec::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut seen = 0usize;

    let mut stack = vec![(o.source.clone(), 0u32)];
    'walk: while let Some((dir, depth)) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => {
                skips.push(Skip::new(
                    &dir,
                    true,
                    SkipCode::Unreadable,
                    "This folder couldn't be read.",
                ));
                continue;
            }
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > o.max_entries {
                skips.push(Skip::new(
                    &o.source,
                    true,
                    SkipCode::TooMany,
                    "There were too many items, so the rest were not looked at.",
                ));
                break 'walk;
            }
            let path = entry.path();
            if is_inside(&path, &o.dest_root) {
                skips.push(Skip::new(
                    &path,
                    false,
                    SkipCode::AlreadyOrganized,
                    "It's already in the destination folder.",
                ));
                continue;
            }
            let Ok(info) = winfs::info(&path) else {
                skips.push(Skip::new(
                    &path,
                    false,
                    SkipCode::Unreadable,
                    "It couldn't be read.",
                ));
                continue;
            };
            let is_dir = info.is_dir();
            if info.is_reparse() {
                skips.push(Skip::new(
                    &path,
                    is_dir,
                    SkipCode::Link,
                    "It's a link to somewhere else, and links are never followed.",
                ));
                continue;
            }
            if info.is_cloud_or_offline() {
                skips.push(Skip::new(
                    &path,
                    is_dir,
                    SkipCode::Cloud,
                    "It's stored in the cloud, not fully on this computer.",
                ));
                continue;
            }
            if info.is_hidden_or_system() {
                skips.push(Skip::new(
                    &path,
                    is_dir,
                    SkipCode::Hidden,
                    "It's a hidden or system item.",
                ));
                continue;
            }
            if is_dir {
                if depth >= o.max_depth {
                    skips.push(Skip::new(
                        &path,
                        true,
                        SkipCode::FolderNotOpened,
                        "Folders are left alone unless you choose to look inside folders.",
                    ));
                    continue;
                }
                match safety::judge_dir(&path, &known, &limits) {
                    DirVerdict::Safe => stack.push((path, depth + 1)),
                    DirVerdict::Unsafe(why) => {
                        skips.push(Skip::new(
                            &path,
                            true,
                            SkipCode::SoftwareFolder,
                            format!("This folder was left untouched. {why}"),
                        ));
                    }
                }
                continue;
            }
            match judge_file(o, &path, &info) {
                Ok(c) => candidates.push(c),
                Err(skip) => skips.push(skip),
            }
        }
    }

    Ok(finish(o, candidates, skips))
}

/// Plans just these files, which must sit directly in the source folder. Used by the automatic
/// mode, so one new download doesn't mean reading the whole folder again.
pub fn plan_files(o: &Options, files: &[PathBuf]) -> Result<Plan, OrganizerError> {
    validate_roots(o)?;
    let mut skips = Vec::new();
    let mut candidates = Vec::new();
    for path in files {
        if !path.parent().is_some_and(|p| is_same(p, &o.source)) {
            skips.push(Skip::new(
                path,
                false,
                SkipCode::Protected,
                "It isn't directly inside the folder being watched.",
            ));
            continue;
        }
        let Ok(info) = winfs::info(path) else {
            skips.push(Skip::new(
                path,
                false,
                SkipCode::Unreadable,
                "It's no longer there.",
            ));
            continue;
        };
        if info.is_dir()
            || info.is_reparse()
            || info.is_cloud_or_offline()
            || info.is_hidden_or_system()
        {
            skips.push(Skip::new(
                path,
                info.is_dir(),
                SkipCode::Hidden,
                "It isn't an ordinary file.",
            ));
            continue;
        }
        match judge_file(o, path, &info) {
            Ok(c) => candidates.push(c),
            Err(skip) => skips.push(skip),
        }
    }
    Ok(finish(o, candidates, skips))
}

fn finish(o: &Options, candidates: Vec<Candidate>, mut skips: Vec<Skip>) -> Plan {
    // Turn candidates into moves, never colliding with anything.
    let dest_vol = dest_volume(&o.dest_root);
    let mut taken: HashSet<String> = HashSet::new();
    let mut moves = Vec::new();
    let mut candidates = candidates;
    candidates.sort_by(|a, b| {
        (a.category.as_str(), a.name.to_lowercase())
            .cmp(&(b.category.as_str(), b.name.to_lowercase()))
    });
    for c in candidates {
        let dir = o.dest_root.join(&c.category);
        let wanted = dir.join(&c.name);
        let occupied = |p: &Path| taken.contains(&norm(p)) || winfs::info(p).is_ok();
        let dst = if !occupied(&wanted) {
            wanted
        } else {
            match o.conflict {
                crate::model::ConflictPolicy::Skip => {
                    skips.push(Skip::new(
                        &c.path,
                        false,
                        SkipCode::Conflict,
                        "A file with this name is already in the destination.",
                    ));
                    continue;
                }
                crate::model::ConflictPolicy::KeepBoth => numbered(&dir, &c.name, &taken),
            }
        };
        taken.insert(norm(&dst));
        moves.push(Move {
            cross_volume: dest_vol.is_some_and(|v| v != c.volume),
            src: c.path,
            dst,
            category: c.category,
            fp: c.fp,
        });
    }
    skips.sort_by(|a, b| a.path.cmp(&b.path));
    Plan {
        source: o.source.clone(),
        dest_root: o.dest_root.clone(),
        moves,
        skips,
    }
}
