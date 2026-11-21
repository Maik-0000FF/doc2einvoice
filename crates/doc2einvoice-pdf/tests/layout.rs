// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The layout on characters that no PDF was read for: the cases a test file
//! with an ordinary font cannot produce.
//!
//! Input table. Each row is one test below.
//!
//! | characters                                          | expected            |
//! |-----------------------------------------------------|---------------------|
//! | none                                                | no blocks           |
//! | white space only                                    | no blocks           |
//! | no advance stated, one letter width apart           | one word            |
//! | no advance stated, several letter widths apart      | two words           |
//! | a row of spaces between two cells                   | two blocks          |
//! | a space right after a letter, the next letter close | two words           |
//! | a space drawn on the letter before it, the next     | one word            |
//! | letter close                                        |                     |
//! | a space drawn on the letter before it, the next     | two words (the gap) |
//! | letter a word gap away                              |                     |
//! | no advance stated: a narrow letter, a space after   | two words           |
//! | it, two letters                                     |                     |
//! | a ligature                                          | its letters, in one |
//! |                                                     | word                |
//! | a letter that is doubled in the word                | both letters        |
//! | a letter and an accent at the same place            | both                |
//! | the same letter three times at nearly one place     | once, two counted   |
//! | a doubled letter in text so condensed that a letter | both letters        |
//! | is narrower than a tenth of the font size           |                     |
//! | a copy in such text, moved by a fifth of the letter | once, one counted   |

use doc2einvoice_pdf::{Glyph, PageLayout};

const FONT_SIZE: f64 = 10.0;
const BASELINE: f64 = 100.0;
/// The width of a letter and of a space in the invented font.
const LETTER_WIDTH: f64 = 5.0;

fn glyph(text: &str, x: f64, advance: f64) -> Glyph {
    Glyph {
        text: text.to_owned(),
        x,
        y: BASELINE,
        advance,
        font_size: FONT_SIZE,
        rotation: 0.0,
    }
}

fn block_texts(glyphs: &[Glyph]) -> Vec<String> {
    PageLayout::of_glyphs(glyphs)
        .blocks
        .iter()
        .map(doc2einvoice_pdf::Block::text)
        .collect()
}

#[test]
fn no_characters_give_no_blocks() {
    assert!(block_texts(&[]).is_empty());
}

#[test]
fn white_space_alone_gives_no_blocks() {
    let glyphs = [
        glyph(" ", 0.0, LETTER_WIDTH),
        glyph("\t", LETTER_WIDTH, LETTER_WIDTH),
    ];

    assert!(block_texts(&glyphs).is_empty());
}

#[test]
fn a_missing_advance_is_assumed() {
    let close = [glyph("A", 0.0, 0.0), glyph("B", LETTER_WIDTH, 0.0)];
    let apart = [glyph("A", 0.0, 0.0), glyph("B", 3.0 * LETTER_WIDTH, 0.0)];

    assert_eq!(block_texts(&close), ["AB"]);
    assert_eq!(block_texts(&apart), ["A B"]);
}

#[test]
fn spaces_do_not_bridge_a_column_gap() {
    let mut glyphs = vec![glyph("A", 0.0, LETTER_WIDTH)];
    let spaces: u8 = 10;
    for index in 1..=spaces {
        glyphs.push(glyph(" ", f64::from(index) * LETTER_WIDTH, LETTER_WIDTH));
    }
    glyphs.push(glyph(
        "B",
        f64::from(spaces + 1) * LETTER_WIDTH,
        LETTER_WIDTH,
    ));

    assert_eq!(block_texts(&glyphs), ["A", "B"]);
}

#[test]
fn a_space_after_a_letter_parts_words() {
    let glyphs = [
        glyph("A", 0.0, LETTER_WIDTH),
        glyph(" ", LETTER_WIDTH, 0.0),
        glyph("B", LETTER_WIDTH, LETTER_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["A B"]);
}

#[test]
fn a_space_drawn_on_the_letter_before_it_parts_nothing() {
    // As one program draws "Nr. 471102": the space on the 4, not before it.
    let on_letter = LETTER_WIDTH / 100.0;
    let glyphs = [
        glyph("4", 0.0, LETTER_WIDTH),
        glyph(" ", on_letter, LETTER_WIDTH),
        glyph("7", LETTER_WIDTH, LETTER_WIDTH),
    ];
    let apart = [
        glyph("4", 0.0, LETTER_WIDTH),
        glyph(" ", on_letter, LETTER_WIDTH),
        glyph("7", 2.0 * LETTER_WIDTH, LETTER_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["47"]);
    assert_eq!(block_texts(&apart), ["4 7"]);
}

#[test]
fn a_space_after_a_narrow_letter_parts_words_without_stated_widths() {
    // The font states no widths, so half the font size is assumed for each
    // letter: more than the "i" takes before the space.
    let narrow = FONT_SIZE / 5.0;
    let glyphs = [
        glyph("i", 0.0, 0.0),
        glyph(" ", narrow, 0.0),
        glyph("a", 2.0 * narrow, 0.0),
        glyph("m", 2.0 * narrow + LETTER_WIDTH, 0.0),
    ];

    assert_eq!(block_texts(&glyphs), ["i am"]);
}

#[test]
fn a_doubled_letter_is_not_a_copy() {
    // The narrowest letters are about a fifth of the font size wide.
    let narrow = FONT_SIZE / 5.0;
    let glyphs = [
        glyph("a", 0.0, LETTER_WIDTH),
        glyph("l", LETTER_WIDTH, narrow),
        glyph("l", LETTER_WIDTH + narrow, narrow),
    ];
    let layout = PageLayout::of_glyphs(&glyphs);

    assert_eq!(block_texts(&glyphs), ["all"]);
    assert_eq!(layout.copied_glyphs, 0);
}

#[test]
fn an_accent_over_a_letter_is_not_a_copy() {
    let glyphs = [glyph("e", 0.0, LETTER_WIDTH), glyph("\u{301}", 0.0, 0.0)];

    assert_eq!(block_texts(&glyphs), ["e\u{301}"]);
    assert_eq!(PageLayout::of_glyphs(&glyphs).copied_glyphs, 0);
}

#[test]
fn three_copies_are_read_once() {
    let shift = FONT_SIZE / 50.0;
    let glyphs = [
        glyph("A", 0.0, LETTER_WIDTH),
        glyph("A", shift, LETTER_WIDTH),
        glyph("A", 2.0 * shift, LETTER_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["A"]);
    assert_eq!(PageLayout::of_glyphs(&glyphs).copied_glyphs, 2);
}

/// A letter in heavily condensed text: narrower than a tenth of the font size.
const CONDENSED_WIDTH: f64 = FONT_SIZE * 0.09;

#[test]
fn a_doubled_letter_in_condensed_text_is_not_a_copy() {
    let glyphs = [
        glyph("l", 0.0, CONDENSED_WIDTH),
        glyph("l", CONDENSED_WIDTH, CONDENSED_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["ll"]);
    assert_eq!(PageLayout::of_glyphs(&glyphs).copied_glyphs, 0);
}

#[test]
fn a_copy_in_condensed_text_is_read_once() {
    let glyphs = [
        glyph("l", 0.0, CONDENSED_WIDTH),
        glyph("l", CONDENSED_WIDTH / 5.0, CONDENSED_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["l"]);
    assert_eq!(PageLayout::of_glyphs(&glyphs).copied_glyphs, 1);
}

#[test]
fn a_ligature_keeps_its_letters() {
    let glyphs = [
        glyph("O", 0.0, LETTER_WIDTH),
        glyph("ffi", LETTER_WIDTH, LETTER_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["Offi"]);
}
