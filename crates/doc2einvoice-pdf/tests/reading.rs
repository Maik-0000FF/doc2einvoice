// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading a PDF and ordering its text, as a whole.
//!
//! Input table. Each row is one test below.
//!
//! | page                                                | expected                             |
//! |-----------------------------------------------------|--------------------------------------|
//! | two words with a space between them                 | one line of two words, at its place  |
//! | a word whose letters are kerned                     | one word                             |
//! | two words set apart without a space                 | two words of one line                |
//! | two table cells on one baseline                     | two blocks                           |
//! | a cell wrapped over two lines beside another cell   | the wrapped cell is one block        |
//! | two lines far below each other                      | two blocks                           |
//! | the same text drawn in another order                | the same layout                      |
//! | a page shown turned, with text upright as shown     | read upright, page size as shown     |
//! | text written at an angle across the page            | left out and counted                 |
//! | a word drawn twice, slightly moved, to look bold    | read once, the copy is counted       |
//! | a page without text                                 | a page without glyphs and blocks     |
//! | no data, and data that is no PDF                    | `Invalid`                            |

mod support;

use doc2einvoice_pdf::{PageLayout, PdfError, read};
use support::{FONT_SIZE, PAGE_HEIGHT, PAGE_WIDTH, pdf, rotated_pdf, text_at};

const LEFT_COLUMN: f64 = 50.0;
const RIGHT_COLUMN: f64 = 400.0;
const FIRST_BASELINE: f64 = 700.0;
/// The distance of two lines of a paragraph.
const LINE_PITCH: f64 = 14.0;
/// How far a place may be off what the instructions state.
const PLACE_TOLERANCE: f64 = 0.01;

/// The layout of the only page of a PDF drawn by `content`.
fn layout_of(content: &str) -> Result<PageLayout, PdfError> {
    let pages = read(&pdf(content))?;
    assert_eq!(pages.len(), 1);
    Ok(PageLayout::of(&pages[0]))
}

fn block_texts(layout: &PageLayout) -> Vec<String> {
    layout
        .blocks
        .iter()
        .map(doc2einvoice_pdf::Block::text)
        .collect()
}

#[test]
fn a_space_separates_words() -> Result<(), PdfError> {
    let layout = layout_of(&text_at(LEFT_COLUMN, FIRST_BASELINE, "Invoice 4711"))?;

    assert_eq!(block_texts(&layout), ["Invoice 4711"]);
    let words = &layout.blocks[0].lines[0].words;
    assert_eq!(words.len(), 2);
    assert!((words[0].left - LEFT_COLUMN).abs() < PLACE_TOLERANCE);
    assert!((words[0].baseline - (PAGE_HEIGHT - FIRST_BASELINE)).abs() < PLACE_TOLERANCE);
    assert!((words[0].font_size - FONT_SIZE).abs() < PLACE_TOLERANCE);
    assert!(words[0].right > words[0].left && words[1].left > words[0].right);
    Ok(())
}

#[test]
fn kerning_does_not_split_a_word() -> Result<(), PdfError> {
    // The number moves the following letters by 2 % of the font size.
    let content =
        format!("BT /F1 {FONT_SIZE} Tf {LEFT_COLUMN} {FIRST_BASELINE} Td [(Inv) -20 (oice)] TJ ET");

    assert_eq!(block_texts(&layout_of(&content)?), ["Invoice"]);
    Ok(())
}

#[test]
fn a_gap_without_a_space_separates_words() -> Result<(), PdfError> {
    // The number moves the following letters by 40 % of the font size.
    let content = format!(
        "BT /F1 {FONT_SIZE} Tf {LEFT_COLUMN} {FIRST_BASELINE} Td [(Net) -400 (100,00)] TJ ET"
    );
    let layout = layout_of(&content)?;

    assert_eq!(block_texts(&layout), ["Net 100,00"]);
    assert_eq!(layout.blocks[0].lines[0].words.len(), 2);
    Ok(())
}

#[test]
fn cells_on_one_baseline_are_separate_blocks() -> Result<(), PdfError> {
    let content = text_at(LEFT_COLUMN, FIRST_BASELINE, "Blue pen")
        + &text_at(RIGHT_COLUMN, FIRST_BASELINE, "9,90");

    assert_eq!(block_texts(&layout_of(&content)?), ["Blue pen", "9,90"]);
    Ok(())
}

#[test]
fn a_wrapped_cell_stays_together() -> Result<(), PdfError> {
    let content = text_at(LEFT_COLUMN, FIRST_BASELINE, "Blue pen with")
        + &text_at(RIGHT_COLUMN, FIRST_BASELINE, "9,90")
        + &text_at(LEFT_COLUMN, FIRST_BASELINE - LINE_PITCH, "a fine tip");

    assert_eq!(
        block_texts(&layout_of(&content)?),
        ["Blue pen with\na fine tip", "9,90"]
    );
    Ok(())
}

#[test]
fn lines_far_apart_are_separate_blocks() -> Result<(), PdfError> {
    let far_below = FIRST_BASELINE - 10.0 * LINE_PITCH;
    let content = text_at(LEFT_COLUMN, FIRST_BASELINE, "Invoice")
        + &text_at(LEFT_COLUMN, far_below, "Thank you");

    assert_eq!(block_texts(&layout_of(&content)?), ["Invoice", "Thank you"]);
    Ok(())
}

#[test]
fn the_drawing_order_does_not_matter() -> Result<(), PdfError> {
    let parts = [
        text_at(LEFT_COLUMN, FIRST_BASELINE, "Blue pen with"),
        text_at(RIGHT_COLUMN, FIRST_BASELINE, "9,90"),
        text_at(LEFT_COLUMN, FIRST_BASELINE - LINE_PITCH, "a fine tip"),
    ];
    let forwards = parts.concat();
    let backwards: String = parts.iter().rev().map(String::as_str).collect();

    assert_eq!(layout_of(&forwards)?, layout_of(&backwards)?);
    Ok(())
}

#[test]
fn a_turned_page_is_read_as_it_is_shown() -> Result<(), PdfError> {
    // The page is shown turned by a quarter clockwise, and the text is drawn
    // turned by a quarter the other way: it appears upright.
    let content = format!("BT /F1 {FONT_SIZE} Tf 0 1 -1 0 300 100 Tm (Landscape) Tj ET");
    let pages = read(&rotated_pdf(&content, 90))?;
    let layout = PageLayout::of(&pages[0]);

    assert!((pages[0].width - PAGE_HEIGHT).abs() < PLACE_TOLERANCE);
    assert!((pages[0].height - PAGE_WIDTH).abs() < PLACE_TOLERANCE);
    assert_eq!(block_texts(&layout), ["Landscape"]);
    assert_eq!(layout.off_axis_glyphs, 0);
    Ok(())
}

#[test]
fn text_at_an_angle_is_left_out_and_counted() -> Result<(), PdfError> {
    const WATERMARK: &str = "COPY";
    // The matrix turns the text by 45 degrees.
    let content = format!(
        "BT /F1 {FONT_SIZE} Tf 0.7071 0.7071 -0.7071 0.7071 200 300 Tm ({WATERMARK}) Tj ET"
    );
    let layout = layout_of(&content)?;

    assert!(layout.blocks.is_empty());
    assert_eq!(layout.off_axis_glyphs, WATERMARK.len());
    Ok(())
}

#[test]
fn text_drawn_twice_is_read_once() -> Result<(), PdfError> {
    const WORD: &str = "Total";
    /// How far the copy is moved to the right, in points.
    const SHIFT: f64 = 0.3;
    let content = text_at(LEFT_COLUMN, FIRST_BASELINE, WORD)
        + &text_at(LEFT_COLUMN + SHIFT, FIRST_BASELINE, WORD);
    let layout = layout_of(&content)?;

    assert_eq!(block_texts(&layout), [WORD]);
    assert_eq!(layout.copied_glyphs, WORD.len());
    Ok(())
}

#[test]
fn a_page_without_text_has_no_blocks() -> Result<(), PdfError> {
    let pages = read(&pdf("0 0 100 100 re f"))?;

    assert_eq!(pages.len(), 1);
    assert!(pages[0].glyphs.is_empty());
    assert_eq!(pages[0].unreadable_glyphs, 0);
    assert!(PageLayout::of(&pages[0]).blocks.is_empty());
    Ok(())
}

#[test]
fn data_that_is_no_pdf_is_refused() {
    assert_eq!(read(b""), Err(PdfError::Invalid));
    assert_eq!(read(b"not a PDF"), Err(PdfError::Invalid));
}
