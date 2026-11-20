// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Ordering the characters of a page into words, lines and blocks.
//!
//! Only the place, the advance and the font size of each character are used,
//! never the order the file draws them in. Every threshold is a share of the
//! font size, so that it holds for small print and headlines alike.
//!
//! - A **word** ends where the distance to the next character is a word gap,
//!   or where a space is drawn.
//! - A **line** is a run of words on one baseline without a column gap. Two
//!   table cells next to each other are two lines.
//! - A **block** is made of lines that stand below each other: they overlap
//!   horizontally and follow at the distance of a line. A value that is
//!   wrapped inside a table cell thereby stays together, and the cells beside
//!   it stay out of it.

use std::collections::HashMap;

use crate::glyph::{Glyph, Page};

/// Characters whose baselines are at most this far apart stand on one line.
const SAME_LINE_EM: f64 = 0.25;

/// A distance above this starts a new word. A space is about a quarter of the
/// font size wide; kerning and letter-spacing stay below the threshold.
const WORD_GAP_EM: f64 = 0.15;

/// A distance above this starts a new column. The word gaps of justified text
/// stay below it.
const COLUMN_GAP_EM: f64 = 1.0;

/// The largest distance of two baselines that still belong to one paragraph.
const LINE_PITCH_EM: f64 = 1.6;

/// Width assumed for a character whose font states no advance: about that of
/// an average letter.
const ASSUMED_ADVANCE_EM: f64 = 0.5;

/// A baseline that deviates from the horizontal by at most this angle (in
/// radians, about one degree) counts as horizontal.
const HORIZONTAL_TOLERANCE: f64 = 0.02;

const WORD_SEPARATOR: char = ' ';
const LINE_SEPARATOR: char = '\n';

/// A run of characters without a gap.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    /// The characters, without white space.
    pub text: String,
    /// Where the first character starts.
    pub left: f64,
    /// Where the last character ends.
    pub right: f64,
    /// Vertical place of the baseline.
    pub baseline: f64,
    /// Size of the largest font in the word.
    pub font_size: f64,
}

/// Words on one baseline that no column gap separates. Never empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The words from left to right.
    pub words: Vec<Word>,
}

impl Line {
    /// The words, separated by one space each.
    #[must_use]
    pub fn text(&self) -> String {
        joined(
            self.words.iter().map(|word| word.text.as_str()),
            WORD_SEPARATOR,
        )
    }

    fn left(&self) -> f64 {
        self.words.first().map_or(0.0, |word| word.left)
    }

    fn right(&self) -> f64 {
        self.words.last().map_or(0.0, |word| word.right)
    }

    fn baseline(&self) -> f64 {
        self.words.first().map_or(0.0, |word| word.baseline)
    }

    fn font_size(&self) -> f64 {
        self.words
            .iter()
            .map(|word| word.font_size)
            .fold(0.0, f64::max)
    }
}

/// Lines that stand below each other and belong together. Never empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The lines from top to bottom.
    pub lines: Vec<Line>,
}

impl Block {
    /// The lines, separated by one line break each.
    #[must_use]
    pub fn text(&self) -> String {
        let lines: Vec<String> = self.lines.iter().map(Line::text).collect();
        joined(lines.iter().map(String::as_str), LINE_SEPARATOR)
    }
}

/// The text of a page in the order it is read.
#[derive(Debug, Clone, PartialEq)]
pub struct PageLayout {
    /// The blocks, ordered by their first line: from top to bottom, and from
    /// left to right where they start on the same baseline.
    pub blocks: Vec<Block>,
    /// Characters that were left out because their baseline is not
    /// horizontal, such as a watermark written across the page.
    pub off_axis_glyphs: usize,
}

impl PageLayout {
    /// Orders the characters of a page.
    #[must_use]
    pub fn of(page: &Page) -> Self {
        Self::of_glyphs(&page.glyphs)
    }

    /// Orders characters that stand on one page.
    ///
    /// | Characters                                          | Result                     |
    /// |-----------------------------------------------------|----------------------------|
    /// | none, or white space only                           | no blocks                  |
    /// | one following the other on a baseline               | one word                   |
    /// | apart by more than the word gap, or a space between | two words of one line      |
    /// | apart by more than the column gap                   | two lines                  |
    /// | lines below each other at line distance that        | one block                  |
    /// | overlap horizontally                                |                            |
    /// | lines further apart than the line distance          | two blocks                 |
    /// | lines below each other that do not overlap          | two blocks                 |
    /// | given in any order                                  | the same result            |
    /// | a font that states no advance                       | half the font size is      |
    /// |                                                     | assumed as the width       |
    /// | on a baseline that is not horizontal                | left out and counted       |
    #[must_use]
    pub fn of_glyphs(glyphs: &[Glyph]) -> Self {
        let (horizontal, off_axis): (Vec<&Glyph>, Vec<&Glyph>) = glyphs
            .iter()
            .partition(|glyph| glyph.rotation.abs() <= HORIZONTAL_TOLERANCE);
        let lines = lines(horizontal);
        Self {
            blocks: blocks(lines),
            off_axis_glyphs: off_axis.len(),
        }
    }
}

fn joined<'a>(parts: impl Iterator<Item = &'a str>, separator: char) -> String {
    let mut text = String::new();
    for (index, part) in parts.enumerate() {
        if index > 0 {
            text.push(separator);
        }
        text.push_str(part);
    }
    text
}

/// Groups characters by baseline, from top to bottom, each group from left to
/// right.
fn rows(mut glyphs: Vec<&Glyph>) -> Vec<Vec<&Glyph>> {
    glyphs.sort_by(|a, b| a.y.total_cmp(&b.y));
    let mut rows: Vec<Vec<&Glyph>> = Vec::new();
    for glyph in glyphs {
        match rows.last_mut() {
            Some(row) if (glyph.y - row[0].y).abs() <= SAME_LINE_EM * row[0].font_size => {
                row.push(glyph);
            }
            _ => rows.push(vec![glyph]),
        }
    }
    for row in &mut rows {
        row.sort_by(|a, b| a.x.total_cmp(&b.x));
    }
    rows
}

/// A line that is still being read from left to right.
struct OpenLine {
    words: Vec<Word>,
    word: Option<Word>,
    /// Where the last character that is no white space ends.
    right: f64,
}

impl OpenLine {
    fn close_word(&mut self) {
        self.words.extend(self.word.take());
    }

    fn into_line(mut self) -> Option<Line> {
        self.close_word();
        (!self.words.is_empty()).then_some(Line { words: self.words })
    }
}

/// Cuts every row into lines and their words.
fn lines(glyphs: Vec<&Glyph>) -> Vec<Line> {
    let mut lines = Vec::new();
    for row in rows(glyphs) {
        let mut open: Option<OpenLine> = None;
        for glyph in row {
            let gap = open.as_ref().map(|line| glyph.x - line.right);
            if gap.is_some_and(|gap| gap > COLUMN_GAP_EM * glyph.font_size) {
                lines.extend(open.take().and_then(OpenLine::into_line));
            }
            // A space ends a word but neither starts a line nor widens one:
            // a row of spaces would otherwise bridge any column gap.
            let visible: String = glyph.text.chars().filter(|c| !c.is_whitespace()).collect();
            if visible.is_empty() {
                if let Some(line) = open.as_mut() {
                    line.close_word();
                }
                continue;
            }
            let line = open.get_or_insert_with(|| OpenLine {
                words: Vec::new(),
                word: None,
                right: glyph.x,
            });
            if gap.is_some_and(|gap| gap > WORD_GAP_EM * glyph.font_size) {
                line.close_word();
            }
            let advance = if glyph.advance > 0.0 {
                glyph.advance
            } else {
                ASSUMED_ADVANCE_EM * glyph.font_size
            };
            let word = line.word.get_or_insert_with(|| Word {
                text: String::new(),
                left: glyph.x,
                right: glyph.x,
                baseline: glyph.y,
                font_size: glyph.font_size,
            });
            word.text.push_str(&visible);
            word.right = word.right.max(glyph.x + advance);
            word.font_size = word.font_size.max(glyph.font_size);
            line.right = line.right.max(word.right);
        }
        lines.extend(open.and_then(OpenLine::into_line));
    }
    lines
}

/// The representative of the set a line belongs to, in a forest of parents.
fn root(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }
    index
}

/// Joins the lines that stand below each other into blocks. The lines come
/// from top to bottom and leave in the same order.
fn blocks(lines: Vec<Line>) -> Vec<Block> {
    let largest_font = lines.iter().map(Line::font_size).fold(0.0, f64::max);
    let mut parents: Vec<usize> = (0..lines.len()).collect();
    for (lower_index, lower) in lines.iter().enumerate() {
        for upper_index in (0..lower_index).rev() {
            let upper = &lines[upper_index];
            let drop = lower.baseline() - upper.baseline();
            // Lines come ordered by baseline: none further up can be near.
            if drop > LINE_PITCH_EM * largest_font {
                break;
            }
            let stacked = drop > SAME_LINE_EM * upper.font_size()
                && drop <= LINE_PITCH_EM * upper.font_size().max(lower.font_size());
            let overlapping = upper.left() < lower.right() && lower.left() < upper.right();
            if stacked && overlapping {
                let upper_root = root(&mut parents, upper_index);
                let lower_root = root(&mut parents, lower_index);
                parents[lower_root] = upper_root;
            }
        }
    }

    let mut blocks: Vec<Block> = Vec::new();
    let mut block_of_root: HashMap<usize, usize> = HashMap::new();
    for (index, line) in lines.into_iter().enumerate() {
        let line_root = root(&mut parents, index);
        let block_index = *block_of_root.entry(line_root).or_insert_with(|| {
            blocks.push(Block { lines: Vec::new() });
            blocks.len() - 1
        });
        blocks[block_index].lines.push(line);
    }
    blocks
}
