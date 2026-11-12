use nomnom_storage::analyze::{self, FolderMode};
use nomnom_storage::report::{self, Format, Value};
use nomnom_storage::{ScanOptions, Session};
use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

fn put(path: &Path, size: usize) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![5u8; size]).unwrap();
}

fn table() -> report::Table {
    let d = tempfile::tempdir().unwrap();
    let r = d.path();
    put(&r.join("movies/clip, final; v2.mp4"), 5000);
    put(&r.join("=SUM(A1)/x.txt"), 100);
    put(&r.join("docs/bold & bright.txt"), 700);
    put(&r.join("docs/deep/er/file.bin"), 90);
    let s = Session::start(1, r.to_path_buf(), ScanOptions::default());
    let t = Instant::now();
    while !s.is_finished() {
        assert!(t.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(10));
    }
    let idx = s.index.read().unwrap();
    report::tree_table(&idx, 0, 3)
}

/// A table with a name Windows wouldn't allow on disk but that a path from elsewhere could hold.
fn evil() -> report::Table {
    let mut t = table();
    let mut row = t.rows[0].clone();
    row[0] = Value::Text("<b>bold</b>.txt".into());
    t.rows = vec![row];
    t
}

#[test]
fn the_date_helper_is_right_for_known_moments() {
    assert_eq!(Value::Time(Some(0)).display(), "1970-01-01 00:00");
    assert_eq!(
        Value::Time(Some(1_700_000_000)).display(),
        "2023-11-14 22:13"
    );
    assert_eq!(Value::Time(None).display(), "");
    assert_eq!(report::human_bytes(1536), "1.5 KB");
}

#[test]
fn csv_quotes_cells_and_blocks_formula_injection() {
    let t = table();
    let csv = String::from_utf8(report::to_csv(&t)).unwrap();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("\"clip, final; v2.mp4\""), "{csv}");
    assert!(
        csv.contains("'=SUM(A1)"),
        "a name starting with = must not become a formula"
    );
    let header = csv.lines().next().unwrap().trim_start_matches('\u{feff}');
    assert_eq!(
        header,
        "Name,Path,Type,Size,On disk,Files,Folders,Modified,% of folder"
    );
    // Sizes are plain byte counts for a spreadsheet to sum.
    assert!(csv.lines().any(|l| l.contains(",File,5000,")), "{csv}");
}

#[test]
fn json_has_numbers_not_text_for_sizes() {
    let t = table();
    let v: serde_json::Value = serde_json::from_slice(&report::to_json(&t)).unwrap();
    let rows = v["rows"].as_array().unwrap();
    assert_eq!(rows.len(), t.rows.len());
    assert!(rows
        .iter()
        .any(|r| r["size"] == 5000 && r["type"] == "File"));
    assert!(v["note"].as_str().unwrap().contains("3 levels"));
}

#[test]
fn html_escapes_names_so_nothing_runs() {
    let t = evil();
    let html = String::from_utf8(report::to_html(&t)).unwrap();
    assert!(!html.contains("<b>bold</b>"));
    assert!(html.contains("&lt;b&gt;bold&lt;/b&gt;.txt"));
    assert!(html.starts_with("<!doctype html>"));
    assert!(!html.contains("<script"));
}

#[test]
fn xlsx_is_a_zip_with_the_right_parts_and_numbers() {
    let mut t = table();
    t.rows.push(evil().rows.remove(0));
    let bytes = report::to_xlsx(&t).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut names: Vec<String> = (0..z.len())
        .map(|i| z.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "[Content_Types].xml",
            "_rels/.rels",
            "xl/_rels/workbook.xml.rels",
            "xl/styles.xml",
            "xl/workbook.xml",
            "xl/worksheets/sheet1.xml"
        ]
    );
    let mut sheet = String::new();
    z.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet)
        .unwrap();
    assert!(sheet.contains("<c r=\"A1\" s=\"1\" t=\"inlineStr\">"));
    assert!(sheet.contains("<v>5000</v>"));
    assert!(sheet.contains("&lt;b&gt;bold"), "text is escaped");
    assert_eq!(sheet.matches("<row ").count(), t.rows.len() + 1);
    assert_eq!(
        sheet.matches("<row ").count(),
        sheet.matches("</row>").count()
    );
    assert!(sheet.ends_with("</sheetData></worksheet>"));
}

#[test]
fn pdf_opens_and_has_pages() {
    let t = table();
    let (pdf, notes) = report::to_pdf(&t);
    let doc = lopdf::Document::load_mem(&pdf).unwrap();
    assert!(!doc.get_pages().is_empty());
    assert!(notes.iter().any(|n| n.contains("CSV or Excel")));
}

#[test]
fn columns_can_be_chosen_and_reordered() {
    let t = table();
    let s = report::select_columns(&t, &["size".into(), "name".into(), "bogus".into()]);
    assert_eq!(
        s.columns.iter().map(|c| c.key).collect::<Vec<_>>(),
        ["size", "name"]
    );
    assert_eq!(s.rows[0].len(), 2);
    assert_eq!(
        report::select_columns(&t, &[]).columns.len(),
        t.columns.len()
    );
}

#[test]
fn every_format_renders_the_other_report_kinds_too() {
    let d = tempfile::tempdir().unwrap();
    put(&d.path().join("a/x.mp4"), 3000);
    put(&d.path().join("b/y.mp4"), 2000);
    let s = Session::start(1, d.path().to_path_buf(), ScanOptions::default());
    while !s.is_finished() {
        std::thread::sleep(Duration::from_millis(10));
    }
    let idx = s.index.read().unwrap();
    let tables = [
        report::top_files_table(&idx, 0, 10),
        report::top_folders_table(&idx, 0, 10, FolderMode::Inclusive),
        report::top_folders_table(&idx, 0, 10, FolderMode::Exclusive),
        report::groups_table("By type", "Extension", &analyze::by_extension(&idx, 0, 50)),
        report::groups_table("By kind", "Kind", &analyze::by_category(&idx, 0)),
    ];
    assert!(
        tables[1].note.as_ref().unwrap().contains("overlap"),
        "the overlap warning travels with the report"
    );
    for t in &tables {
        for f in [
            Format::Csv,
            Format::Json,
            Format::Html,
            Format::Xlsx,
            Format::Pdf,
        ] {
            let out = report::render(t, f).unwrap();
            assert!(!out.is_empty(), "{} as {:?}", t.title, f);
        }
    }
}
