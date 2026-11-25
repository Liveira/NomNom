//! Merging PDFs into one and taking pages out of a PDF. The originals are only read; results go to a
//! temporary file and are moved into place under a name that isn't taken.

use crate::naming;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn err(msg: impl Into<String>) -> String {
    msg.into()
}

fn load(path: &Path) -> Result<Document, String> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let doc = Document::load(path).map_err(|_| err(format!("{name} can't be read as a PDF.")))?;
    if doc.is_encrypted() {
        return Err(err(format!(
            "{name} is protected with a password. Remove the password first."
        )));
    }
    if doc.get_pages().is_empty() {
        return Err(err(format!("{name} has no pages.")));
    }
    Ok(doc)
}

/// How many pages a PDF has.
pub fn page_count(path: &Path) -> Result<u32, String> {
    Ok(load(path)?.get_pages().len() as u32)
}

/// Pages from text like "1-3, 5, 8-": a list of page numbers in order, without repeats.
pub fn parse_pages(spec: &str, total: u32) -> Result<Vec<u32>, String> {
    let mut out: Vec<u32> = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (a, b) = match part.split_once('-') {
            Some((a, b)) => (a.trim(), Some(b.trim())),
            None => (part, None),
        };
        let from: u32 = if a.is_empty() {
            1
        } else {
            a.parse()
                .map_err(|_| err(format!("\"{part}\" isn't a page number.")))?
        };
        let to: u32 = match b {
            None => from,
            Some("") => total,
            Some(b) => b
                .parse()
                .map_err(|_| err(format!("\"{part}\" isn't a page number.")))?,
        };
        if from == 0 || to == 0 || from > to {
            return Err(err(format!("\"{part}\" isn't a valid range of pages.")));
        }
        if to > total {
            return Err(err(format!("This PDF has only {total} pages.")));
        }
        for p in from..=to {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    if out.is_empty() {
        return Err(err("Type the pages you want, like 1-3, 5."));
    }
    Ok(out)
}

/// Gives each page the attributes it would otherwise inherit from its parents, so a page keeps its
/// size and resources when it is moved under a new Pages node.
fn flatten_inherited(doc: &mut Document) {
    const KEYS: [&[u8]; 4] = [b"MediaBox", b"CropBox", b"Rotate", b"Resources"];
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for id in pages {
        let mut found: BTreeMap<Vec<u8>, Object> = BTreeMap::new();
        let mut cur = Some(id);
        let mut hops = 0;
        while let Some(c) = cur {
            hops += 1;
            if hops > 64 {
                break;
            }
            let Ok(dict) = doc.get_dictionary(c) else {
                break;
            };
            for k in KEYS {
                if let Ok(v) = dict.get(k) {
                    found.entry(k.to_vec()).or_insert_with(|| v.clone());
                }
            }
            cur = dict.get(b"Parent").ok().and_then(|p| p.as_reference().ok());
        }
        if let Ok(Object::Dictionary(page)) = doc.get_object_mut(id) {
            for (k, v) in found {
                if !page.has(&k) {
                    page.set(k, v);
                }
            }
        }
    }
}

fn save_new(mut doc: Document, dir: &Path, stem: &str) -> Result<PathBuf, String> {
    let tmp = naming::temp_for(dir, "pdf");
    doc.save(&tmp)
        .map_err(|e| err(format!("The PDF couldn't be written: {e}")))?;
    match naming::place(&tmp, dir, stem, "pdf") {
        Ok(p) => Ok(p),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(err(format!("The PDF couldn't be saved: {e}")))
        }
    }
}

/// Joins PDFs, in the order given, into one new file in `dir`.
pub fn merge(inputs: &[PathBuf], dir: &Path, stem: &str) -> Result<PathBuf, String> {
    if inputs.len() < 2 {
        return Err(err("Pick at least two PDFs to join."));
    }
    let mut max_id = 1;
    let mut pages: Vec<(ObjectId, Object)> = Vec::new();
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    for path in inputs {
        let mut doc = load(path)?;
        flatten_inherited(&mut doc);
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;
        for (_, id) in doc.get_pages() {
            if let Ok(obj) = doc.get_object(id) {
                pages.push((id, obj.to_owned()));
            }
        }
        objects.extend(doc.objects);
    }

    let mut out = Document::with_version("1.5");
    let mut catalog: Option<(ObjectId, Object)> = None;
    let mut pages_node: Option<(ObjectId, Object)> = None;
    for (id, obj) in &objects {
        match obj.type_name().unwrap_or("") {
            "Catalog" => {
                catalog = Some((catalog.map(|(i, _)| i).unwrap_or(*id), obj.clone()));
            }
            "Pages" => {
                if let Ok(d) = obj.as_dict() {
                    let mut dict = d.clone();
                    if let Some((_, Object::Dictionary(old))) = &pages_node {
                        dict.extend(old);
                    }
                    pages_node = Some((
                        pages_node.as_ref().map(|(i, _)| *i).unwrap_or(*id),
                        Object::Dictionary(dict),
                    ));
                }
            }
            "Page" | "Outlines" | "Outline" => {}
            _ => {
                out.objects.insert(*id, obj.clone());
            }
        }
    }
    let (Some((cat_id, cat)), Some((pages_id, pages_obj))) = (catalog, pages_node) else {
        return Err(err(
            "These PDFs have an unusual structure and can't be joined.",
        ));
    };
    let count = pages.len() as u32;
    let kids: Vec<Object> = pages.iter().map(|(k, _)| Object::Reference(*k)).collect();
    for (id, obj) in pages {
        if let Ok(d) = obj.as_dict() {
            let mut d: Dictionary = d.clone();
            d.set("Parent", pages_id);
            out.objects.insert(id, Object::Dictionary(d));
        }
    }
    if let Ok(d) = pages_obj.as_dict() {
        let mut d = d.clone();
        d.set("Count", count);
        d.set("Kids", kids);
        // These belonged to one of the inputs, and no longer fit all pages.
        for k in ["MediaBox", "CropBox", "Rotate", "Resources"] {
            d.remove(k.as_bytes());
        }
        out.objects.insert(pages_id, Object::Dictionary(d));
    }
    if let Ok(d) = cat.as_dict() {
        let mut d = d.clone();
        d.set("Pages", pages_id);
        d.remove(b"Outlines");
        out.objects.insert(cat_id, Object::Dictionary(d));
    }
    out.trailer.set("Root", cat_id);
    out.max_id = out.objects.len() as u32;
    out.renumber_objects();
    out.compress();
    save_new(out, dir, stem)
}

/// One new PDF with only the given pages (1-based, in that order is not kept: the original order is).
pub fn extract(input: &Path, wanted: &[u32], dir: &Path, stem: &str) -> Result<PathBuf, String> {
    let mut doc = load(input)?;
    let total = doc.get_pages().len() as u32;
    let drop: Vec<u32> = (1..=total).filter(|p| !wanted.contains(p)).collect();
    if drop.len() as u32 == total {
        return Err(err("Pick at least one page."));
    }
    flatten_inherited(&mut doc);
    doc.delete_pages(&drop);
    doc.prune_objects();
    doc.compress();
    save_new(doc, dir, stem)
}

/// Every page of the PDF as its own file. Returns the new files.
pub fn split_each(input: &Path, dir: &Path, stem: &str) -> Result<Vec<PathBuf>, String> {
    let total = load(input)?.get_pages().len() as u32;
    if total > 500 {
        return Err(err(
            "That PDF has more than 500 pages. Take out a range of pages instead.",
        ));
    }
    let width = total.to_string().len().max(2);
    let mut out = Vec::new();
    for p in 1..=total {
        out.push(extract(
            input,
            &[p],
            dir,
            &format!("{stem} - page {p:0width$}"),
        )?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Stream};

    /// A small valid PDF with `pages` pages. Page n draws "n" lines so contents differ.
    fn make(path: &Path, pages: u32, media: [i64; 4]) {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for n in 1..=pages {
            let content: Content<Vec<Operation>> = Content {
                operations: (0..n)
                    .map(|i| Operation::new("l", vec![(10 + i as i64).into(), 20.into()]))
                    .collect(),
            };
            let cid = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let pid = doc.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id, "Contents" => cid,
            });
            kids.push(pid.into());
        }
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => kids, "Count" => pages,
                "MediaBox" => vec![media[0].into(), media[1].into(), media[2].into(), media[3].into()],
                "Resources" => dictionary! {},
            }),
        );
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", cat);
        doc.save(path).unwrap();
    }

    #[test]
    fn page_ranges_are_read() {
        assert_eq!(parse_pages("1-3, 5", 10).unwrap(), [1, 2, 3, 5]);
        assert_eq!(parse_pages("8-", 10).unwrap(), [8, 9, 10]);
        assert_eq!(parse_pages("-2", 10).unwrap(), [1, 2]);
        assert_eq!(parse_pages("2,2,1", 10).unwrap(), [2, 1]);
        assert!(parse_pages("0", 10).is_err());
        assert!(parse_pages("3-1", 10).is_err());
        assert!(parse_pages("11", 10).is_err());
        assert!(parse_pages("abc", 10).is_err());
        assert!(parse_pages("  ", 10).is_err());
    }

    #[test]
    fn joins_pdfs_in_order_and_keeps_each_pages_size() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a.pdf"), d.path().join("b.pdf"));
        make(&a, 2, [0, 0, 200, 300]);
        make(&b, 3, [0, 0, 500, 700]);
        let out = merge(&[a.clone(), b.clone()], d.path(), "merged").unwrap();
        let doc = Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 5);
        let sizes: Vec<i64> = doc
            .get_pages()
            .values()
            .map(|id| {
                let d = doc.get_dictionary(*id).unwrap();
                d.get(b"MediaBox").unwrap().as_array().unwrap()[2]
                    .as_i64()
                    .unwrap()
            })
            .collect();
        assert_eq!(sizes, [200, 200, 500, 500, 500]);
        // The inputs are untouched.
        assert_eq!(page_count(&a).unwrap(), 2);
        assert_eq!(page_count(&b).unwrap(), 3);
    }

    #[test]
    fn never_replaces_an_existing_file() {
        let d = tempfile::tempdir().unwrap();
        let (a, b) = (d.path().join("a.pdf"), d.path().join("b.pdf"));
        make(&a, 1, [0, 0, 100, 100]);
        make(&b, 1, [0, 0, 100, 100]);
        std::fs::write(d.path().join("merged.pdf"), b"mine").unwrap();
        let out = merge(&[a, b], d.path(), "merged").unwrap();
        assert_ne!(out.file_name().unwrap(), "merged.pdf");
        assert_eq!(std::fs::read(d.path().join("merged.pdf")).unwrap(), b"mine");
    }

    #[test]
    fn needs_two_files_and_refuses_what_is_not_a_pdf() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.pdf");
        make(&a, 1, [0, 0, 100, 100]);
        assert!(merge(std::slice::from_ref(&a), d.path(), "m").is_err());
        let bad = d.path().join("bad.pdf");
        std::fs::write(&bad, b"not a pdf").unwrap();
        let e = merge(&[a, bad], d.path(), "m").unwrap_err();
        assert!(e.contains("bad.pdf"), "{e}");
    }

    #[test]
    fn takes_chosen_pages_out_into_a_new_file() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.pdf");
        make(&a, 5, [0, 0, 100, 100]);
        let out = extract(&a, &[2, 4], d.path(), "a - part").unwrap();
        assert_eq!(page_count(&out).unwrap(), 2);
        assert_eq!(page_count(&a).unwrap(), 5);
        assert!(extract(&a, &[], d.path(), "x").is_err());
    }

    #[test]
    fn splits_every_page_into_its_own_file() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.pdf");
        make(&a, 3, [0, 0, 100, 100]);
        let files = split_each(&a, d.path(), "a").unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|f| page_count(f).unwrap() == 1));
        let names: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            ["a - page 01.pdf", "a - page 02.pdf", "a - page 03.pdf"]
        );
    }
}
