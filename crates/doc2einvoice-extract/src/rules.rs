// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The rules that read a business term next to its labels.
//!
//! A value is looked for after its label in the same line, then to its
//! right on the same baseline. For the terms of the header, it is then
//! looked for at the start of the next line of the block, if the label
//! closes a line with text before it as a sentence that wraps, and then in
//! the line below. A value beside its label counts more than one below it:
//! below a label stand the values of a column too, which is why a total is
//! never looked for there, nor where a table heading ends with its label.
//!
//! A label that names its term counts more than a general one: "Datum" is
//! the issue date only where no "Rechnungsdatum" gives one. Labels that do
//! not count where they stand, as "vom" away from the invoice number, are
//! ordinary words and do not end the value of the label before them.
//!
//! A label in the line of the invoice number, as "vom" in "Rechnung Nr. 17
//! vom 16.09.2026", counts more than a label elsewhere. For the header, the
//! words after a label in its line are its value or nothing: only a label
//! with nothing after it looks further.
//!
//! Of several values found, the header takes the first, as the invoice
//! number is printed before any reference to it; a total takes the last, as
//! the totals of an invoice close it.

use doc2einvoice_model::{Draft, Entry, Origin};
use doc2einvoice_pdf::PageLayout;

use crate::labels::{
    self, AMOUNT_DUE, CURRENCY, Found, INVOICE_NUMBER, ISSUE_DATE, Scope, TOTAL_WITH_VAT,
    TOTAL_WITHOUT_VAT, VAT_TOTAL, Weight,
};
use crate::page::{self, Line, Span};
use crate::values::{self, Read};

/// Which of several values found counts.
#[derive(Clone, Copy)]
enum Choice {
    First,
    Last,
}

/// A word that only separates a label from its value, as the colon in
/// "Währung | : EUR".
const SEPARATING_WORD: &str = ":";

/// How a business term is read.
struct Rule {
    term: u16,
    read: fn(&[&str]) -> Option<Read>,
    /// Whether the value may stand below its label.
    below: bool,
    /// Whether the value may start the next line where the sentence that
    /// ends with its label wraps.
    wraps: bool,
    /// Whether the value is looked for beside the label in another block
    /// when the words after the label in its line are not one.
    past_words: bool,
    choice: Choice,
}

/// The terms read, the invoice number first: the issue date may follow it.
const RULES: [Rule; 7] = [
    header(INVOICE_NUMBER, values::identifier),
    header(ISSUE_DATE, values::date),
    header(CURRENCY, values::currency),
    total(TOTAL_WITHOUT_VAT),
    total(VAT_TOTAL),
    total(TOTAL_WITH_VAT),
    total(AMOUNT_DUE),
];

const fn header(term: u16, read: fn(&[&str]) -> Option<Read>) -> Rule {
    Rule {
        term,
        read,
        below: true,
        wraps: true,
        past_words: false,
        choice: Choice::First,
    }
}

const fn total(term: u16) -> Rule {
    Rule {
        term,
        read: values::monetary,
        below: false,
        wraps: false,
        past_words: true,
        choice: Choice::Last,
    }
}

/// Where the label of a value stands: in the line of the invoice number,
/// or elsewhere.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Standing {
    InNumberLine,
    Elsewhere,
}

/// How near a value stands to its label: beside it, at the start of the
/// next line where its sentence wraps, or below it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Nearness {
    Beside,
    Wrapped,
    Below,
}

/// A value found for a term.
struct Candidate {
    standing: Standing,
    nearness: Nearness,
    /// The line of the label.
    line: usize,
    /// The word after the value, if it stands in the line of its label.
    end_in_line: Option<usize>,
    entry: Entry,
}

/// The labels of one line.
struct Labelled<'a> {
    line: usize,
    found: Vec<Found>,
    words: Vec<&'a str>,
}

/// The values the rules find on `pages`.
pub fn draft(pages: &[PageLayout]) -> Draft {
    let lines = page::lines(pages);
    let labelled: Vec<Labelled> = lines
        .iter()
        .enumerate()
        .map(|(line, printed)| {
            let words = printed.texts();
            Labelled {
                line,
                found: labels::find(&words),
                words,
            }
        })
        .collect();
    let mut draft = Draft::default();
    let mut number: Option<(usize, usize)> = None;
    for rule in &RULES {
        let chosen = chosen(rule, &lines, &labelled, number);
        if let Some(candidate) = chosen {
            if rule.term == INVOICE_NUMBER {
                number = candidate.end_in_line.map(|end| (candidate.line, end));
            }
            draft.entries.push(candidate.entry);
        } else if rule.term == CURRENCY
            && let Some(entry) = marked_currency(&lines)
        {
            draft.entries.push(entry);
        }
    }
    draft
}

/// The value that counts for `rule`, if one is found. `number` is the line
/// of the invoice number and the word after it.
fn chosen(
    rule: &Rule,
    lines: &[Line],
    labelled: &[Labelled],
    number: Option<(usize, usize)>,
) -> Option<Candidate> {
    let mut candidates: Vec<(Weight, Candidate)> = Vec::new();
    for line in labelled {
        let valid: Vec<&Found> = line
            .found
            .iter()
            .filter(|found| in_scope(found, line.line, number))
            .collect();
        for (index, found) in valid.iter().enumerate() {
            if found.label.term != rule.term {
                continue;
            }
            let end = valid
                .get(index + 1)
                .map_or(line.words.len(), |next| next.first);
            if let Some(mut candidate) = candidate(rule, lines, line.line, found, end) {
                if found.label.scope == Scope::AfterNumber {
                    candidate.standing = Standing::InNumberLine;
                }
                candidates.push((found.label.weight, candidate));
            }
        }
    }
    let rank = |(weight, candidate): &(Weight, Candidate)| {
        (candidate.standing, *weight, candidate.nearness)
    };
    let best = candidates.iter().map(rank).min()?;
    let mut nearest = candidates
        .into_iter()
        .filter(|candidate| rank(candidate) == best)
        .map(|(_, candidate)| candidate);
    match rule.choice {
        Choice::First => nearest.next(),
        Choice::Last => nearest.next_back(),
    }
}

/// Whether a label found in `line` counts where it stands.
fn in_scope(found: &Found, line: usize, number: Option<(usize, usize)>) -> bool {
    match found.label.scope {
        Scope::Anywhere | Scope::LineStart => true,
        Scope::AfterNumber => {
            number.is_some_and(|(number_line, after)| number_line == line && found.first >= after)
        }
    }
}

/// The value next to one label, if `rule` reads one there.
fn candidate(
    rule: &Rule,
    lines: &[Line],
    line: usize,
    found: &Found,
    end: usize,
) -> Option<Candidate> {
    let printed = &lines[line];
    let first = printed.words.get(found.first)?;
    let last = printed.words.get(found.end.checked_sub(1)?)?;
    let after = page::after(printed, found.end, end);
    if let Some(read) = read_after_separators(rule, &after) {
        let end_in_line = found.end + read.skipped + read.used;
        return entry(rule.term, &after, &read).map(|entry| Candidate {
            standing: Standing::Elsewhere,
            nearness: Nearness::Beside,
            line,
            end_in_line: Some(end_in_line),
            entry,
        });
    }
    let only_separators = after.texts().iter().all(|text| *text == SEPARATING_WORD);
    if !only_separators && !rule.past_words {
        return None;
    }
    // Only a label that closes a line with text before it ends a sentence
    // that may wrap: a label alone in its line heads a column.
    let closes_sentence = rule.wraps && found.first > 0 && end == printed.words.len();
    let places = [
        (Nearness::Beside, Some(page::right_of(lines, line, last))),
        (
            Nearness::Wrapped,
            closes_sentence
                .then(|| page::wrapped(lines, line))
                .flatten(),
        ),
        (
            Nearness::Below,
            rule.below.then(|| page::below(lines, line, first, last)),
        ),
    ];
    places.into_iter().find_map(|(nearness, span)| {
        let span = span?;
        let read = read_after_separators(rule, &span)?;
        entry(rule.term, &span, &read).map(|entry| Candidate {
            standing: Standing::Elsewhere,
            nearness,
            line,
            end_in_line: None,
            entry,
        })
    })
}

/// The value `rule` reads from `span`, after any words that only separate
/// it from its label.
fn read_after_separators(rule: &Rule, span: &Span) -> Option<Read> {
    let texts = span.texts();
    let separators = texts
        .iter()
        .take_while(|text| **text == SEPARATING_WORD)
        .count();
    let mut read = (rule.read)(&texts[separators..])?;
    read.skipped += separators;
    Some(read)
}

fn entry(term: u16, span: &Span, read: &Read) -> Option<Entry> {
    Some(Entry {
        term,
        within: Vec::new(),
        text: read.text.clone(),
        origin: Origin::Rule,
        location: Some(span.location(read.skipped, read.used)?),
    })
}

/// The currency the amounts of the invoice are marked with, if they are all
/// marked with the same one: a mark directly before or after an amount with
/// decimals, or a symbol joined to it.
///
/// | Printed                                   | Currency |
/// |-------------------------------------------|----------|
/// | `571,04` `€` … `75,04` `€`                | `EUR`    |
/// | `EUR` `1.234,50`                          | `EUR`    |
/// | `€571.04`                                 | `EUR`    |
/// | `571,04` `€` … `CHF` `12,00`              | none (two currencies) |
/// | `HRB` `372876` (no decimals)              | none     |
/// | `EUR` alone                               | none     |
fn marked_currency(lines: &[Line]) -> Option<Entry> {
    let mut seen: Vec<(String, Span)> = Vec::new();
    for line in lines {
        let texts = line.texts();
        for (index, word) in texts.iter().enumerate() {
            let beside = |offset: isize| {
                index
                    .checked_add_signed(offset)
                    .and_then(|other| texts.get(other))
                    .is_some_and(|other| values::has_decimals(other))
            };
            let code = values::currency_mark(word)
                .filter(|_| beside(-1) || beside(1))
                .or_else(|| values::joined_symbol(word));
            if let Some(code) = code {
                let span = Span {
                    page: line.page,
                    words: vec![&line.words[index]],
                };
                seen.push((code, span));
            }
        }
    }
    let (code, span) = seen.first()?;
    if seen.iter().any(|(other, _)| other != code) {
        return None;
    }
    Some(Entry {
        term: CURRENCY,
        within: Vec::new(),
        text: code.clone(),
        origin: Origin::Rule,
        location: span.location(0, 1),
    })
}

#[cfg(test)]
mod tests {
    use doc2einvoice_pdf::{Block, Line, PageLayout, Word};

    use super::draft;
    use crate::labels::{
        AMOUNT_DUE, CURRENCY, INVOICE_NUMBER, ISSUE_DATE, TOTAL_WITH_VAT, TOTAL_WITHOUT_VAT,
        VAT_TOTAL,
    };

    /// The font size of every word, and the width of one character.
    const FONT: f64 = 10.0;
    const CHARACTER: f64 = 5.0;
    /// The gap between two words of a line.
    const GAP: f64 = 3.0;
    /// The distance of two baselines in a block.
    const LINE_PITCH: f64 = 12.0;

    /// A line of `texts` from `left` on `baseline`, as one block.
    fn block(left: f64, baseline: f64, texts: &[&str]) -> Block {
        let mut words = Vec::new();
        let mut at = left;
        for text in texts {
            let right = text.chars().fold(at, |end, _| end + CHARACTER);
            words.push(Word {
                text: (*text).to_owned(),
                left: at,
                right,
                baseline,
                font_size: FONT,
            });
            at = right + GAP;
        }
        Block {
            lines: vec![Line { words }],
        }
    }

    fn page(blocks: Vec<Block>) -> PageLayout {
        PageLayout {
            blocks,
            off_axis_glyphs: 0,
            copied_glyphs: 0,
        }
    }

    fn value(pages: &[PageLayout], term: u16) -> Option<String> {
        let found = draft(pages);
        let mut values = found.values_of(term).map(|entry| entry.text.clone());
        let first = values.next();
        assert!(values.next().is_none(), "one value for BT-{term}");
        first
    }

    #[test]
    fn a_value_after_its_label_is_read_with_its_place() {
        let pages = [page(vec![block(
            50.0,
            100.0,
            &["Rechnungsnummer:", "RE-17"],
        )])];
        assert_eq!(value(&pages, INVOICE_NUMBER).as_deref(), Some("RE-17"));
        let found = draft(&pages);
        let location = found
            .values_of(INVOICE_NUMBER)
            .next()
            .and_then(|entry| entry.location);
        let place = location.map(|at| (at.page, at.top, at.bottom));
        assert_eq!(place, Some((0, 100.0 - FONT, 100.0)));
    }

    #[test]
    fn a_value_in_another_block_on_the_baseline_is_read() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Währung:"]),
            block(300.0, 100.5, &["EUR"]),
            block(50.0, 120.0, &["Rechnungssumme", "ohne", "USt."]),
            block(400.0, 120.0, &["1.234,50"]),
        ])];
        assert_eq!(value(&pages, CURRENCY).as_deref(), Some("EUR"));
        assert_eq!(value(&pages, TOTAL_WITHOUT_VAT).as_deref(), Some("1234.5"));
    }

    /// Lines below each other as one block, each from `left`, the first on
    /// `top`, the others a line pitch below the one before.
    fn wrapped_block(left: f64, top: f64, rows: &[&[&str]]) -> Block {
        let mut lines = Vec::new();
        let mut baseline = top;
        for texts in rows {
            lines.extend(block(left, baseline, texts).lines);
            baseline += LINE_PITCH;
        }
        Block { lines }
    }

    #[test]
    fn a_value_after_a_label_that_closes_a_wrapped_line_is_read() {
        let pages = [page(vec![wrapped_block(
            50.0,
            100.0,
            &[
                &[
                    "Gutschrift",
                    "(Selbst",
                    "ausgestellte",
                    "Rechnung)",
                    "Nr.",
                    "471102",
                    "vom",
                ],
                &["05.03.2018"],
            ],
        )])];
        assert_eq!(value(&pages, INVOICE_NUMBER).as_deref(), Some("471102"));
        assert_eq!(value(&pages, ISSUE_DATE).as_deref(), Some("2018-03-05"));
    }

    #[test]
    fn a_table_heading_that_ends_with_a_total_label_does_not_wrap() {
        let pages = [page(vec![wrapped_block(
            50.0,
            100.0,
            &[&["Pos", "Menge", "Gesamtbetrag"], &["1", "10", "5,00"]],
        )])];
        assert_eq!(value(&pages, TOTAL_WITH_VAT), None);
    }

    #[test]
    fn a_line_of_another_block_does_not_continue_a_sentence() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Rechnung", "Nr.", "17", "vom"]),
            block(50.0, 100.0 + LINE_PITCH, &["05.03.2018"]),
        ])];
        assert_eq!(value(&pages, INVOICE_NUMBER).as_deref(), Some("17"));
        assert_eq!(value(&pages, ISSUE_DATE), None);
    }

    #[test]
    fn a_label_alone_in_its_line_does_not_wrap() {
        let pages = [page(vec![wrapped_block(
            300.0,
            100.0,
            &[&["Gesamtbetrag"], &["160,00"]],
        )])];
        assert_eq!(value(&pages, TOTAL_WITH_VAT), None);
    }

    #[test]
    fn a_colon_standing_alone_is_passed_over() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Währung"]),
            block(300.0, 100.0, &[":", "EUR"]),
            block(50.0, 120.0, &["Rechnungsnummer", ":", "RE-17"]),
        ])];
        assert_eq!(value(&pages, CURRENCY).as_deref(), Some("EUR"));
        assert_eq!(value(&pages, INVOICE_NUMBER).as_deref(), Some("RE-17"));
    }

    #[test]
    fn a_header_value_may_stand_below_its_label_but_a_total_not() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Rechnungsdatum"]),
            block(52.0, 112.0, &["16.09.2026"]),
            block(300.0, 100.0, &["Steuerbetrag"]),
            block(300.0, 112.0, &["11,20"]),
        ])];
        assert_eq!(value(&pages, ISSUE_DATE).as_deref(), Some("2026-09-16"));
        assert_eq!(value(&pages, VAT_TOTAL), None);
    }

    #[test]
    fn the_date_in_the_line_of_the_number_counts_first() {
        let pages = [page(vec![
            block(50.0, 50.0, &["Version", "2.7", "vom", "06.05.2026"]),
            block(50.0, 80.0, &["Datum:", "01.01.2026"]),
            block(
                50.0,
                100.0,
                &[
                    "Handelsrechnung",
                    "(380)",
                    "Nr.",
                    "471102",
                    "vom",
                    "16.09.2026",
                ],
            ),
        ])];
        assert_eq!(value(&pages, INVOICE_NUMBER).as_deref(), Some("471102"));
        assert_eq!(value(&pages, ISSUE_DATE).as_deref(), Some("2026-09-16"));
    }

    #[test]
    fn a_header_label_with_other_words_after_it_does_not_look_further() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Rechnung", "gemäß", "Bestellung"]),
            block(300.0, 100.0, &["4.284,00"]),
        ])];
        assert_eq!(value(&pages, INVOICE_NUMBER), None);
    }

    #[test]
    fn the_last_total_counts_and_longer_labels_keep_their_words() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Gesamtbetrag", "der", "Zuschläge:", "5,00"]),
            block(50.0, 120.0, &["Gesamtbetrag:", "100,00"]),
            block(50.0, 140.0, &["Gesamtbetrag:", "571,04", "€"]),
            block(50.0, 160.0, &["Zahlbetrag", "EUR", "571,04"]),
        ])];
        assert_eq!(value(&pages, TOTAL_WITH_VAT).as_deref(), Some("571.04"));
        assert_eq!(value(&pages, AMOUNT_DUE).as_deref(), Some("571.04"));
    }

    #[test]
    fn a_label_that_does_not_count_where_it_stands_is_an_ordinary_word() {
        let pages = [page(vec![block(
            50.0,
            100.0,
            &["Facture", "du", "16/11/2017", "N°", "FA-2017-1"],
        )])];
        assert_eq!(value(&pages, INVOICE_NUMBER).as_deref(), Some("FA-2017-1"));
    }

    #[test]
    fn a_number_named_for_another_term_is_not_the_invoice_number() {
        let pages = [page(vec![block(
            50.0,
            100.0,
            &["Rechnung", "Kunden", "Nr.", "4711"],
        )])];
        assert_eq!(value(&pages, INVOICE_NUMBER), None);
    }

    #[test]
    fn a_specific_date_label_counts_before_a_general_one() {
        let ordered = [page(vec![
            block(50.0, 100.0, &["Order", "Date:", "01.01.2026"]),
            block(50.0, 120.0, &["Invoice", "Date:", "02.01.2026"]),
        ])];
        assert_eq!(value(&ordered, ISSUE_DATE).as_deref(), Some("2026-01-02"));
        let general_first = [page(vec![
            block(50.0, 100.0, &["Datum:", "01.01.2026"]),
            block(50.0, 120.0, &["Rechnungsdatum:", "02.01.2026"]),
        ])];
        assert_eq!(
            value(&general_first, ISSUE_DATE).as_deref(),
            Some("2026-01-02")
        );
        let only_general = [page(vec![block(50.0, 100.0, &["Datum:", "01.01.2026"])])];
        assert_eq!(
            value(&only_general, ISSUE_DATE).as_deref(),
            Some("2026-01-01")
        );
    }

    #[test]
    fn a_total_has_at_most_two_decimal_places() {
        let pages = [page(vec![block(
            50.0,
            100.0,
            &["Gesamtbetrag:", "1234,567"],
        )])];
        assert_eq!(value(&pages, TOTAL_WITH_VAT), None);
    }

    #[test]
    fn a_rate_before_a_total_is_passed_over() {
        let pages = [page(vec![block(
            50.0,
            100.0,
            &["Steuerbetrag", "19", "%", "11,20"],
        )])];
        assert_eq!(value(&pages, VAT_TOTAL).as_deref(), Some("11.2"));
    }

    #[test]
    fn an_unlabelled_currency_is_taken_from_the_marks_of_amounts_if_they_agree() {
        let euro = [page(vec![block(50.0, 100.0, &["Summe", "571,04", "€"])])];
        assert_eq!(value(&euro, CURRENCY).as_deref(), Some("EUR"));
        let mixed = [page(vec![
            block(50.0, 100.0, &["571,04", "€"]),
            block(50.0, 120.0, &["CHF", "12,00"]),
        ])];
        assert_eq!(value(&mixed, CURRENCY), None);
        let register = [page(vec![block(50.0, 100.0, &["HRB", "372876"])])];
        assert_eq!(value(&register, CURRENCY), None);
        let no_currency = [page(vec![block(50.0, 100.0, &["VAT", "19,00"])])];
        assert_eq!(value(&no_currency, CURRENCY), None);
    }
}
