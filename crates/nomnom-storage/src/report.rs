//! Reports: a table built from a scan, written as CSV, JSON, HTML, XLSX or PDF.
//! Numbers stay numbers in the data formats (bytes), and become readable sizes on screen formats.

use crate::analyze::{self, FolderMode, Group};
use crate::dupes::DupGroup;
use crate::index::{filetime_to_unix, Index, NodeId};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::io::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    Int(i64),
    /// A size in bytes.
    Bytes(u64),
    /// Unix seconds.
    Time(Option<i64>),
    Percent(f64),
}

#[derive(Clone, Debug)]
pub struct Column {
    pub key: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub title: String,
    pub note: Option<String>,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Csv,
    Json,
    Html,
    Xlsx,
    Pdf,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Json => "json",
            Format::Html => "html",
            Format::Xlsx => "xlsx",
            Format::Pdf => "pdf",
        }
    }
}

pub fn human_bytes(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let (mut v, mut i) = (n as f64, 0);
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 || v >= 100.0 {
        format!("{v:.0} {}", units[i])
    } else {
        format!("{v:.1} {}", units[i])
    }
}

fn date_text(t: Option<i64>) -> String {
    let Some(t) = t else { return String::new() };
    // Civil date from days since 1970 (proleptic Gregorian).
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60
    )
}

impl Value {
    /// How it reads on screen and in text formats.
    pub fn display(&self) -> String {
        match self {
            Value::Text(s) => s.clone(),
            Value::Int(n) => n.to_string(),
            Value::Bytes(b) => human_bytes(*b),
            Value::Time(t) => date_text(*t),
            Value::Percent(p) => format!("{p:.1}%"),
        }
    }
}

/// Keeps only the wanted columns, in the order asked. Unknown keys are ignored.
pub fn select_columns(t: &Table, keys: &[String]) -> Table {
    if keys.is_empty() {
        return t.clone();
    }
    let picked: Vec<usize> = keys
        .iter()
        .filter_map(|k| t.columns.iter().position(|c| c.key == k))
        .collect();
    Table {
        title: t.title.clone(),
        note: t.note.clone(),
        columns: picked.iter().map(|i| t.columns[*i].clone()).collect(),
        rows: t
            .rows
            .iter()
            .map(|r| picked.iter().map(|i| r[*i].clone()).collect())
            .collect(),
    }
}

// ---- tables from a scan -------------------------------------------------------------------------

const FILE_COLS: [Column; 9] = [
    Column {
        key: "name",
        label: "Name",
    },
    Column {
        key: "path",
        label: "Path",
    },
    Column {
        key: "type",
        label: "Type",
    },
    Column {
        key: "size",
        label: "Size",
    },
    Column {
        key: "alloc",
        label: "On disk",
    },
    Column {
        key: "files",
        label: "Files",
    },
    Column {
        key: "dirs",
        label: "Folders",
    },
    Column {
        key: "modified",
        label: "Modified",
    },
    Column {
        key: "percent",
        label: "% of folder",
    },
];

fn node_row(idx: &Index, id: NodeId, size_override: Option<u64>) -> Vec<Value> {
    let n = idx.node(id).unwrap();
    let parent_size = if n.parent == crate::index::NONE {
        n.size
    } else {
        idx.node(n.parent).unwrap().size
    };
    let size = size_override.unwrap_or(n.size);
    vec![
        Value::Text(idx.name(id).to_string()),
        Value::Text(idx.path_of(id).to_string_lossy().to_string()),
        Value::Text(
            if n.is_link() {
                "Link"
            } else if n.is_dir() {
                "Folder"
            } else {
                "File"
            }
            .into(),
        ),
        Value::Bytes(size),
        Value::Bytes(n.alloc),
        Value::Int(i64::from(n.files)),
        Value::Int(i64::from(n.dirs)),
        Value::Time(filetime_to_unix(n.mtime)),
        Value::Percent(if parent_size == 0 {
            0.0
        } else {
            size as f64 / parent_size as f64 * 100.0
        }),
    ]
}

/// Every folder and file below `scope` down to `depth` levels, biggest first within each folder.
pub fn tree_table(idx: &Index, scope: NodeId, depth: u32) -> Table {
    let mut rows = Vec::new();
    fn walk(idx: &Index, id: NodeId, depth: u32, rows: &mut Vec<Vec<Value>>) {
        let mut kids: Vec<NodeId> = idx.children(id).collect();
        kids.sort_by(|a, b| {
            idx.node(*b)
                .unwrap()
                .size
                .cmp(&idx.node(*a).unwrap().size)
                .then(idx.name(*a).cmp(idx.name(*b)))
        });
        for k in kids {
            rows.push(node_row(idx, k, None));
            if depth > 1 && idx.node(k).unwrap().is_dir() && !idx.node(k).unwrap().is_link() {
                walk(idx, k, depth - 1, rows);
            }
        }
    }
    rows.push(node_row(idx, scope, None));
    walk(idx, scope, depth.max(1), &mut rows);
    Table {
        title: format!("Folder report: {}", idx.path_of(scope).display()),
        note: Some(format!("Down to {depth} levels. Sizes are what the scan found at the time; they may have changed since.")),
        columns: FILE_COLS.to_vec(),
        rows,
    }
}

pub fn top_files_table(idx: &Index, scope: NodeId, n: usize) -> Table {
    let rows = analyze::top_files(idx, scope, n, &analyze::Filter::default())
        .into_iter()
        .map(|v| node_row(idx, v.id, None))
        .collect();
    Table {
        title: "Biggest files".into(),
        note: None,
        columns: FILE_COLS.to_vec(),
        rows,
    }
}

pub fn top_folders_table(idx: &Index, scope: NodeId, n: usize, mode: FolderMode) -> Table {
    let rows = analyze::top_folders(idx, scope, n, mode, 0)
        .into_iter()
        .map(|v| node_row(idx, v.id, Some(v.size)))
        .collect();
    Table {
        title: "Biggest folders".into(),
        note: Some(match mode {
            FolderMode::Inclusive => "A folder's size includes everything inside it, so a folder and its subfolders overlap. Do not add these together.".into(),
            FolderMode::Exclusive => "Sizes count only the files directly inside each folder, so they add up without counting anything twice.".into(),
        }),
        columns: FILE_COLS.to_vec(),
        rows,
    }
}

pub fn groups_table(title: &str, key_label: &'static str, groups: &[Group]) -> Table {
    let total: u64 = groups.iter().map(|g| g.size).sum();
    Table {
        title: title.into(),
        note: None,
        columns: vec![
            Column {
                key: "key",
                label: key_label,
            },
            Column {
                key: "files",
                label: "Files",
            },
            Column {
                key: "size",
                label: "Size",
            },
            Column {
                key: "alloc",
                label: "On disk",
            },
            Column {
                key: "percent",
                label: "% of total",
            },
        ],
        rows: groups
            .iter()
            .map(|g| {
                vec![
                    Value::Text(if g.key.is_empty() {
                        "(no extension)".into()
                    } else {
                        g.key.clone()
                    }),
                    Value::Int(g.files as i64),
                    Value::Bytes(g.size),
                    Value::Bytes(g.alloc),
                    Value::Percent(if total == 0 {
                        0.0
                    } else {
                        g.size as f64 / total as f64 * 100.0
                    }),
                ]
            })
            .collect(),
    }
}

pub fn duplicates_table(groups: &[DupGroup]) -> Table {
    let mut rows = Vec::new();
    for (i, g) in groups.iter().enumerate() {
        for c in &g.copies {
            rows.push(vec![
                Value::Int(i as i64 + 1),
                Value::Text(c.path.clone()),
                Value::Bytes(g.size),
                Value::Time(c.mtime),
                Value::Int(i64::from(c.names)),
                Value::Bytes(g.recoverable),
                Value::Text(g.sha256.clone()),
            ]);
        }
    }
    Table {
        title: "Duplicate files".into(),
        note: Some("Files in the same group have identical content. Recoverable space is what deleting all but one copy would free.".into()),
        columns: vec![
            Column { key: "group", label: "Group" },
            Column { key: "path", label: "Path" },
            Column { key: "size", label: "Size" },
            Column { key: "modified", label: "Modified" },
            Column { key: "names", label: "Names" },
            Column { key: "recoverable", label: "Recoverable" },
            Column { key: "sha256", label: "SHA-256" },
        ],
        rows,
    }
}

// ---- writers ---------------------------------------------------------------------------------------

fn csv_cell(s: &str) -> String {
    // A cell starting with = + - @ is read as a formula by spreadsheets; keep it text.
    let safe = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    if safe.contains([',', '"', '\n', '\r', ';']) {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}

fn raw(v: &Value) -> String {
    match v {
        Value::Bytes(b) => b.to_string(),
        Value::Time(t) => date_text(*t),
        Value::Percent(p) => format!("{p:.2}"),
        other => other.display(),
    }
}

pub fn to_csv(t: &Table) -> Vec<u8> {
    // The byte order mark makes Excel read accents correctly.
    let mut out = String::from("\u{feff}");
    out.push_str(
        &t.columns
            .iter()
            .map(|c| csv_cell(c.label))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.push_str("\r\n");
    for r in &t.rows {
        out.push_str(
            &r.iter()
                .map(|v| csv_cell(&raw(v)))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out.into_bytes()
}

pub fn to_json(t: &Table) -> Vec<u8> {
    let rows: Vec<serde_json::Value> = t
        .rows
        .iter()
        .map(|r| {
            let mut m = serde_json::Map::new();
            for (c, v) in t.columns.iter().zip(r) {
                m.insert(
                    c.key.to_string(),
                    match v {
                        Value::Text(s) => s.clone().into(),
                        Value::Int(n) => (*n).into(),
                        Value::Bytes(b) => (*b).into(),
                        Value::Time(t) => t
                            .map(serde_json::Value::from)
                            .unwrap_or(serde_json::Value::Null),
                        Value::Percent(p) => serde_json::json!((p * 100.0).round() / 100.0),
                    },
                );
            }
            serde_json::Value::Object(m)
        })
        .collect();
    serde_json::to_vec_pretty(
        &serde_json::json!({ "title": t.title, "note": t.note, "rows": rows }),
    )
    .unwrap()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn to_html(t: &Table) -> Vec<u8> {
    let mut h = String::new();
    let _ = write!(
        h,
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title>\
         <style>body{{font:14px system-ui,sans-serif;margin:2rem;color:#222}}table{{border-collapse:collapse;width:100%}}\
         th,td{{border-bottom:1px solid #ddd;padding:4px 8px;text-align:left}}th{{background:#f4efe9}}\
         td.n{{text-align:right;font-variant-numeric:tabular-nums}}p.note{{color:#555}}</style></head><body><h1>{}</h1>",
        esc(&t.title),
        esc(&t.title)
    );
    if let Some(n) = &t.note {
        let _ = write!(h, "<p class=\"note\">{}</p>", esc(n));
    }
    h.push_str("<table><thead><tr>");
    for c in &t.columns {
        let _ = write!(h, "<th>{}</th>", esc(c.label));
    }
    h.push_str("</tr></thead><tbody>");
    for r in &t.rows {
        h.push_str("<tr>");
        for v in r {
            let cls = if matches!(v, Value::Bytes(_) | Value::Int(_) | Value::Percent(_)) {
                " class=\"n\""
            } else {
                ""
            };
            let _ = write!(h, "<td{cls}>{}</td>", esc(&v.display()));
        }
        h.push_str("</tr>");
    }
    let _ = write!(
        h,
        "</tbody></table><p class=\"note\">{} rows.</p></body></html>",
        t.rows.len()
    );
    h.into_bytes()
}

fn col_letter(mut i: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (i % 26) as u8) as char);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s
}

/// A minimal Excel workbook: one sheet, text and numbers, a bold header row.
pub fn to_xlsx(t: &Table) -> std::io::Result<Vec<u8>> {
    let mut sheet = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    );
    let cell = |col: usize, row: usize, v: &Value, header: bool| -> String {
        let r = format!("{}{}", col_letter(col), row);
        let style = if header { " s=\"1\"" } else { "" };
        match v {
            Value::Int(n) => format!("<c r=\"{r}\"{style}><v>{n}</v></c>"),
            Value::Bytes(b) => format!("<c r=\"{r}\"{style}><v>{b}</v></c>"),
            Value::Percent(p) => format!("<c r=\"{r}\"{style}><v>{p:.4}</v></c>"),
            other => format!(
                "<c r=\"{r}\"{style} t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                esc(&other.display())
            ),
        }
    };
    sheet.push_str("<row r=\"1\">");
    for (i, c) in t.columns.iter().enumerate() {
        sheet.push_str(&cell(i, 1, &Value::Text(c.label.to_string()), true));
    }
    sheet.push_str("</row>");
    for (ri, r) in t.rows.iter().enumerate() {
        let _ = write!(sheet, "<row r=\"{}\">", ri + 2);
        for (ci, v) in r.iter().enumerate() {
            sheet.push_str(&cell(ci, ri + 2, v, false));
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    let parts: [(&str, String); 5] = [
        ("[Content_Types].xml", "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>".into()),
        ("_rels/.rels", "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>".into()),
        ("xl/workbook.xml", format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"{}\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>", esc(&sheet_name(&t.title)))),
        ("xl/_rels/workbook.xml.rels", "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>".into()),
        ("xl/styles.xml", "<?xml version=\"1.0\" encoding=\"UTF-8\"?><styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs></styleSheet>".into()),
    ];
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in parts {
            z.start_file(name, o).map_err(std::io::Error::other)?;
            z.write_all(body.as_bytes())?;
        }
        z.start_file("xl/worksheets/sheet1.xml", o)
            .map_err(std::io::Error::other)?;
        z.write_all(sheet.as_bytes())?;
        z.finish().map_err(std::io::Error::other)?;
    }
    Ok(buf.into_inner())
}

fn sheet_name(title: &str) -> String {
    let n: String = title
        .chars()
        .filter(|c| !matches!(c, '\\' | '/' | '?' | '*' | '[' | ']' | ':'))
        .take(31)
        .collect();
    if n.trim().is_empty() {
        "Report".into()
    } else {
        n
    }
}

/// A plain table in a monospaced font, split over pages. Long cells are cut with "…" for width.
pub fn to_pdf(t: &Table) -> (Vec<u8>, Vec<String>) {
    let cells: Vec<Vec<String>> = t
        .rows
        .iter()
        .map(|r| r.iter().map(Value::display).collect())
        .collect();
    let cap = 48usize;
    let widths: Vec<usize> = (0..t.columns.len())
        .map(|i| {
            let max = cells
                .iter()
                .map(|r| r[i].chars().count())
                .max()
                .unwrap_or(0)
                .max(t.columns[i].label.chars().count());
            max.min(cap)
        })
        .collect();
    let fit = |s: &str, w: usize| -> String {
        let n = s.chars().count();
        if n <= w {
            format!("{s:<w$}")
        } else {
            format!("{}~", s.chars().take(w - 1).collect::<String>())
        }
    };
    let mut text = String::new();
    text.push_str(&t.title);
    text.push('\n');
    if let Some(n) = &t.note {
        text.push_str(n);
        text.push('\n');
    }
    text.push('\n');
    text.push_str(
        &t.columns
            .iter()
            .zip(&widths)
            .map(|(c, w)| fit(c.label, *w))
            .collect::<Vec<_>>()
            .join("  "),
    );
    text.push('\n');
    for r in &cells {
        text.push_str(
            &r.iter()
                .zip(&widths)
                .map(|(c, w)| fit(c, *w))
                .collect::<Vec<_>>()
                .join("  "),
        );
        text.push('\n');
    }
    let (pdf, mut notes) = nomnom_converter::pdf::text_to_pdf(&text);
    notes.push("Long cells are cut with ~ so each row fits the page width. Use CSV or Excel for the full text.".into());
    (pdf, notes)
}

pub fn render(t: &Table, f: Format) -> std::io::Result<Vec<u8>> {
    Ok(match f {
        Format::Csv => to_csv(t),
        Format::Json => to_json(t),
        Format::Html => to_html(t),
        Format::Xlsx => to_xlsx(t)?,
        Format::Pdf => to_pdf(t).0,
    })
}
