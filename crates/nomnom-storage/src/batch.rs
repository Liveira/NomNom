//! Operations on many items at once: renaming by a rule, and putting items in a ZIP file.

use crate::index::{Index, NodeId};
use crate::ops::{
    self, free_name, Item, ItemResult, Kind, Plan, Progress, Refusal, Report, Status,
};
use crate::protect::{valid_new_name, Protected};
use crate::scan::long_os_path;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct RenameRule {
    /// Text (or a pattern, when `regex` is set) to look for in the name without its extension.
    pub find: String,
    pub replace: String,
    pub regex: bool,
    pub prefix: String,
    pub suffix: String,
    /// Written for `{n}` in the replacement, prefix or suffix.
    pub start: u64,
    pub step: u64,
    pub pad: usize,
    /// "lower", "upper" or empty.
    pub case: String,
    /// Change the extension too (otherwise it is kept as it is).
    pub include_extension: bool,
}

fn split_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    }
}

/// New names for `names`, in order. A problem with one name is reported for that name only.
pub fn rename_preview(
    names: &[String],
    rule: &RenameRule,
) -> Result<Vec<Result<String, String>>, String> {
    let re = if rule.regex && !rule.find.is_empty() {
        Some(
            regex::RegexBuilder::new(&rule.find)
                .size_limit(1 << 20)
                .build()
                .map_err(|e| format!("That pattern isn't valid: {e}"))?,
        )
    } else {
        None
    };
    let step = rule.step.max(1);
    Ok(names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let n = rule.start + (i as u64) * step;
            let num = format!("{n:0width$}", width = rule.pad);
            let token = |s: &str| s.replace("{n}", &num);
            let (stem, ext) = if rule.include_extension {
                (name.as_str(), "")
            } else {
                split_ext(name)
            };
            let mut stem = if rule.find.is_empty() {
                stem.to_string()
            } else if let Some(re) = &re {
                re.replace_all(stem, token(&rule.replace).as_str())
                    .to_string()
            } else {
                let find = Regex::new(&format!("(?i){}", regex::escape(&rule.find))).unwrap();
                find.replace_all(stem, regex::NoExpand(&token(&rule.replace)))
                    .to_string()
            };
            stem = format!("{}{}{}", token(&rule.prefix), stem, token(&rule.suffix));
            let mut full = format!("{stem}{ext}");
            match rule.case.as_str() {
                "lower" => full = full.to_lowercase(),
                "upper" => full = full.to_uppercase(),
                _ => {}
            }
            valid_new_name(&full).map(|_| full)
        })
        .collect())
}

/// Plans renaming every selected item by `rule`. Names that would clash with each other, with
/// something already in the folder, or that are invalid are left out with the reason.
pub fn plan_batch_rename(
    idx: &Index,
    nodes: &[NodeId],
    rule: &RenameRule,
    protect: &Protected,
) -> Result<Plan, String> {
    // The checks (protected places, items changed since the scan, folder inside folder) are the same
    // for every kind; only the kind and the names differ.
    let mut plan = ops::plan(idx, Kind::Delete, nodes, None, None, protect);
    plan.kind = Kind::BatchRename;
    plan.needs_typed = false;
    plan.new_name = None;
    let mut sorted: Vec<Item> = std::mem::take(&mut plan.items);
    sorted.sort_by(|a, b| {
        a.path
            .to_string_lossy()
            .to_lowercase()
            .cmp(&b.path.to_string_lossy().to_lowercase())
    });
    let names: Vec<String> = sorted
        .iter()
        .map(|i| i.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    let proposed = rename_preview(&names, rule)?;
    let renamed_ids: HashSet<NodeId> = sorted.iter().map(|i| i.node).collect();
    let mut taken: HashSet<(PathBuf, String)> = HashSet::new();
    let mut kept = Vec::new();
    for (mut item, new) in sorted.into_iter().zip(proposed) {
        let old = item.path.file_name().unwrap().to_string_lossy().to_string();
        let refuse = |plan: &mut Plan, why: &str, item: &Item| {
            plan.refused.push(Refusal {
                path: item.path.to_string_lossy().to_string(),
                reason: why.to_string(),
            })
        };
        let new = match new {
            Ok(n) => n,
            Err(why) => {
                refuse(&mut plan, &why, &item);
                continue;
            }
        };
        if new == old {
            refuse(&mut plan, "The name would not change.", &item);
            continue;
        }
        let parent = item.path.parent().unwrap().to_path_buf();
        let key = (parent.clone(), new.to_lowercase());
        if !taken.insert(key) {
            refuse(
                &mut plan,
                &format!("Another item would also be called {new}."),
                &item,
            );
            continue;
        }
        // Something already has that name in the folder (and isn't itself being renamed).
        let clash = idx
            .resolve_path(&parent.join(&new))
            .is_some_and(|other| other != item.node && !renamed_ids.contains(&other))
            || (parent.join(&new).exists()
                && !new.eq_ignore_ascii_case(&old)
                && idx.resolve_path(&parent.join(&new)).is_none());
        if clash {
            refuse(
                &mut plan,
                &format!("{new} already exists in this folder."),
                &item,
            );
            continue;
        }
        item.new_name = Some(new);
        kept.push(item);
    }
    plan.items = kept;
    plan.total_size = plan.items.iter().map(|i| i.size).sum();
    plan.total_files = plan.items.len() as u64;
    Ok(plan)
}

/// Where the archive will be written, and what it is called.
pub fn zip_target(dest: &Path, name: &str) -> PathBuf {
    let n = if name.to_lowercase().ends_with(".zip") {
        name.to_string()
    } else {
        format!("{name}.zip")
    };
    free_name(dest, &n)
}

pub fn run_zip(plan: &Plan, cancel: &AtomicBool, progress: &dyn Fn(Progress)) -> Report {
    let mut report = Report::default();
    let dest = plan.dest.clone().unwrap_or_default();
    let name = plan.new_name.clone().unwrap_or_else(|| "Archive".into());
    let target = zip_target(&dest, &name);
    let total = plan.items.len();
    let outcome = (|| -> io::Result<Vec<(usize, Result<(), String>)>> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(long_os_path(&target))?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        let mut used: HashSet<String> = HashSet::new();
        let mut each = Vec::new();
        for (i, item) in plan.items.iter().enumerate() {
            progress(Progress {
                done: i,
                total,
                current: item
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            });
            if cancel.load(Ordering::SeqCst) {
                each.push((i, Err("Stopped before this item.".to_string())));
                continue;
            }
            if let Err(why) = ops_revalidate(item) {
                each.push((i, Err(why)));
                continue;
            }
            let base = unique_entry_name(
                item.path.file_name().unwrap().to_string_lossy().as_ref(),
                &mut used,
            );
            let mut skipped_links = 0;
            let r = add_tree(
                &mut zip,
                &item.path,
                &base,
                opts,
                cancel,
                &mut skipped_links,
            );
            each.push((i, r.map_err(|e| e.to_string())));
        }
        zip.finish()?;
        Ok(each)
    })();
    match outcome {
        Ok(each) => {
            for (i, r) in each {
                let item = &plan.items[i];
                let status = if r.is_ok() {
                    Status::Done
                } else {
                    Status::Failed
                };
                report.results.push(ItemResult {
                    path: item.path.to_string_lossy().to_string(),
                    status,
                    message: r.err(),
                    size: item.size,
                    to: Some(target.to_string_lossy().to_string()),
                });
            }
            if report.results.iter().all(|r| r.status != Status::Done) {
                let _ = std::fs::remove_file(long_os_path(&target));
            }
        }
        Err(e) => {
            let _ = std::fs::remove_file(long_os_path(&target));
            for item in &plan.items {
                report.results.push(ItemResult {
                    path: item.path.to_string_lossy().to_string(),
                    status: Status::Failed,
                    message: Some(format!("The archive couldn't be made: {e}")),
                    size: item.size,
                    to: None,
                });
            }
        }
    }
    report.cancelled = cancel.load(Ordering::SeqCst);
    progress(Progress {
        done: total,
        total,
        current: String::new(),
    });
    report
}

fn ops_revalidate(item: &Item) -> Result<(), String> {
    let now = nomnom_organizer::winfs::info(&long_os_path(&item.path))
        .map_err(|_| "It is gone or can't be read now.".to_string())?;
    if now.id != item.snap.id || now.is_dir() != item.snap.is_dir {
        return Err("A different file or folder is at this path now.".into());
    }
    if !item.snap.is_dir && (now.size != item.snap.size || now.mtime != item.snap.mtime) {
        return Err("The file changed after the preview.".into());
    }
    Ok(())
}

fn unique_entry_name(name: &str, used: &mut HashSet<String>) -> String {
    let mut candidate = name.to_string();
    let mut n = 2;
    while !used.insert(candidate.to_lowercase()) {
        let (stem, ext) = split_ext(name);
        candidate = format!("{stem} ({n}){ext}");
        n += 1;
    }
    candidate
}

/// Adds a file, or a folder and everything in it. Links are not followed and not stored.
fn add_tree(
    zip: &mut zip::ZipWriter<File>,
    path: &Path,
    entry: &str,
    opts: zip::write::SimpleFileOptions,
    cancel: &AtomicBool,
    skipped_links: &mut usize,
) -> io::Result<()> {
    if cancel.load(Ordering::SeqCst) {
        return Err(io::Error::other("Stopped."));
    }
    let lp = long_os_path(path);
    let md = std::fs::symlink_metadata(&lp)?;
    if md.file_attributes() & 0x400 != 0 {
        *skipped_links += 1;
        return Ok(());
    }
    if md.is_dir() {
        zip.add_directory(format!("{entry}/"), opts)
            .map_err(io::Error::other)?;
        for e in std::fs::read_dir(&lp)? {
            let e = e?;
            add_tree(
                zip,
                &path.join(e.file_name()),
                &format!("{entry}/{}", e.file_name().to_string_lossy()),
                opts,
                cancel,
                skipped_links,
            )?;
        }
        return Ok(());
    }
    zip.start_file(entry, opts).map_err(io::Error::other)?;
    let mut f = File::open(&lp)?;
    io::copy(&mut f, zip)?;
    zip.flush()?;
    Ok(())
}
