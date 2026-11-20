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
//! | a ligature                                          | its letters, in one |
//! |                                                     | word                |

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
fn a_ligature_keeps_its_letters() {
    let glyphs = [
        glyph("O", 0.0, LETTER_WIDTH),
        glyph("ffi", LETTER_WIDTH, LETTER_WIDTH),
    ];

    assert_eq!(block_texts(&glyphs), ["Offi"]);
}
