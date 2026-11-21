// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The lines of an invoice in reading order, each with its page, and the
//! places a value may stand at next to a label: after it in its line, to
//! its right on the same baseline, at the start of the next line of its
//! block where a sentence wraps, or in the line below it.

use doc2einvoice_model::Location;
use doc2einvoice_pdf::{PageLayout, Word};

/// How far two baselines may lie apart and still be one, as a share of the
/// font size: what a value set in another block beside its label is off by.
const SAME_BASELINE: f64 = 0.3;

/// How far below a label the line with its value may start, as a multiple
/// of the font size of the label.
const BELOW_WITHIN: f64 = 2.5;

/// How far left of a label the value below it may start, as a multiple of
/// the font size: a value set slightly out of line with its label.
const BELOW_SLACK: f64 = 1.0;

/// One printed line of an invoice.
pub struct Line<'a> {
    pub page: usize,
    /// The block of the page the line stands in.
    pub block: usize,
    pub words: &'a [Word],
}

impl Line<'_> {
    pub fn texts(&self) -> Vec<&str> {
        texts(self.words)
    }
}

/// The texts of `words`, in their order.
fn texts<'w>(words: impl IntoIterator<Item = &'w Word>) -> Vec<&'w str> {
    words.into_iter().map(|word| word.text.as_str()).collect()
}

/// Words that stand together, on one page.
pub struct Span<'a> {
    pub page: usize,
    pub words: Vec<&'a Word>,
}

impl Span<'_> {
    pub fn texts(&self) -> Vec<&str> {
        texts(self.words.iter().copied())
    }

    /// The place of the words from `first` on, `count` of them.
    pub fn location(&self, first: usize, count: usize) -> Option<Location> {
        let words = self.words.get(first..first + count)?;
        let (head, tail) = (words.first()?, words.last()?);
        let height = words.iter().map(|word| word.font_size).fold(0.0, f64::max);
        let baseline = words
            .iter()
            .map(|word| word.baseline)
            .fold(f64::MIN, f64::max);
        Some(Location {
            page: self.page,
            left: head.left,
            top: baseline - height,
            right: tail.right,
            bottom: baseline,
        })
    }
}

/// Every line of `pages`, in reading order.
pub fn lines(pages: &[PageLayout]) -> Vec<Line<'_>> {
    pages
        .iter()
        .enumerate()
        .flat_map(|(page, layout)| {
            layout
                .blocks
                .iter()
                .enumerate()
                .flat_map(move |(block, lines)| {
                    lines.lines.iter().map(move |line| Line {
                        page,
                        block,
                        words: &line.words,
                    })
                })
        })
        .collect()
}

/// The words of the line after `line` if it continues the same block: where
/// a sentence that ends with a label wraps, its value starts that line.
pub fn wrapped<'a>(lines: &[Line<'a>], line: usize) -> Option<Span<'a>> {
    let (this, next) = (lines.get(line)?, lines.get(line + 1)?);
    (next.page == this.page && next.block == this.block).then(|| Span {
        page: next.page,
        words: next.words.iter().collect(),
    })
}

/// The words after a label in its line, up to `end`.
pub fn after<'a>(line: &Line<'a>, from: usize, end: usize) -> Span<'a> {
    Span {
        page: line.page,
        words: line.words[from..end].iter().collect(),
    }
}

/// The words of other lines on the same page that stand right of the label
/// ending with `last` on its baseline, from left to right.
pub fn right_of<'a>(lines: &[Line<'a>], line: usize, last: &Word) -> Span<'a> {
    let page = lines[line].page;
    let tolerance = last.font_size * SAME_BASELINE;
    let mut words: Vec<&Word> = lines
        .iter()
        .enumerate()
        .filter(|(index, other)| *index != line && other.page == page)
        .flat_map(|(_, other)| other.words.iter())
        .filter(|word| {
            (word.baseline - last.baseline).abs() <= tolerance && word.left >= last.right
        })
        .collect();
    words.sort_by(|a, b| a.left.total_cmp(&b.left));
    Span { page, words }
}

/// The words of the nearest line below the label from `first` to `last`
/// that starts under it.
pub fn below<'a>(lines: &[Line<'a>], line: usize, first: &Word, last: &Word) -> Span<'a> {
    let page = lines[line].page;
    let slack = first.font_size * BELOW_SLACK;
    let reach = first.baseline + first.font_size * BELOW_WITHIN;
    let nearest = lines
        .iter()
        .filter(|other| other.page == page)
        .filter_map(|other| other.words.first().map(|head| (other, head)))
        .filter(|(_, head)| {
            head.baseline > first.baseline + first.font_size * SAME_BASELINE
                && head.baseline <= reach
                && head.left >= first.left - slack
                && head.left <= last.right
        })
        .min_by(|(_, a), (_, b)| a.baseline.total_cmp(&b.baseline));
    Span {
        page,
        words: nearest.map_or_else(Vec::new, |(other, _)| other.words.iter().collect()),
    }
}
