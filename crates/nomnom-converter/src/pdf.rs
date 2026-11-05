//! PDF output without a PDF library: pictures, plain text and Markdown.
//!
//! Text uses PDF's built-in Helvetica and Courier, so only Western European characters can be
//! written. Anything else becomes "?" and the user is told how many.

use crate::error::ConvertError;
use crate::image_conv;
use crate::model::{Options, PdfPage};
use flate2::{write::ZlibEncoder, Compression};
use image::ImageFormat;
use std::io::Write;
use std::path::Path;

struct Writer {
    objs: Vec<Vec<u8>>,
    pages: Vec<usize>,
}

const CATALOG: usize = 1;
const PAGES: usize = 2;
const F_REGULAR: usize = 3;
const F_BOLD: usize = 4;
const F_MONO: usize = 5;

impl Writer {
    fn new() -> Self {
        let mut w = Writer {
            objs: vec![],
            pages: vec![],
        };
        w.objs.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()); // 1
        w.objs.push(vec![]); // 2, filled in by finish()
        for font in ["Helvetica", "Helvetica-Bold", "Courier"] {
            w.objs.push(
                format!(
                    "<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>"
                )
                .into_bytes(),
            );
        }
        w
    }

    fn add(&mut self, body: Vec<u8>) -> usize {
        self.objs.push(body);
        self.objs.len()
    }

    fn stream(&mut self, dict: &str, data: &[u8]) -> usize {
        let mut b = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
        b.extend_from_slice(data);
        b.extend_from_slice(b"\nendstream");
        self.add(b)
    }

    fn page(&mut self, w: f32, h: f32, content: &[u8], images: &[usize]) {
        let c = self.stream("", content);
        let xobj: String = images
            .iter()
            .enumerate()
            .map(|(i, id)| format!("/Im{i} {id} 0 R "))
            .collect();
        let body = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.2} {h:.2}] /Contents {c} 0 R \
             /Resources << /Font << /F1 {F_REGULAR} 0 R /F2 {F_BOLD} 0 R /F3 {F_MONO} 0 R >> \
             /XObject << {xobj}>> >> >>"
        );
        let id = self.add(body.into_bytes());
        self.pages.push(id);
    }

    fn finish(mut self) -> Vec<u8> {
        let kids: String = self.pages.iter().map(|p| format!("{p} 0 R ")).collect();
        self.objs[PAGES - 1] = format!(
            "<< /Type /Pages /Kids [{kids}] /Count {} >>",
            self.pages.len()
        )
        .into_bytes();
        debug_assert!(!self.objs[CATALOG - 1].is_empty());
        let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::with_capacity(self.objs.len());
        for (i, body) in self.objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", self.objs.len() + 1).as_bytes(),
        );
        for o in offsets {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                self.objs.len() + 1
            )
            .as_bytes(),
        );
        out
    }
}

// ---- pictures ------------------------------------------------------------------------------

fn flate(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = e.write_all(data);
    e.finish().unwrap_or_default()
}

/// One page per picture. JPEG sources are stored as JPEG (small); everything else is stored
/// without loss.
pub fn images_to_pdf(
    paths: &[&Path],
    o: &Options,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Vec<u8>, Vec<String>), ConvertError> {
    let mut w = Writer::new();
    let mut notes = Vec::new();
    for p in paths {
        if cancelled() {
            return Err(ConvertError::Cancelled);
        }
        let d = image_conv::decode(p, true)?;
        let had_alpha = d.had_alpha;
        let was_jpeg = d.format == Some(ImageFormat::Jpeg);
        let img = image_conv::fit(d.image, o.max_width, o.max_height);
        let rgb = image_conv::flatten(&img, o.background);
        let (pw, ph) = (rgb.width(), rgb.height());
        let (filter, data) = if was_jpeg {
            let mut buf = Vec::new();
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 92);
            rgb.write_with_encoder(enc)
                .map_err(|e| ConvertError::Other(format!("couldn't prepare the picture: {e}")))?;
            ("/DCTDecode", buf)
        } else {
            ("/FlateDecode", flate(rgb.as_raw()))
        };
        let im = w.stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {pw} /Height {ph} /ColorSpace /DeviceRGB \
                 /BitsPerComponent 8 /Filter {filter}"
            ),
            &data,
        );
        // 96 pixels per inch.
        let (iw, ih) = (pw as f32 * 0.75, ph as f32 * 0.75);
        match o.pdf_page {
            PdfPage::Fit => {
                let (iw, ih) = (iw.min(14400.0), ih.min(14400.0));
                let content = format!("q {iw:.2} 0 0 {ih:.2} 0 0 cm /Im0 Do Q");
                w.page(iw, ih, content.as_bytes(), &[im]);
            }
            PdfPage::A4 => {
                let (pgw, pgh) = if iw > ih {
                    (842.0, 595.0)
                } else {
                    (595.0, 842.0)
                };
                let margin = 36.0;
                let scale = ((pgw - 2.0 * margin) / iw)
                    .min((pgh - 2.0 * margin) / ih)
                    .min(1.0);
                let (dw, dh) = (iw * scale, ih * scale);
                let (x, y) = ((pgw - dw) / 2.0, (pgh - dh) / 2.0);
                let content = format!("q {dw:.2} 0 0 {dh:.2} {x:.2} {y:.2} cm /Im0 Do Q");
                w.page(pgw, pgh, content.as_bytes(), &[im]);
            }
        }
        if had_alpha {
            notes.push("Transparent areas were filled with a solid colour.".to_string());
        }
    }
    notes.dedup();
    Ok((w.finish(), notes))
}

// ---- text ----------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Face {
    Regular,
    Bold,
    Mono,
}

/// Helvetica widths for ASCII 32..=126, in 1/1000 em.
const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Maps a character to its WinAnsi byte, or None if the font can't show it.
fn win_ansi(c: char) -> Option<u8> {
    let n = c as u32;
    Some(match c {
        '\u{20AC}' => 0x80,
        '\u{2026}' => 0x85,
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201C}' => 0x93,
        '\u{201D}' => 0x94,
        '\u{2022}' => 0x95,
        '\u{2013}' => 0x96,
        '\u{2014}' => 0x97,
        _ if (0x20..0x7F).contains(&n) || (0xA0..=0xFF).contains(&n) => n as u8,
        _ => return None,
    })
}

fn char_width(b: u8, face: Face) -> f32 {
    let em = match face {
        Face::Mono => 600.0,
        _ => {
            let w = match b {
                0x20..=0x7E => f32::from(HELV[(b - 0x20) as usize]),
                0x85 | 0x97 => 1000.0,
                0x91..=0x94 => 300.0,
                0x95 => 350.0,
                0x96 => 556.0,
                0xC0..=0xDF => 667.0,
                _ => 556.0,
            };
            if face == Face::Bold {
                w * 1.08
            } else {
                w
            }
        }
    };
    em / 1000.0
}

fn text_width(bytes: &[u8], size: f32, face: Face) -> f32 {
    bytes.iter().map(|b| char_width(*b, face)).sum::<f32>() * size
}

struct Line {
    indent: f32,
    size: f32,
    face: Face,
    bytes: Vec<u8>,
    space_before: f32,
}

struct Layout {
    w: Writer,
    page_w: f32,
    page_h: f32,
    margin: f32,
    content: Vec<u8>,
    y: f32,
    dropped: usize,
    any_page: bool,
}

impl Layout {
    fn new() -> Self {
        Layout {
            w: Writer::new(),
            page_w: 595.0,
            page_h: 842.0,
            margin: 56.0,
            content: Vec::new(),
            y: 842.0 - 56.0,
            dropped: 0,
            any_page: false,
        }
    }

    fn encode_str(&mut self, s: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(s.len());
        for c in s.chars() {
            if c == '\t' {
                out.extend_from_slice(b"    ");
            } else if c.is_control() {
                continue;
            } else if let Some(b) = win_ansi(c) {
                out.push(b);
            } else {
                self.dropped += 1;
                out.push(b'?');
            }
        }
        out
    }

    fn flush_page(&mut self) {
        let content = std::mem::take(&mut self.content);
        self.w.page(self.page_w, self.page_h, &content, &[]);
        self.any_page = true;
        self.y = self.page_h - self.margin;
    }

    fn put(&mut self, line: &Line) {
        let leading = line.size * 1.35;
        let need = leading + line.space_before;
        if self.y - need < self.margin && !self.content.is_empty() {
            self.flush_page();
        }
        self.y -= need;
        let font = match line.face {
            Face::Regular => "F1",
            Face::Bold => "F2",
            Face::Mono => "F3",
        };
        let mut esc = Vec::with_capacity(line.bytes.len() + 8);
        for &b in &line.bytes {
            if matches!(b, b'(' | b')' | b'\\') {
                esc.push(b'\\');
            }
            esc.push(b);
        }
        let x = self.margin + line.indent;
        let mut op =
            format!("BT /{font} {:.2} Tf {x:.2} {:.2} Td (", line.size, self.y).into_bytes();
        op.extend_from_slice(&esc);
        op.extend_from_slice(b") Tj ET\n");
        self.content.extend_from_slice(&op);
    }

    /// Wraps `text` to the page width and writes it.
    fn paragraph(
        &mut self,
        text: &str,
        size: f32,
        face: Face,
        indent: f32,
        space_before: f32,
        first_prefix: Option<&str>,
    ) {
        let avail = self.page_w - 2.0 * self.margin - indent;
        let mut bytes = self.encode_str(text);
        if face == Face::Mono {
            // Code keeps its spacing and is cut at the edge instead of re-flowed.
            let per = (avail / (size * 0.6)).floor().max(1.0) as usize;
            if bytes.is_empty() {
                self.put(&Line {
                    indent,
                    size,
                    face,
                    bytes,
                    space_before,
                });
                return;
            }
            let mut first = true;
            for chunk in bytes.chunks(per) {
                self.put(&Line {
                    indent,
                    size,
                    face,
                    bytes: chunk.to_vec(),
                    space_before: if first { space_before } else { 0.0 },
                });
                first = false;
            }
            return;
        }
        if let Some(p) = first_prefix {
            let mut pre = self.encode_str(p);
            pre.push(b' ');
            pre.extend_from_slice(&bytes);
            bytes = pre;
        }
        let mut lines: Vec<Vec<u8>> = Vec::new();
        let mut cur: Vec<u8> = Vec::new();
        for word in bytes.split(|b| *b == b' ').filter(|w| !w.is_empty()) {
            let candidate_w = if cur.is_empty() {
                text_width(word, size, face)
            } else {
                text_width(&cur, size, face)
                    + text_width(b" ", size, face)
                    + text_width(word, size, face)
            };
            if candidate_w <= avail {
                if !cur.is_empty() {
                    cur.push(b' ');
                }
                cur.extend_from_slice(word);
            } else {
                if !cur.is_empty() {
                    lines.push(std::mem::take(&mut cur));
                }
                // A word longer than the line is cut where it no longer fits.
                let mut piece: Vec<u8> = Vec::new();
                for &b in word {
                    if text_width(&piece, size, face) + char_width(b, face) * size > avail
                        && !piece.is_empty()
                    {
                        lines.push(std::mem::take(&mut piece));
                    }
                    piece.push(b);
                }
                cur = piece;
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        for (i, l) in lines.into_iter().enumerate() {
            self.put(&Line {
                indent,
                size,
                face,
                bytes: l,
                space_before: if i == 0 { space_before } else { 0.0 },
            });
        }
    }

    fn finish(mut self) -> (Vec<u8>, usize) {
        if !self.content.is_empty() || !self.any_page {
            self.flush_page();
        }
        let dropped = self.dropped;
        (self.w.finish(), dropped)
    }
}

fn dropped_note(n: usize) -> Vec<String> {
    if n == 0 {
        vec![]
    } else {
        vec![format!(
            "{n} character{} couldn't be shown in the PDF's built-in font and became ?.",
            if n == 1 { "" } else { "s" }
        )]
    }
}

/// Reads a text file as UTF-8, or UTF-16 with a byte order mark.
pub fn read_text(path: &Path) -> Result<String, ConvertError> {
    let bytes = std::fs::read(path).map_err(|e| ConvertError::io(path, e))?;
    if bytes.len() > 50 * 1024 * 1024 {
        return Err(ConvertError::Other("This text file is too large.".into()));
    }
    decode_text(&bytes).ok_or(ConvertError::NotReadable("text"))
}

#[allow(clippy::all)]
fn decode_text(bytes: &[u8]) -> Option<String> {
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let u: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&u).ok();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let u: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&u).ok();
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes.to_vec()).ok()
}

pub fn text_to_pdf(text: &str) -> (Vec<u8>, Vec<String>) {
    let mut l = Layout::new();
    for line in text.lines() {
        l.paragraph(line, 10.5, Face::Mono, 0.0, 0.0, None);
    }
    let (pdf, dropped) = l.finish();
    (pdf, dropped_note(dropped))
}

pub fn markdown_to_pdf(md: &str) -> (Vec<u8>, Vec<String>) {
    use pulldown_cmark::{Event, HeadingLevel, Options as MdOptions, Parser, Tag, TagEnd};
    let mut l = Layout::new();
    let mut notes = Vec::new();
    let mut buf = String::new();
    let mut heading: Option<HeadingLevel> = None;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut item_prefix: Option<String> = None;
    let mut skipped_rich = false;
    let mut in_table = false;

    fn flush(
        l: &mut Layout,
        buf: &mut String,
        heading: Option<HeadingLevel>,
        depth: usize,
        prefix: Option<String>,
    ) {
        if buf.trim().is_empty() {
            buf.clear();
            return;
        }
        let text = std::mem::take(buf);
        match heading {
            Some(h) => {
                let size = match h {
                    HeadingLevel::H1 => 20.0,
                    HeadingLevel::H2 => 16.0,
                    HeadingLevel::H3 => 13.5,
                    _ => 12.0,
                };
                l.paragraph(&text, size, Face::Bold, 0.0, size * 0.8, None);
            }
            None => {
                let indent = (depth as f32) * 18.0;
                let before = if prefix.is_some() { 2.0 } else { 7.0 };
                l.paragraph(
                    &text,
                    11.0,
                    Face::Regular,
                    indent,
                    before,
                    prefix.as_deref(),
                );
            }
        }
    }

    for ev in Parser::new_ext(
        md,
        MdOptions::ENABLE_STRIKETHROUGH | MdOptions::ENABLE_TABLES,
    ) {
        if in_table && !matches!(ev, Event::End(TagEnd::Table)) {
            continue;
        }
        match ev {
            Event::Start(Tag::Heading { level, .. }) => heading = Some(level),
            Event::End(TagEnd::Heading(_)) => flush(&mut l, &mut buf, heading.take(), 0, None),
            Event::Start(Tag::CodeBlock(_)) => buf.clear(),
            Event::End(TagEnd::CodeBlock) => {
                for (i, line) in buf.trim_end_matches('\n').split('\n').enumerate() {
                    let before = if i == 0 { 7.0 } else { 0.0 };
                    l.paragraph(line, 9.5, Face::Mono, 12.0, before, None);
                }
                buf.clear();
            }
            Event::Start(Tag::List(start)) => list_stack.push(start),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                item_prefix = Some(match list_stack.last_mut() {
                    Some(Some(n)) => {
                        let s = format!("{n}.");
                        *n += 1;
                        s
                    }
                    _ => "\u{2022}".to_string(),
                });
            }
            Event::End(TagEnd::Item) => {
                flush(&mut l, &mut buf, None, list_stack.len(), item_prefix.take());
            }
            Event::End(TagEnd::Paragraph) => {
                // Inside a list item the text is written with the item.
                if item_prefix.is_none() {
                    flush(&mut l, &mut buf, None, list_stack.len(), None);
                }
            }
            Event::Text(t) | Event::Code(t) => buf.push_str(&t),
            Event::SoftBreak | Event::HardBreak => buf.push(' '),
            Event::Rule => l.paragraph(
                "________________________________",
                11.0,
                Face::Regular,
                0.0,
                6.0,
                None,
            ),
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                skipped_rich = true;
            }
            Event::End(TagEnd::Table) => in_table = false,
            Event::Start(Tag::Image { .. }) => skipped_rich = true,
            Event::Html(_) | Event::InlineHtml(_) => skipped_rich = true,
            _ => {}
        }
    }
    flush(&mut l, &mut buf, None, 0, None);
    if skipped_rich {
        notes.push("Tables, pictures and raw HTML aren't included in the PDF.".to_string());
    }
    let (pdf, dropped) = l.finish();
    notes.extend(dropped_note(dropped));
    (pdf, notes)
}

pub fn markdown_to_html(md: &str, title: &str) -> String {
    use pulldown_cmark::{html, Event, Options as MdOptions, Parser};
    let parser = Parser::new_ext(
        md,
        MdOptions::ENABLE_TABLES | MdOptions::ENABLE_STRIKETHROUGH,
    )
    // Raw HTML in the Markdown is shown as text, never run.
    .map(|e| match e {
        Event::Html(h) | Event::InlineHtml(h) => Event::Text(h),
        other => other,
    });
    let mut body = String::new();
    html::push_html(&mut body, parser);
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n\
         <style>body{{font:16px/1.6 system-ui,sans-serif;max-width:46rem;margin:2rem auto;padding:0 1rem;color:#222}}\
         pre,code{{font-family:Consolas,monospace;background:#f3f3f3;border-radius:4px}}\
         pre{{padding:.8rem;overflow:auto}}code{{padding:.1rem .3rem}}pre code{{padding:0}}\
         table{{border-collapse:collapse}}td,th{{border:1px solid #ccc;padding:.3rem .6rem}}\
         img{{max-width:100%}}blockquote{{border-left:3px solid #ccc;margin-left:0;padding-left:1rem;color:#555}}</style>\n\
         </head>\n<body>\n{body}</body>\n</html>\n",
        esc(title)
    )
}

pub fn validate_pdf(path: &Path, min_pages: usize) -> Result<usize, ConvertError> {
    let doc = lopdf::Document::load(path)
        .map_err(|_| ConvertError::BadOutput("The result can't be opened as a PDF."))?;
    let pages = doc.get_pages().len();
    if pages < min_pages.max(1) {
        return Err(ConvertError::BadOutput("The PDF has no pages."));
    }
    Ok(pages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(bytes: &[u8]) -> lopdf::Document {
        lopdf::Document::load_mem(bytes).expect("valid pdf")
    }

    fn first_page_content(doc: &lopdf::Document) -> Vec<u8> {
        let page = *doc.get_pages().values().next().unwrap();
        doc.get_page_content(page).unwrap()
    }

    #[test]
    fn plain_text_makes_a_valid_pdf_and_flows_onto_more_pages() {
        let text = (0..400)
            .map(|i| format!("line number {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (pdf, notes) = text_to_pdf(&text);
        let doc = load(&pdf);
        assert!(doc.get_pages().len() >= 6, "{}", doc.get_pages().len());
        assert!(notes.is_empty());
    }

    #[test]
    fn empty_text_still_gives_one_page() {
        let (pdf, _) = text_to_pdf("");
        assert_eq!(load(&pdf).get_pages().len(), 1);
    }

    #[test]
    fn accents_are_kept_and_other_scripts_are_reported() {
        let (pdf, notes) = text_to_pdf("café — “quoted” 日本語");
        let content = first_page_content(&load(&pdf));
        // é is 0xE9 in WinAnsi, the em dash is 0x97.
        assert!(content.contains(&0xE9));
        assert!(content.contains(&0x97));
        assert_eq!(notes.len(), 1);
        assert!(notes[0].starts_with("3 characters"), "{notes:?}");
    }

    #[test]
    fn parentheses_and_backslashes_do_not_break_the_page() {
        let (pdf, _) = text_to_pdf("a (b) \\ c");
        let content = first_page_content(&load(&pdf));
        assert!(String::from_utf8_lossy(&content).contains("\\(b\\)"));
    }

    #[test]
    fn very_long_lines_are_cut_not_overflowed() {
        let (pdf, _) = text_to_pdf(&"x".repeat(5000));
        assert!(!load(&pdf).get_pages().is_empty());
    }

    #[test]
    fn markdown_headings_lists_and_code_are_laid_out() {
        let md = "# Title\n\nSome *text* here.\n\n- one\n- two\n\n1. first\n2. second\n\n```\ncode line\n```\n";
        let (pdf, notes) = markdown_to_pdf(md);
        let text = String::from_utf8_lossy(&first_page_content(&load(&pdf))).to_string();
        assert!(text.contains("(Title)"), "{text}");
        assert!(text.contains("first"));
        assert!(text.contains("code line"));
        assert!(notes.is_empty());
    }

    #[test]
    fn markdown_tables_are_reported_as_left_out() {
        let (_, notes) = markdown_to_pdf("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(notes.iter().any(|n| n.contains("Tables")));
    }

    #[test]
    fn html_output_never_runs_raw_html() {
        let html = markdown_to_html("# Hi\n\n<script>alert(1)</script>\n", "t");
        assert!(!html.contains("<script>alert"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("<h1>Hi</h1>"));
    }

    #[test]
    fn utf16_and_bom_text_is_read() {
        assert_eq!(
            decode_text(&[0xEF, 0xBB, 0xBF, b'h', b'i']).as_deref(),
            Some("hi")
        );
        assert_eq!(
            decode_text(&[0xFF, 0xFE, b'h', 0, b'i', 0]).as_deref(),
            Some("hi")
        );
        assert_eq!(decode_text(&[b'a', 0, b'b']), None);
    }
}
