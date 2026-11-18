//! File Tools: joining PDFs and taking pages out of a PDF. Results are written next to the original
//! under a name that isn't taken; the originals are never changed.

use crate::filetools::output_problem;
use crate::state::AppState;
use nomnom_converter::pdf_tools;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

type CmdResult<T> = Result<T, String>;

#[derive(Serialize)]
pub struct PdfInfo {
    pub path: String,
    pub name: String,
    pub pages: Option<u32>,
    pub problem: Option<String>,
}

fn is_pdf(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".into())
}

fn checked(state: &AppState, paths: &[String]) -> CmdResult<Vec<PathBuf>> {
    let files: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err("Pick a PDF first.".into());
    }
    if files
        .iter()
        .any(|f| !f.is_absolute() || !is_pdf(f) || !f.is_file())
    {
        return Err("Only PDF files can be used here.".into());
    }
    if let Some(problem) = output_problem(state, &files, &None) {
        return Err(problem);
    }
    Ok(files)
}

/// Page count and any problem for each chosen PDF.
#[tauri::command]
pub async fn pdf_info(paths: Vec<String>) -> Vec<PdfInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|p| {
                let path = PathBuf::from(&p);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                match pdf_tools::page_count(&path) {
                    Ok(n) => PdfInfo {
                        path: p,
                        name,
                        pages: Some(n),
                        problem: None,
                    },
                    Err(e) => PdfInfo {
                        path: p,
                        name,
                        pages: None,
                        problem: Some(e),
                    },
                }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// Joins the PDFs, in the order given. Returns the new file.
#[tauri::command]
pub async fn pdf_merge(state: State<'_, AppState>, paths: Vec<String>) -> CmdResult<String> {
    let files = checked(&state, &paths)?;
    let dir = files[0]
        .parent()
        .map(Path::to_path_buf)
        .ok_or("That file has no folder.")?;
    let name = format!("{} + {} more", stem(&files[0]), files.len() - 1);
    tauri::async_runtime::spawn_blocking(move || pdf_tools::merge(&files, &dir, &name))
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.to_string_lossy().into_owned())
}

/// Either every page as its own file (`mode` "each"), or the chosen pages as one new PDF ("pages").
#[tauri::command]
pub async fn pdf_split(
    state: State<'_, AppState>,
    path: String,
    mode: String,
    pages: String,
) -> CmdResult<Vec<String>> {
    let files = checked(&state, std::slice::from_ref(&path))?;
    let file = files[0].clone();
    let dir = file
        .parent()
        .map(Path::to_path_buf)
        .ok_or("That file has no folder.")?;
    tauri::async_runtime::spawn_blocking(move || -> CmdResult<Vec<PathBuf>> {
        match mode.as_str() {
            "each" => pdf_tools::split_each(&file, &dir, &stem(&file)),
            "pages" => {
                let total = pdf_tools::page_count(&file)?;
                let wanted = pdf_tools::parse_pages(&pages, total)?;
                let list = pages.trim().to_string();
                Ok(vec![pdf_tools::extract(
                    &file,
                    &wanted,
                    &dir,
                    &format!("{} - pages {}", stem(&file), list),
                )?])
            }
            _ => Err("Pick every page or some pages.".into()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .map(|v| {
        v.into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    })
}
