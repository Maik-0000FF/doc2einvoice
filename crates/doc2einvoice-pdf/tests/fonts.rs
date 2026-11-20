// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a font does to the size, the advance and the text of its characters.
//!
//! Input table. Each row is one test below.
//!
//! | page                                                 | expected                          |
//! |------------------------------------------------------|-----------------------------------|
//! | a font the file does not carry and no reader knows,  | the stated font size; words as    |
//! | with widths of its own                               | the widths of the file place them |
//! | text condensed to half its width                     | the stated font size; the words   |
//! |                                                      | half as wide; words kept apart    |
//! | a word whose letters are spaced out                  | one word                          |
//! | doubled letters and digits in heavily condensed text | every character is read           |
//! | text that is filled and outlined, to look bold       | every character once              |
//! | text that is outlined only                           | read                              |
//! | a character the file maps to a very long text        | not kept, counted as unreadable   |
//! | a character the file maps to a ligature              | kept with its letters             |

mod support;

use doc2einvoice_pdf::{Glyph, PageLayout, PdfError, Word, read};
use support::{FIRST_EXTRA_OBJECT, FONT_SIZE, pdf, pdf_with_font, stream, text_at};

const X: f64 = 50.0;
const Y: f64 = 700.0;
const TOLERANCE: f64 = 0.01;

/// Codes of the characters the unknown font covers: the printable ASCII.
const FIRST_CODE: u8 = 32;
const LAST_CODE: u8 = 126;
/// Every character of the unknown font is this wide, in thousandths of the
/// font size. No real font has such widths, so a reader that takes the
/// widths of a substitute instead is found out.
const UNKNOWN_FONT_WIDTH: u16 = 700;

/// A font that is not in the file and that no reader can know by its name.
fn unknown_font() -> (String, Vec<String>) {
    let widths: Vec<String> = (FIRST_CODE..=LAST_CODE)
        .map(|_| UNKNOWN_FONT_WIDTH.to_string())
        .collect();
    let font = format!(
        "<< /Type /Font /Subtype /TrueType /BaseFont /Quillon /Encoding /WinAnsiEncoding \
         /FirstChar {FIRST_CODE} /LastChar {LAST_CODE} /Widths [{}] \
         /FontDescriptor {FIRST_EXTRA_OBJECT} 0 R >>",
        widths.join(" ")
    );
    let descriptor = "<< /Type /FontDescriptor /FontName /Quillon /Flags 32 \
                      /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 \
                      /CapHeight 700 /StemV 80 >>"
        .to_owned();
    (font, vec![descriptor])
}

/// Helvetica with a table that maps the letter A to `text`, given as the
/// hexadecimal digits of its UTF-16 code units.
fn font_mapping_a_to(text_hex: &str) -> (String, Vec<String>) {
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding \
         /ToUnicode {FIRST_EXTRA_OBJECT} 0 R >>"
    );
    let table = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
         1 beginbfchar\n<41> <{text_hex}>\nendbfchar\n\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend"
    );
    (font, vec![stream(&table)])
}

fn words_of(data: &[u8]) -> Result<Vec<Word>, PdfError> {
    let pages = read(data)?;
    Ok(PageLayout::of(&pages[0])
        .blocks
        .into_iter()
        .flat_map(|block| block.lines)
        .flat_map(|line| line.words)
        .collect())
}

fn texts(words: &[Word]) -> Vec<&str> {
    words.iter().map(|word| word.text.as_str()).collect()
}

fn has_stated_size(glyphs: &[Glyph]) -> bool {
    glyphs
        .iter()
        .all(|glyph| (glyph.font_size - FONT_SIZE).abs() < TOLERANCE)
}

#[test]
fn an_unknown_font_keeps_its_size_and_widths() -> Result<(), PdfError> {
    let (font, extra) = unknown_font();
    let data = pdf_with_font(&text_at(X, Y, "Invoice 4711"), &font, &extra);
    let pages = read(&data)?;
    let words = words_of(&data)?;

    assert!(has_stated_size(&pages[0].glyphs));
    assert_eq!(texts(&words), ["Invoice", "4711"]);
    let letters = "Invoice".len();
    let expected_width =
        f64::from(u32::try_from(letters).unwrap_or(0)) * FONT_SIZE * f64::from(UNKNOWN_FONT_WIDTH)
            / 1000.0;
    assert!((words[0].right - words[0].left - expected_width).abs() < TOLERANCE);
    Ok(())
}

#[test]
fn condensed_text_keeps_its_size() -> Result<(), PdfError> {
    let plain = format!("BT /F1 {FONT_SIZE} Tf {X} {Y} Td (Invoice 4711) Tj ET");
    let condensed = format!("BT /F1 {FONT_SIZE} Tf 50 Tz {X} {Y} Td (Invoice 4711) Tj ET");
    let plain_words = words_of(&pdf(&plain))?;
    let condensed_data = pdf(&condensed);
    let condensed_words = words_of(&condensed_data)?;

    assert!(has_stated_size(&read(&condensed_data)?[0].glyphs));
    assert_eq!(texts(&condensed_words), ["Invoice", "4711"]);
    let plain_width = plain_words[0].right - plain_words[0].left;
    let condensed_width = condensed_words[0].right - condensed_words[0].left;
    assert!((condensed_width - plain_width / 2.0).abs() < TOLERANCE);
    Ok(())
}

#[test]
fn spaced_out_letters_are_one_word() -> Result<(), PdfError> {
    // Every letter is followed by three points of extra space.
    let content = format!("BT /F1 {FONT_SIZE} Tf 3 Tc {X} {Y} Td (TOTAL) Tj ET");

    assert_eq!(texts(&words_of(&pdf(&content))?), ["TOTAL"]);
    Ok(())
}

#[test]
fn condensed_text_keeps_its_doubled_letters() -> Result<(), PdfError> {
    let content = format!("BT /F1 {FONT_SIZE} Tf 40 Tz {X} {Y} Td (bill ill 11) Tj ET");
    let data = pdf(&content);

    assert_eq!(texts(&words_of(&data)?), ["bill", "ill", "11"]);
    assert_eq!(PageLayout::of(&read(&data)?[0]).copied_glyphs, 0);
    Ok(())
}

#[test]
fn filled_and_outlined_text_is_read_once() -> Result<(), PdfError> {
    const WORD: &str = "Total";
    // Mode 2 fills the letters and draws their contours.
    let content = format!("BT /F1 {FONT_SIZE} Tf 2 Tr {X} {Y} Td ({WORD}) Tj ET");
    let pages = read(&pdf(&content))?;

    assert_eq!(pages[0].glyphs.len(), WORD.len());
    assert_eq!(PageLayout::of(&pages[0]).copied_glyphs, 0);
    Ok(())
}

#[test]
fn outlined_text_is_read() -> Result<(), PdfError> {
    // Mode 1 draws the contours only.
    let content = format!("BT /F1 {FONT_SIZE} Tf 1 Tr {X} {Y} Td (Total) Tj ET");

    assert_eq!(texts(&words_of(&pdf(&content))?), ["Total"]);
    Ok(())
}

#[test]
fn a_character_mapped_to_a_long_text_is_not_kept() -> Result<(), PdfError> {
    const REPETITIONS: usize = 100;
    let (font, extra) = font_mapping_a_to(&"0058".repeat(REPETITIONS));
    let pages = read(&pdf_with_font(&text_at(X, Y, "AB"), &font, &extra))?;

    let kept: Vec<&str> = pages[0].glyphs.iter().map(|g| g.text.as_str()).collect();
    assert_eq!(kept, ["B"]);
    assert_eq!(pages[0].unreadable_glyphs, 1);
    Ok(())
}

#[test]
fn a_character_mapped_to_a_ligature_is_kept() -> Result<(), PdfError> {
    // The letters f, f and i.
    let (font, extra) = font_mapping_a_to("006600660069");
    let pages = read(&pdf_with_font(&text_at(X, Y, "A"), &font, &extra))?;

    let kept: Vec<&str> = pages[0].glyphs.iter().map(|g| g.text.as_str()).collect();
    assert_eq!(kept, ["ffi"]);
    Ok(())
}
