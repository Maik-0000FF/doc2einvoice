// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writes the small PDF files the tests read.
//!
//! Every page has A4 size and one font, Helvetica at [`FONT_SIZE`], which
//! every PDF reader knows without the file carrying it. The tests state the
//! drawing instructions of each page themselves.

// Each user of this module needs a different part of it.
#![allow(dead_code)]

use std::fmt::Write;

/// Width of a page in points.
pub const PAGE_WIDTH: f64 = 595.0;
/// Height of a page in points.
pub const PAGE_HEIGHT: f64 = 842.0;
/// Size of the only font, in points.
pub const FONT_SIZE: f64 = 12.0;

const HEADER: &str = "%PDF-1.4\n";
/// Object numbers of the parts every file has; the pages follow them.
const CATALOG: usize = 1;
const PAGE_TREE: usize = 2;
const FONT: usize = 3;
/// Each page takes two objects: the page and its drawing instructions.
const OBJECTS_PER_PAGE: usize = 2;

/// A PDF with one page that is drawn by `content`.
pub fn pdf(content: &str) -> Vec<u8> {
    pdf_of_pages(&[content], "")
}

/// A PDF with one page that the file marks as shown turned by `degrees`
/// clockwise.
pub fn rotated_pdf(content: &str, degrees: u16) -> Vec<u8> {
    pdf_of_pages(&[content], &format!("/Rotate {degrees}"))
}

/// A PDF with one page per entry of `contents`, each drawn by its entry.
pub fn pdf_of(contents: &[&str]) -> Vec<u8> {
    pdf_of_pages(contents, "")
}

/// Instructions that draw `text` with its baseline starting at `x`, `y`,
/// measured from the bottom left corner as a PDF does.
pub fn text_at(x: f64, y: f64, text: &str) -> String {
    format!("BT /F1 {FONT_SIZE} Tf {x} {y} Td ({text}) Tj ET\n")
}

fn pdf_of_pages(contents: &[&str], page_entries: &str) -> Vec<u8> {
    let page_number = |index: usize| FONT + 1 + index * OBJECTS_PER_PAGE;
    // Writing to a string cannot fail, so the results are not looked at.
    let mut kids = String::new();
    for index in 0..contents.len() {
        let _ = write!(kids, "{} 0 R ", page_number(index));
    }

    let mut objects = vec![
        format!("<< /Type /Catalog /Pages {PAGE_TREE} 0 R >>"),
        format!(
            "<< /Type /Pages /Kids [{kids}] /Count {} >>",
            contents.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];
    for (index, content) in contents.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent {PAGE_TREE} 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
             /Resources << /Font << /F1 {FONT} 0 R >> >> /Contents {} 0 R {page_entries} >>",
            page_number(index) + 1
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
    }

    let mut file = String::from(HEADER);
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(file.len());
        let _ = write!(file, "{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let table_offset = file.len();
    let entries = objects.len() + 1;
    let _ = write!(file, "xref\n0 {entries}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(file, "{offset:010} 00000 n ");
    }
    let _ = write!(
        file,
        "trailer\n<< /Size {entries} /Root {CATALOG} 0 R >>\nstartxref\n{table_offset}\n%%EOF\n"
    );
    file.into_bytes()
}
