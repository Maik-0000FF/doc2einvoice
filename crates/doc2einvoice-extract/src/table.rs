// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the tables of an invoice have in common: the numbers of a row, the
//! columns a heading names, where a table ends, whether numbers agree, and
//! the VAT category of a row.

use std::ops::Range;
use std::str::FromStr;

use doc2einvoice_model::{AMOUNT_DECIMAL_PLACES, Location, Origin, VatCategory};
use doc2einvoice_pdf::Word;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::labels::{self, Scope, bracketed, normalized};
use crate::page::{Row, Span};
use crate::values;

/// One number of a row: its text in the form of the draft, the number it
/// stands for, where it is printed, and whether it was read there or worked
/// out from what is printed there.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub text: String,
    pub location: Option<Location>,
    pub value: Decimal,
    pub origin: Origin,
}

impl Part {
    /// The value as the draft takes it.
    pub fn stated(&self) -> Stated {
        Stated {
            text: self.text.clone(),
            location: self.location,
            origin: self.origin,
        }
    }
}

/// A value of a row as the draft takes it: its text, where it is printed,
/// and whether it was read there or worked out from what is printed there.
/// A value that is worked out is never passed off as printed: the user is
/// shown which values the page does not state itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Stated {
    pub text: String,
    pub location: Option<Location>,
    pub origin: Origin,
}

/// A number of a row.
pub struct Figure {
    pub value: Decimal,
    pub text: String,
    /// Its first word in the row, and how many it spans.
    pub first: usize,
    pub count: usize,
    /// Whether it carries the sign of a rate.
    pub marked: bool,
    /// Whether it can be a monetary amount.
    pub monetary: bool,
}

/// The numbers of `words`, from left to right.
///
/// | Words                     | Figures                          |
/// |---------------------------|----------------------------------|
/// | `7%` `160,00` `€` `11,20` | rate 7; 160; 11.2                |
/// | `19` `%` `64,46`          | rate 19 (two words); 64.46       |
/// | `19` `2.923,55`           | 19 (could be a rate); 2923.55    |
/// | `0,025`                   | 0.025 (no monetary amount)       |
/// | `USt.` `(S)` `16.09.2026` | none                             |
pub fn figures(words: &[&Word]) -> Vec<Figure> {
    let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < texts.len() {
        let rate = values::rate(&texts[at..]);
        let amount = values::amount(&texts[at..=at]).filter(|read| read.skipped == 0);
        let monetary = values::monetary(&texts[at..=at]).filter(|read| read.skipped == 0);
        let marked = rate.is_some();
        let count = rate.as_ref().map_or(1, |read| read.used);
        let read = rate.or_else(|| monetary.clone()).or(amount);
        if let Some(read) = read
            && let Ok(value) = Decimal::from_str(&read.text)
        {
            found.push(Figure {
                value,
                text: read.text,
                first: at,
                count,
                marked,
                monetary: !marked && monetary.is_some(),
            });
        }
        at += count;
    }
    found
}

/// The value `figure` of `row` stands for, with its place.
pub fn part(row: &Row, figure: &Figure) -> Part {
    Part {
        text: figure.text.clone(),
        location: row
            .span(figure.first, figure.count)
            .and_then(|span| span.location(0, figure.count)),
        value: figure.value,
        origin: Origin::Rule,
    }
}

/// The columns `words` name, from left to right, each with the words of
/// its name. The names are written as [`normalized`] gives their words. Of
/// two names that start at a word, the longer counts.
///
/// | Names                         | Words                     | Named                  |
/// |-------------------------------|---------------------------|------------------------|
/// | `vat`: rate, `vat amount`: tax | `VAT` `Net` `VAT` `amount` | rate (0), tax (2 and 3) |
/// | `menge`: quantity             | `Pos` `Menge:` `Preis`    | quantity (1)           |
/// | `ust %`: rate                 | `USt.` `%`                | rate (0 and 1)         |
/// | `ust %`: rate                 | `USt.`                    | none (the name cut short) |
/// | any                           | nothing                   | none                   |
pub fn named<C: Copy>(names: &[(&[&str], C)], words: &[&Word]) -> Vec<(C, Range<usize>)> {
    let normal: Vec<String> = words.iter().map(|word| normalized(&word.text)).collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < normal.len() {
        let longest = names
            .iter()
            .filter(|(name, _)| {
                normal.get(at..at + name.len()).is_some_and(|printed| {
                    printed
                        .iter()
                        .zip(name.iter())
                        .all(|(word, expected)| word.as_str() == *expected)
                })
            })
            .max_by_key(|(name, _)| name.len());
        match longest {
            Some((name, column)) => {
                found.push((*column, at..at + name.len()));
                at += name.len();
            }
            None => at += 1,
        }
    }
    found
}

/// Whether the words of a row end a table: a label or the heading of a
/// section. A word that is a label only after the invoice number, as "von",
/// is an ordinary word here.
///
/// | Words of the row                   | Ends the table  |
/// |------------------------------------|-----------------|
/// | `Positionssumme` `320,00`          | yes (a label)   |
/// | `Käufer`                           | yes (a section) |
/// | `VAT` `(E)`                        | no              |
/// | `befreit` `von` `der` `Steuer`     | no              |
pub fn ends(words: &[&Word]) -> bool {
    let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    let labelled = labels::find(&texts)
        .iter()
        .any(|found| found.label.scope != Scope::AfterNumber);
    labelled || labels::heading(&texts).is_some()
}

/// Whether a printed amount is what a calculation comes to, give or take
/// the smallest amount of money: a sum of amounts rounded one by one may be
/// off by it. A calculation that could not be carried out comes to nothing.
///
/// | Calculated | Printed | Comes to it |
/// |------------|---------|-------------|
/// | 16.6041    | 16.61   | yes         |
/// | 16.6041    | 16.60   | yes (off by the smallest amount) |
/// | 16.6041    | 16.63   | no          |
/// | 11.2       | 11.20   | yes         |
/// | -19        | -19.00  | yes         |
/// | 0.005      | 0.02    | yes (0.01 after rounding) |
/// | 0.00475    | 20.00   | no          |
/// | none       | any     | no          |
pub fn comes_to(calculated: Option<Decimal>, printed: Decimal) -> bool {
    let smallest = Decimal::new(1, AMOUNT_DECIMAL_PLACES);
    calculated
        .map(|exact| {
            exact.round_dp_with_strategy(
                AMOUNT_DECIMAL_PLACES,
                RoundingStrategy::MidpointAwayFromZero,
            )
        })
        .and_then(|rounded| rounded.checked_sub(printed))
        .is_some_and(|difference| difference.abs() <= smallest)
}

/// The code of a VAT category printed in brackets among `words`, and its
/// place.
///
/// | Word            | Code |
/// |-----------------|------|
/// | `(S)`, `(AE)`   | `S`, `AE` |
/// | `(s)`, `(X)`    | none (no code of the list) |
/// | `S`, `(S`       | none (not in brackets) |
/// | `(380)`         | none |
pub fn printed_category(page: usize, words: &[&Word]) -> Option<Stated> {
    words.iter().find_map(|word| {
        let category = VatCategory::from_code(bracketed(&word.text)?).ok()?;
        let printed = Span {
            page,
            words: vec![*word],
        };
        Some(Stated {
            text: category.code().to_owned(),
            location: printed.location(0, 1),
            origin: Origin::Rule,
        })
    })
}

/// The VAT category a rate tells where none is printed, marked as worked
/// out and placed at the rate: a rate above nothing is the standard rate,
/// which the standard also files reduced rates under. Without tax the
/// category is open: exempt, zero rated, reverse charge and others all
/// print 0 %.
///
/// | Rate | Category             |
/// |------|----------------------|
/// | 19   | `S`, worked out      |
/// | 7    | `S`, worked out      |
/// | 0    | none                 |
pub fn told_by_rate(rate: &Part) -> Option<Stated> {
    (rate.value > Decimal::ZERO).then(|| Stated {
        text: VatCategory::StandardRate.code().to_owned(),
        location: rate.location,
        origin: Origin::Derived,
    })
}

/// Rows built from texts, for the tests of the tables.
#[cfg(test)]
pub mod printed {
    use doc2einvoice_pdf::Word;

    use crate::page::Row;

    /// The font size of every word, the width of one character, the gap
    /// between two words and the distance of two baselines.
    const FONT: f64 = 10.0;
    const CHARACTER: f64 = 5.0;
    const GAP: f64 = 3.0;
    pub const PITCH: f64 = 12.0;
    /// Where the rows start.
    pub const LEFT: f64 = 50.0;
    pub const TOP: f64 = 100.0;

    /// The words of `texts` from `left` on `baseline`.
    pub fn words(left: f64, baseline: f64, texts: &[&str]) -> Vec<Word> {
        let mut at = left;
        texts
            .iter()
            .map(|text| {
                let right = text.chars().fold(at, |end, _| end + CHARACTER);
                let word = Word {
                    text: (*text).to_owned(),
                    left: at,
                    right,
                    baseline,
                    font_size: FONT,
                };
                at = right + GAP;
                word
            })
            .collect()
    }

    /// The words as one row of the first page.
    pub fn row(words: &[Word]) -> Row<'_> {
        Row {
            page: 0,
            words: words.iter().collect(),
        }
    }

    /// The rows of a table, each `PITCH` below the one before.
    pub fn stacked(rows: &[&[&str]]) -> Vec<Vec<Word>> {
        let mut baseline = TOP;
        rows.iter()
            .map(|texts| {
                let printed = words(LEFT, baseline, texts);
                baseline += PITCH;
                printed
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::printed::{LEFT, TOP, row, words};
    use doc2einvoice_model::Origin;

    use super::{Part, comes_to, ends, figures, named, printed_category, told_by_rate};

    fn number(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap_or_default()
    }

    #[test]
    fn figures_follow_their_table() {
        type Figures = &'static [(&'static str, bool, bool)];
        let table: [(&[&str], Figures); 5] = [
            (
                &["7%", "160,00", "€", "11,20"],
                &[
                    ("7", true, false),
                    ("160", false, true),
                    ("11.2", false, true),
                ],
            ),
            (
                &["19", "%", "64,46"],
                &[("19", true, false), ("64.46", false, true)],
            ),
            (
                &["19", "2.923,55"],
                &[("19", false, true), ("2923.55", false, true)],
            ),
            (&["0,025"], &[("0.025", false, false)]),
            (&["USt.", "(S)", "16.09.2026"], &[]),
        ];
        for (texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            let found = figures(&printed.iter().collect::<Vec<_>>());
            let got: Vec<(Decimal, bool, bool)> = found
                .iter()
                .map(|figure| (figure.value, figure.marked, figure.monetary))
                .collect();
            let want: Vec<(Decimal, bool, bool)> = expected
                .iter()
                .map(|(text, marked, monetary)| (number(text), *marked, *monetary))
                .collect();
            assert_eq!(got, want, "{texts:?}");
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Column {
        Rate,
        Tax,
        Quantity,
    }

    #[test]
    fn named_columns_follow_their_table() {
        const NAMES: &[(&[&str], Column)] = &[
            (&["vat"], Column::Rate),
            (&["vat", "amount"], Column::Tax),
            (&["menge"], Column::Quantity),
            (&["ust", "%"], Column::Rate),
        ];
        type Named = &'static [(Column, Range<usize>)];
        let table: [(&[&str], Named); 5] = [
            (
                &["VAT", "Net", "VAT", "amount"],
                &[(Column::Rate, 0..1), (Column::Tax, 2..4)],
            ),
            (&["Pos", "Menge:", "Preis"], &[(Column::Quantity, 1..2)]),
            (&["USt.", "%"], &[(Column::Rate, 0..2)]),
            (&["USt."], &[]),
            (&[], &[]),
        ];
        for (texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            assert_eq!(named(NAMES, &row(&printed).words), expected, "{texts:?}");
        }
    }

    #[test]
    fn the_end_of_a_table_follows_its_table() {
        let table: [(&[&str], bool); 4] = [
            (&["Positionssumme", "320,00"], true),
            (&["Käufer"], true),
            (&["VAT", "(E)"], false),
            (&["befreit", "von", "der", "Steuer"], false),
        ];
        for (texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            assert_eq!(ends(&row(&printed).words), expected, "{texts:?}");
        }
    }

    #[test]
    fn calculations_come_to_amounts_as_their_table_says() {
        let table = [
            ("16.6041", "16.61", true),
            ("16.6041", "16.60", true),
            ("16.6041", "16.63", false),
            ("11.2", "11.20", true),
            ("-19", "-19.00", true),
            ("0.005", "0.02", true),
            ("0.00475", "20.00", false),
        ];
        for (calculated, printed, expected) in table {
            assert_eq!(
                comes_to(Some(number(calculated)), number(printed)),
                expected,
                "{calculated} {printed}"
            );
        }
        assert!(!comes_to(None, Decimal::ZERO));
    }

    #[test]
    fn a_rate_tells_the_category_as_its_table_says() {
        let rate = |text: &str| Part {
            text: text.to_owned(),
            location: None,
            value: number(text),
            origin: Origin::Rule,
        };
        for (text, expected) in [("19", Some("S")), ("7", Some("S")), ("0", None)] {
            let told = told_by_rate(&rate(text));
            assert_eq!(told.as_ref().map(|code| code.text.as_str()), expected);
            assert!(told.is_none_or(|code| code.origin == Origin::Derived));
        }
    }

    #[test]
    fn codes_of_categories_follow_their_table() {
        let table: [(&str, Option<&str>); 8] = [
            ("(S)", Some("S")),
            ("(AE)", Some("AE")),
            ("(s)", None),
            ("(X)", None),
            ("S", None),
            ("(S", None),
            ("(380)", None),
            ("()", None),
        ];
        for (text, expected) in table {
            let printed = words(LEFT, TOP, &[text]);
            let found = printed_category(0, &printed.iter().collect::<Vec<_>>());
            assert_eq!(found.map(|code| code.text).as_deref(), expected, "{text}");
        }
    }
}
