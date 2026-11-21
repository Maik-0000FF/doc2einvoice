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

impl<'a> Line<'a> {
    pub fn texts(&self) -> Vec<&'a str> {
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

/// The words of the line after `line` if it continues the same block below
/// it: where a sentence that ends with a label wraps, its value starts that
/// line. A line of the block on the same baseline is a cell beside it.
pub fn wrapped<'a>(lines: &[Line<'a>], line: usize) -> Option<Span<'a>> {
    let (this, next) = (lines.get(line)?, lines.get(line + 1)?);
    let (label, start) = (this.words.first()?, next.words.first()?);
    let continues = next.page == this.page && next.block == this.block && lies_below(start, label);
    continues.then(|| Span {
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

/// The cell right of the label ending with `last`: the line that stands
/// nearest right of the label on its baseline, and the lines of its block
/// directly above it that nothing stands left of, from top to bottom.
///
/// A name set over two lines beside its label, which stands on the last of
/// them, is read whole; the street and town below a name, and the lines
/// beside other labels, are not.
///
/// | Left       | Right of it                  | Cell of "Name:"                  |
/// |------------|------------------------------|----------------------------------|
/// |            | `MVM Musterhafter`           |                                  |
/// | `Name:`    | `Versicherungsverein a.G.`   | both lines                       |
/// | `Name:`    | `Muster GmbH`                | `Muster GmbH`                    |
/// | `Straße:`  | `Hauptstr. 1`                |                                  |
/// | `Nummer:`  | `549910`                     |                                  |
/// | `Name:`    | `Muster GmbH`                | `Muster GmbH` (a label left of the line above) |
/// | `Name:`    | `Muster GmbH`                | `Muster GmbH` (the address below is left out) |
/// |            | `Hauptstr. 1`                |                                  |
pub fn cell_right_of<'a>(lines: &[Line<'a>], line: usize, last: &Word) -> Span<'a> {
    let page = lines[line].page;
    let tolerance = last.font_size * SAME_BASELINE;
    let nearest = lines
        .iter()
        .enumerate()
        .filter(|(index, other)| *index != line && other.page == page)
        .filter_map(|(index, other)| {
            let head = other.words.first()?;
            let beside =
                (head.baseline - last.baseline).abs() <= tolerance && head.left >= last.right;
            beside.then_some((index, head.left))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index);
    let Some(start) = nearest else {
        return Span {
            page,
            words: Vec::new(),
        };
    };
    let block = lines[start].block;
    let mut first = start;
    while first > 0 {
        let above = first - 1;
        let same_block = lines[above].page == page && lines[above].block == block;
        if !same_block || has_text_left(lines, above) {
            break;
        }
        first = above;
    }
    Span {
        page,
        words: lines[first..=start]
            .iter()
            .flat_map(|cell| cell.words.iter())
            .collect(),
    }
}

/// The words of other lines of the page of `line` that stand on its
/// baseline.
fn on_its_baseline<'l, 'a>(
    lines: &'l [Line<'a>],
    line: usize,
) -> impl Iterator<Item = &'a Word> + 'l {
    let head = lines[line].words.first();
    let page = lines[line].page;
    lines
        .iter()
        .enumerate()
        .filter(move |(index, other)| *index != line && other.page == page)
        .flat_map(|(_, other)| other.words.iter())
        .filter(move |word| {
            head.is_some_and(|head| same_baseline(word.baseline, head.baseline, head.font_size))
        })
}

/// Whether anything is printed left of `line` on its baseline, as a label.
fn has_text_left(lines: &[Line], line: usize) -> bool {
    let Some(head) = lines[line].words.first() else {
        return false;
    };
    on_its_baseline(lines, line).any(|word| word.right <= head.left)
}

/// The texts printed beside `line` on its baseline, left or right, from
/// left to right.
pub fn text_beside<'a>(lines: &[Line<'a>], line: usize) -> Vec<&'a str> {
    let mut words: Vec<&Word> = on_its_baseline(lines, line).collect();
    words.sort_by(|a, b| a.left.total_cmp(&b.left));
    words.iter().map(|word| word.text.as_str()).collect()
}

/// Whether two baselines are one: no further apart than the tolerance of
/// one baseline at `font_size`.
pub fn same_baseline(a: f64, b: f64, font_size: f64) -> bool {
    (a - b).abs() <= font_size * SAME_BASELINE
}

/// Whether `word` stands on a baseline below that of `above`, beyond the
/// tolerance of one baseline.
pub fn lies_below(word: &Word, above: &Word) -> bool {
    word.baseline > above.baseline + above.font_size * SAME_BASELINE
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
            lies_below(head, first)
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

#[cfg(test)]
mod tests {
    use doc2einvoice_pdf::{Block, Line as Printed, PageLayout, Word};

    use super::{cell_right_of, lines, wrapped};

    /// The font size of every word, and the distance of two baselines.
    const FONT: f64 = 10.0;
    const PITCH: f64 = 12.0;

    fn line(left: f64, baseline: f64, text: &str) -> Printed {
        Printed {
            words: vec![Word {
                text: text.to_owned(),
                left,
                right: left + FONT,
                baseline,
                font_size: FONT,
            }],
        }
    }

    fn texts(pages: &[PageLayout], at: usize) -> Option<Vec<String>> {
        let all = lines(pages);
        wrapped(&all, at).map(|span| span.texts().iter().map(|text| (*text).to_owned()).collect())
    }

    #[test]
    fn the_next_line_of_a_block_continues_a_sentence_only_below_it() {
        let below = [PageLayout {
            blocks: vec![Block {
                lines: vec![
                    line(50.0, 100.0, "vom"),
                    line(50.0, 100.0 + PITCH, "05.03.2018"),
                ],
            }],
            off_axis_glyphs: 0,
            copied_glyphs: 0,
        }];
        assert_eq!(texts(&below, 0), Some(vec!["05.03.2018".to_owned()]));
        let beside = [PageLayout {
            blocks: vec![Block {
                lines: vec![line(50.0, 100.0, "vom"), line(300.0, 100.0, "05.03.2018")],
            }],
            off_axis_glyphs: 0,
            copied_glyphs: 0,
        }];
        assert_eq!(texts(&beside, 0), None);
    }

    /// One block of lines at `left`, each on its baseline.
    fn column(left: f64, rows: &[(f64, &str)]) -> Block {
        Block {
            lines: rows
                .iter()
                .map(|(baseline, text)| line(left, *baseline, text))
                .collect(),
        }
    }

    fn cell_texts(pages: &[PageLayout], label_line: usize) -> Vec<String> {
        let all = lines(pages);
        let label = all[label_line].words.last().cloned();
        label.map_or_else(Vec::new, |last| {
            cell_right_of(&all, label_line, &last)
                .texts()
                .iter()
                .map(|text| (*text).to_owned())
                .collect()
        })
    }

    fn layout(blocks: Vec<Block>) -> [PageLayout; 1] {
        [PageLayout {
            blocks,
            off_axis_glyphs: 0,
            copied_glyphs: 0,
        }]
    }

    #[test]
    fn cells_follow_their_table() {
        let two_lines = layout(vec![
            column(50.0, &[(100.0 + PITCH, "Name:")]),
            column(300.0, &[(100.0, "MVM"), (100.0 + PITCH, "Verein")]),
        ]);
        assert_eq!(cell_texts(&two_lines, 0), ["MVM", "Verein"]);
        let form = layout(vec![
            column(
                50.0,
                &[
                    (100.0, "Nummer:"),
                    (100.0 + PITCH, "Name:"),
                    (100.0 + 2.0 * PITCH, "Straße:"),
                ],
            ),
            column(
                300.0,
                &[
                    (100.0, "549910"),
                    (100.0 + PITCH, "Muster"),
                    (100.0 + 2.0 * PITCH, "Hauptstr."),
                ],
            ),
        ]);
        assert_eq!(cell_texts(&form, 1), ["Muster"]);
        let address = layout(vec![
            column(50.0, &[(100.0, "Name:")]),
            column(300.0, &[(100.0, "Muster"), (100.0 + PITCH, "Hauptstr.")]),
        ]);
        assert_eq!(cell_texts(&address, 0), ["Muster"]);
    }
}
