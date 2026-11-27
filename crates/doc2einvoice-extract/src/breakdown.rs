// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The VAT breakdown of an invoice (BG-23): for each rate the amount the tax
//! is charged on, the rate, the tax and the category.
//!
//! A breakdown is printed as a table under a heading that names its three
//! columns, or as rows of the totals that start with a word for VAT. In
//! both, a row counts only if its numbers agree: the amount the tax is
//! charged on, times the rate, is the tax. Numbers that merely stand beside
//! each other, as a rate, a price and a sum in an invoice line, do not.
//!
//! A row of the totals that gives the tax alone, as `USt. 19 % 16,61`, is
//! read where the totals of the invoice confirm it.

use std::str::FromStr;

use doc2einvoice_model::{AMOUNT_DECIMAL_PLACES, Entry, Location, VatCategory};
use doc2einvoice_pdf::Word;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::labels::{self, Scope, normalized};
use crate::page::Row;
use crate::values;

/// The business terms of a row of the breakdown.
pub const TAXABLE_AMOUNT: u16 = 116;
pub const TAX_AMOUNT: u16 = 117;
pub const CATEGORY: u16 = 118;
pub const RATE: u16 = 119;

/// One value of a row: its text in the form of the draft, the number it
/// stands for, and where it is printed.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub text: String,
    pub location: Option<Location>,
    value: Decimal,
}

/// One row of the breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct Breakdown {
    /// The amount the tax is charged on, where the row prints it.
    pub base: Option<Part>,
    pub rate: Part,
    pub tax: Part,
    /// The code of the category and where it is printed, or where the rate
    /// is that tells it.
    pub category: Option<(String, Option<Location>)>,
}

/// A column of a breakdown table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Base,
    Rate,
    Tax,
}

/// How many columns a breakdown has.
const COLUMN_COUNT: usize = 3;

/// The names the columns of a breakdown table are headed with, as
/// [`normalized`] gives their words. Of two names that start at a word, the
/// longer counts: "VAT amount" heads the tax, "VAT" alone the rate. One
/// program heads the amount "Net amout", and in German "Nettopreis".
const COLUMN_NAMES: &[(&[&str], Column)] = &[
    (&["basisbetrag"], Column::Base),
    (&["steuerbasisbetrag"], Column::Base),
    (&["net", "amount"], Column::Base),
    (&["net", "amout"], Column::Base),
    (&["base"], Column::Base),
    (&["nettopreis"], Column::Base),
    (&["ust", "satz"], Column::Rate),
    (&["steuersatz"], Column::Rate),
    (&["ust", "%"], Column::Rate),
    (&["vat"], Column::Rate),
    (&["vat", "rate"], Column::Rate),
    (&["taxe"], Column::Rate),
    (&["taux"], Column::Rate),
    (&["steuerbetrag"], Column::Tax),
    (&["ust.-betrag"], Column::Tax),
    (&["vat", "amount"], Column::Tax),
    (&["montant"], Column::Tax),
];

/// The words for VAT a row of the totals starts with, as [`normalized`]
/// gives them. Other taxes, as the one on insurance, are no VAT.
const VAT_WORDS: [&str; 6] = [
    "ust",
    "mwst",
    "umsatzsteuer",
    "mehrwertsteuer",
    "vat",
    "tva",
];

/// How far right of the heading of its last column a value of a table may
/// start, as a multiple of the font size of the heading: what stands
/// further right is beside the table, as the totals of the invoice.
const TABLE_REACH: f64 = 3.0;

/// The brackets the code of a category is printed in, as `(S)`.
const CODE_BRACKETS: (char, char) = ('(', ')');

/// A number of a row.
struct Figure {
    value: Decimal,
    text: String,
    /// Its first word in the row, and how many it spans.
    first: usize,
    count: usize,
    /// Whether it carries the sign of a rate.
    marked: bool,
    /// Whether it can be a monetary amount.
    monetary: bool,
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
fn figures(words: &[&Word]) -> Vec<Figure> {
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

/// Whether a tax of `tax` is what `rate` per cent of `base` come to, give
/// or take the smallest amount of money: the sum of taxes rounded line by
/// line may be off by it.
///
/// | Base    | Rate | Tax    | Agree |
/// |---------|------|--------|-------|
/// | 160.00  | 7    | 11.20  | yes   |
/// | 87.39   | 19   | 16.61  | yes (16.6041) |
/// | 87.39   | 19   | 16.60  | yes (off by the smallest amount) |
/// | 87.39   | 19   | 16.63  | no    |
/// | -100.00 | 19   | -19.00 | yes   |
/// | 220.00  | 0    | 0.00   | yes   |
/// | 0.025   | 19   | 20.00  | no    |
/// | too large to multiply | | | no  |
fn agree(base: Decimal, rate: Decimal, tax: Decimal) -> bool {
    let smallest = Decimal::new(1, AMOUNT_DECIMAL_PLACES);
    base.checked_mul(rate)
        .and_then(|product| product.checked_div(Decimal::ONE_HUNDRED))
        .map(|exact| {
            exact.round_dp_with_strategy(
                AMOUNT_DECIMAL_PLACES,
                RoundingStrategy::MidpointAwayFromZero,
            )
        })
        .and_then(|computed| computed.checked_sub(tax))
        .is_some_and(|difference| difference.abs() <= smallest)
}

fn part(row: &Row, figure: &Figure) -> Part {
    Part {
        text: figure.text.clone(),
        location: row
            .span(figure.first, figure.count)
            .and_then(|span| span.location(0, figure.count)),
        value: figure.value,
    }
}

/// A breakdown table: the order of its columns, and how far right its
/// values may start.
struct Table {
    order: [Column; COLUMN_COUNT],
    right: f64,
}

/// The table a row heads, if it names each column of a breakdown once.
///
/// | Row                                                          | Table              |
/// |--------------------------------------------------------------|--------------------|
/// | `Art` `(Kategorie)` `Warenwert` `Zu-/Abschlag` `Basisbetrag` `Steuersatz` `[%]` `Steuerbetrag` | base, rate, tax |
/// | `Steuerbasisbetrag` `USt.` `%` `USt.-Betrag`                 | base, rate, tax    |
/// | `VAT` `Net` `amount` `VAT` `amount`                          | rate, base, tax    |
/// | `Taxe` `Base` `Montant`                                      | rate, base, tax    |
/// | `Pos` `Beschreibung` `Menge` `Steuersatz` `Nettobetrag`      | none (the rate only) |
/// | `Datum` `Steuerbetrag` `Steuersatz`                          | none (no amount charged on) |
/// | `VAT` `Net` `amount` `VAT` `amount` `VAT`                    | none (a column twice) |
/// | `7%` `160,00` `11,20`                                        | none               |
fn heading(row: &Row) -> Option<Table> {
    let normal: Vec<String> = row
        .words
        .iter()
        .map(|word| normalized(&word.text))
        .collect();
    let mut order: Vec<Column> = Vec::new();
    let mut right: Option<f64> = None;
    let mut at = 0;
    while at < normal.len() {
        let named = COLUMN_NAMES
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
        let Some((name, column)) = named else {
            at += 1;
            continue;
        };
        if order.contains(column) {
            return None;
        }
        order.push(*column);
        at += name.len();
        let last = row.words.get(at - 1)?;
        right = Some(last.right + last.font_size * TABLE_REACH);
    }
    Some(Table {
        order: order.try_into().ok()?,
        right: right?,
    })
}

/// The row of a table its numbers make: the last three numbers standing
/// together that fill the columns in the order of the heading and agree. In
/// a table the rate may stand without its sign, under a column headed "%".
///
/// | Columns          | Numbers of the row            | Read                      |
/// |------------------|-------------------------------|---------------------------|
/// | base, rate, tax  | 15387.08, 19, 2923.55         | all three                 |
/// | base, rate, tax  | 100, -10, 90, 19, 17.10       | 90, 19, 17.10 (the last three) |
/// | base, rate, tax  | 220, 0, 0                     | 220, 0, 0                 |
/// | rate, base, tax  | 7%, 160, 11.20                | 160, 7, 11.20             |
/// | rate, base, tax  | 20%, 81.90, 16.38, 46.25      | 81.90, 20, 16.38 (a total beside the table left out) |
/// | rate, base, tax  | 19%, 0.025, 20                | none (do not agree)       |
/// | base, rate, tax  | 100, 119, 119                 | none (no rate)            |
/// | base, rate, tax  | 220, 0                        | none (too few)            |
fn table_row(row: &Row, words: &[&Word], order: [Column; COLUMN_COUNT]) -> Option<Breakdown> {
    let figures = figures(words);
    let place = |column: Column| order.iter().position(|other| *other == column);
    let (base_at, rate_at, tax_at) = (
        place(Column::Base)?,
        place(Column::Rate)?,
        place(Column::Tax)?,
    );
    figures.windows(COLUMN_COUNT).rev().find_map(|window| {
        let (base, rate, tax) = (&window[base_at], &window[rate_at], &window[tax_at]);
        let fits = base.monetary
            && tax.monetary
            && values::may_be_rate(rate.value)
            && agree(base.value, rate.value, tax.value);
        fits.then(|| Breakdown {
            base: Some(part(row, base)),
            rate: part(row, rate),
            tax: part(row, tax),
            category: None,
        })
    })
}

/// The words of `row` that belong to `table`: those that start no further
/// right than its values may.
fn within<'a>(row: &Row<'a>, table: &Table) -> Vec<&'a Word> {
    row.words
        .iter()
        .copied()
        .filter(|word| word.left <= table.right)
        .collect()
}

/// Whether the words of a row end a table: a label, the heading of a
/// section, or the heading of another table. A word that is a label only
/// after the invoice number, as "von", is an ordinary word here.
///
/// | Words of the row                   | Ends the table |
/// |------------------------------------|----------------|
/// | `Positionssumme` `320,00`          | yes (a label)  |
/// | `Käufer`                           | yes (a section) |
/// | `Taxe` `Base` `Montant`            | yes (another table) |
/// | `VAT` `(E)`                        | no             |
/// | `befreit` `von` `der` `Steuer`     | no             |
fn ends_table(row: &Row, words: &[&Word]) -> bool {
    let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    let labelled = labels::find(&texts)
        .iter()
        .any(|found| found.label.scope != Scope::AfterNumber);
    labelled || labels::heading(&texts).is_some() || heading(row).is_some()
}

/// The rows of the table that the row at `at` heads: the rows below it on
/// its page that make one, up to the row that ends the table. Rows between
/// them without numbers that agree are text of a cell, as the reason of an
/// exemption set over several lines.
///
/// The category of a row is the code printed in it or between it and the
/// row before, as `VAT (E)` above the numbers of its row.
///
/// A breakdown has one row for each rate of a category. A table with two
/// rows of the same is none: it lists the lines of the invoice, each with
/// its tax.
///
/// | Rows of the table (rate, category)  | Read  |
/// |-------------------------------------|-------|
/// | 7 % `S`, 19 % `S`                   | both  |
/// | 0 % `E`, 0 % `Z`                    | both  |
/// | 19 % `S`, 7 % `S`, 19 % `S`         | none  |
/// | 0 % and 0 %, no category printed    | none  |
fn table_rows(rows: &[Row], at: usize, table: &Table) -> Vec<Breakdown> {
    let mut found = Vec::new();
    let mut before = at;
    for (index, row) in rows.iter().enumerate().skip(at + 1) {
        if row.page != rows[at].page {
            break;
        }
        let words = within(row, table);
        if words.is_empty() {
            continue;
        }
        if let Some(mut breakdown) = table_row(row, &words, table.order) {
            let printed = rows[before + 1..=index]
                .iter()
                .rev()
                .find_map(|above| printed_category(above.page, &within(above, table)));
            breakdown.category = printed.or_else(|| told_by_rate(&breakdown.rate));
            found.push(breakdown);
            before = index;
        } else if ends_table(row, &words) {
            break;
        }
    }
    let code = |breakdown: &Breakdown| breakdown.category.as_ref().map(|(code, _)| code.clone());
    let repeats = found.iter().enumerate().any(|(index, row)| {
        found[..index]
            .iter()
            .any(|other| other.rate.value == row.rate.value && code(other) == code(row))
    });
    if repeats { Vec::new() } else { found }
}

/// The code of a category printed in brackets among `words`, and its place.
///
/// | Word            | Code |
/// |-----------------|------|
/// | `(S)`, `(AE)`   | `S`, `AE` |
/// | `(s)`, `(X)`    | none (no code of the list) |
/// | `S`, `(S`       | none (not in brackets) |
/// | `(380)`         | none |
fn printed_category(page: usize, words: &[&Word]) -> Option<(String, Option<Location>)> {
    words.iter().find_map(|word| {
        let code = word
            .text
            .strip_prefix(CODE_BRACKETS.0)?
            .strip_suffix(CODE_BRACKETS.1)?;
        let category = VatCategory::from_code(code).ok()?;
        let location = Location {
            page,
            left: word.left,
            top: word.baseline - word.font_size,
            right: word.right,
            bottom: word.baseline,
        };
        Some((category.code().to_owned(), Some(location)))
    })
}

/// The category a rate tells where none is printed: a rate above nothing is
/// the standard rate, which the standard also files reduced rates under.
/// Without tax the category is open: exempt, zero rated, reverse charge and
/// others all print 0 %.
///
/// | Rate | Category |
/// |------|----------|
/// | 19   | `S`      |
/// | 7    | `S`      |
/// | 0    | none     |
fn told_by_rate(rate: &Part) -> Option<(String, Option<Location>)> {
    (rate.value > Decimal::ZERO)
        .then(|| (VatCategory::StandardRate.code().to_owned(), rate.location))
}

/// The row of the totals `row` is, if it starts with a word for VAT before
/// any number, its first number is a rate with its sign, and what follows
/// is the amount the tax is charged on and the tax, which agree, or one
/// amount alone, the tax.
///
/// | Row                                              | Read                    |
/// |--------------------------------------------------|-------------------------|
/// | `Steuerbasisbetrag` `USt.` `7%` `275,00` `19,25` | 275, 7, 19.25           |
/// | `zzgl.` `Umsatzsteuer` `7%` `von` `129,37` `9,06` | 129.37, 7, 9.06        |
/// | `TVA` `collectée` `(vente)` `20,0%` `81,90` `€` `16,38` `€` | 81.90, 20, 16.38 |
/// | `TVA` `20%` `81,90` `16,38` `46,25`              | 81.90, 20, 16.38 (the first two that agree) |
/// | `USt.` `(19` `%)` `16,61` `€`                    | 19, 16.61, no amount charged on |
/// | `MWST` `19,00` `%` `566,45` `EUR`                | 19, 566.45, no amount charged on |
/// | `MwSt` `19%` `100,00` `25,00`                    | none (do not agree)     |
/// | `MwSt` `19%`                                     | none (no amount)        |
/// | `Umsatzsteuer` `16,61`                           | none (no rate)          |
/// | `Umsatzsteuer` `19` `16,61`                      | none (no sign of a rate) |
/// | `1` `Trennblätter` `9,99` `19` `%` `VAT` `(S)` `199,80` | none (a number first) |
/// | `Versicherungsteuer` `19` `%` `9,99`             | none (no VAT)           |
/// | `2%` `Skonto` `USt.` `19%` `100,00` `19,00`      | none (a number first)   |
fn totals_row(row: &Row) -> Option<Breakdown> {
    let figures = figures(&row.words);
    let (rate, amounts) = figures.split_first()?;
    let labelled = row.words[..rate.first]
        .iter()
        .any(|word| VAT_WORDS.contains(&normalized(&word.text).as_str()));
    if !labelled || !rate.marked {
        return None;
    }
    let amounts: Vec<&Figure> = amounts
        .iter()
        .take_while(|figure| !figure.marked)
        .filter(|figure| figure.monetary)
        .collect();
    let pair = amounts
        .windows(2)
        .find(|pair| agree(pair[0].value, rate.value, pair[1].value));
    let (base, tax) = match (pair, amounts.as_slice()) {
        (Some(pair), _) => (Some(pair[0]), pair[1]),
        (None, [tax]) => (None, *tax),
        (None, _) => return None,
    };
    let rate = part(row, rate);
    Some(Breakdown {
        base: base.map(|base| part(row, base)),
        tax: part(row, tax),
        category: printed_category(row.page, &row.words).or_else(|| told_by_rate(&rate)),
        rate,
    })
}

/// The rows of the breakdown printed on `rows`, each once: those of the
/// first table that has any, or else the rows of the totals.
pub fn read(rows: &[Row]) -> Vec<Breakdown> {
    let table = rows
        .iter()
        .enumerate()
        .filter_map(|(at, row)| heading(row).map(|table| table_rows(rows, at, &table)))
        .find(|found| !found.is_empty());
    let found = table.unwrap_or_else(|| rows.iter().filter_map(totals_row).collect());
    let mut once: Vec<Breakdown> = Vec::new();
    for breakdown in found {
        let printed_before = once.iter().any(|other| same_values(other, &breakdown));
        if !printed_before {
            once.push(breakdown);
        }
    }
    once
}

/// Whether two rows state the same: totals printed again on a later page.
fn same_values(a: &Breakdown, b: &Breakdown) -> bool {
    let value = |part: &Part| part.value;
    a.rate.value == b.rate.value
        && a.tax.value == b.tax.value
        && a.base.as_ref().map(value) == b.base.as_ref().map(value)
        && a.category.as_ref().map(|(code, _)| code) == b.category.as_ref().map(|(code, _)| code)
}

/// The rows the totals of the invoice bear out. Rows that print the amount
/// the tax is charged on bear themselves out, and where there are any, only
/// they count. Rows that print the tax alone need a total:
///
/// | Rows                         | Totals                                  | Read                 |
/// |------------------------------|-----------------------------------------|----------------------|
/// | with the amount charged on   | any                                     | those rows           |
/// | one, the tax alone           | the total without VAT agrees with it    | it, charged on that total |
/// | one or more, the tax alone   | their taxes add up to the total of VAT  | them, no amount charged on |
/// | the tax alone                | neither                                 | none                 |
/// | none                         | any                                     | none                 |
pub fn borne_out(
    found: Vec<Breakdown>,
    without_vat: Option<&Entry>,
    vat_total: Option<&Entry>,
) -> Vec<Breakdown> {
    let (whole, mut alone): (Vec<Breakdown>, Vec<Breakdown>) = found
        .into_iter()
        .partition(|breakdown| breakdown.base.is_some());
    if !whole.is_empty() {
        return whole;
    }
    let number = |entry: &Entry| Decimal::from_str(&entry.text).ok();
    if let ([only], Some(total)) = (alone.as_mut_slice(), without_vat)
        && let Some(value) = number(total)
        && agree(value, only.rate.value, only.tax.value)
    {
        only.base = Some(Part {
            text: total.text.clone(),
            location: total.location,
            value,
        });
        return alone;
    }
    let taxes = alone.iter().try_fold(Decimal::ZERO, |sum, breakdown| {
        sum.checked_add(breakdown.tax.value)
    });
    let added_up = !alone.is_empty() && taxes.is_some() && taxes == vat_total.and_then(number);
    if added_up { alone } else { Vec::new() }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use doc2einvoice_model::{Entry, Origin};
    use doc2einvoice_pdf::Word;
    use rust_decimal::Decimal;

    use super::{
        Breakdown, Column, agree, borne_out, ends_table, figures, heading, printed_category, read,
        table_row, totals_row,
    };
    use crate::page::Row;

    /// The font size of every word, the width of one character, the gap
    /// between two words and the distance of two baselines.
    const FONT: f64 = 10.0;
    const CHARACTER: f64 = 5.0;
    const GAP: f64 = 3.0;
    const PITCH: f64 = 12.0;
    /// Where the rows of the tests start.
    const LEFT: f64 = 50.0;
    const TOP: f64 = 100.0;

    /// The words of `texts` from `left` on `baseline`.
    fn words(left: f64, baseline: f64, texts: &[&str]) -> Vec<Word> {
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

    fn row(words: &[Word]) -> Row<'_> {
        Row {
            page: 0,
            words: words.iter().collect(),
        }
    }

    fn number(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap_or_default()
    }

    /// The amount charged on, the rate, the tax and the category of a row.
    type Values = (Option<String>, String, String, Option<String>);

    fn values(breakdown: &Breakdown) -> Values {
        (
            breakdown.base.as_ref().map(|base| base.text.clone()),
            breakdown.rate.text.clone(),
            breakdown.tax.text.clone(),
            breakdown.category.as_ref().map(|(code, _)| code.clone()),
        )
    }

    fn expected(base: Option<&str>, rate: &str, tax: &str, category: Option<&str>) -> Values {
        (
            base.map(str::to_owned),
            rate.to_owned(),
            tax.to_owned(),
            category.map(str::to_owned),
        )
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

    #[test]
    fn amounts_agree_as_their_table_says() {
        let table = [
            ("160", "7", "11.2", true),
            ("87.39", "19", "16.61", true),
            ("87.39", "19", "16.60", true),
            ("87.39", "19", "16.63", false),
            ("-100.00", "19", "-19.00", true),
            ("220", "0", "0", true),
            ("0.025", "19", "20.00", false),
        ];
        for (base, rate, tax, agrees) in table {
            assert_eq!(
                agree(number(base), number(rate), number(tax)),
                agrees,
                "{base} {rate} {tax}"
            );
        }
        assert!(!agree(Decimal::MAX, Decimal::ONE_HUNDRED, Decimal::ZERO));
    }

    #[test]
    fn headings_follow_their_table() {
        use Column::{Base, Rate, Tax};
        let table: [(&[&str], Option<[Column; 3]>); 8] = [
            (
                &[
                    "Art",
                    "(Kategorie)",
                    "Warenwert",
                    "Zu-/Abschlag",
                    "Basisbetrag",
                    "Steuersatz",
                    "[%]",
                    "Steuerbetrag",
                ],
                Some([Base, Rate, Tax]),
            ),
            (
                &["Steuerbasisbetrag", "USt.", "%", "USt.-Betrag"],
                Some([Base, Rate, Tax]),
            ),
            (
                &["VAT", "Net", "amount", "VAT", "amount"],
                Some([Rate, Base, Tax]),
            ),
            (&["Taxe", "Base", "Montant"], Some([Rate, Base, Tax])),
            (
                &["Pos", "Beschreibung", "Menge", "Steuersatz", "Nettobetrag"],
                None,
            ),
            (&["Datum", "Steuerbetrag", "Steuersatz"], None),
            (&["VAT", "Net", "amount", "VAT", "amount", "VAT"], None),
            (&["7%", "160,00", "11,20"], None),
        ];
        for (texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            let order = heading(&row(&printed)).map(|table| table.order);
            assert_eq!(order, expected, "{texts:?}");
        }
    }

    #[test]
    fn rows_of_a_table_follow_their_table() {
        use Column::{Base, Rate, Tax};
        let table: [([Column; 3], &[&str], Option<Values>); 8] = [
            (
                [Base, Rate, Tax],
                &["15.387,08", "19", "2.923,55"],
                Some(expected(Some("15387.08"), "19", "2923.55", None)),
            ),
            (
                [Base, Rate, Tax],
                &["100,00", "-10,00", "90,00", "19", "17,10"],
                Some(expected(Some("90"), "19", "17.1", None)),
            ),
            (
                [Base, Rate, Tax],
                &["220,00", "0", "0,00"],
                Some(expected(Some("220"), "0", "0", None)),
            ),
            (
                [Rate, Base, Tax],
                &["7%", "160,00", "€", "11,20", "€"],
                Some(expected(Some("160"), "7", "11.2", None)),
            ),
            (
                [Rate, Base, Tax],
                &["20,0%", "81,90", "16,38", "46,25"],
                Some(expected(Some("81.9"), "20", "16.38", None)),
            ),
            ([Rate, Base, Tax], &["19%", "0,025", "20,00"], None),
            ([Base, Rate, Tax], &["100,00", "119", "119,00"], None),
            ([Base, Rate, Tax], &["220,00", "0"], None),
        ];
        for (order, texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            let printed = row(&printed);
            let found = table_row(&printed, &printed.words, order);
            assert_eq!(found.as_ref().map(values), expected, "{texts:?}");
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
            assert_eq!(found.map(|(code, _)| code).as_deref(), expected, "{text}");
        }
    }

    #[test]
    fn rows_of_the_totals_follow_their_table() {
        let table: [(&[&str], Option<Values>); 13] = [
            (
                &["Steuerbasisbetrag", "USt.", "7%", "275,00", "19,25"],
                Some(expected(Some("275"), "7", "19.25", Some("S"))),
            ),
            (
                &["zzgl.", "Umsatzsteuer", "7%", "von", "129,37", "9,06"],
                Some(expected(Some("129.37"), "7", "9.06", Some("S"))),
            ),
            (
                &[
                    "TVA",
                    "collectée",
                    "(vente)",
                    "20,0%",
                    "81,90",
                    "€",
                    "16,38",
                    "€",
                ],
                Some(expected(Some("81.9"), "20", "16.38", Some("S"))),
            ),
            (
                &["TVA", "20%", "81,90", "16,38", "46,25"],
                Some(expected(Some("81.9"), "20", "16.38", Some("S"))),
            ),
            (
                &["USt.", "(19", "%)", "16,61", "€"],
                Some(expected(None, "19", "16.61", Some("S"))),
            ),
            (
                &["MWST", "19,00", "%", "566,45", "EUR"],
                Some(expected(None, "19", "566.45", Some("S"))),
            ),
            (&["MwSt", "19%", "100,00", "25,00"], None),
            (&["MwSt", "19%"], None),
            (&["Umsatzsteuer", "16,61"], None),
            (&["Umsatzsteuer", "19", "16,61"], None),
            (
                &[
                    "1",
                    "Trennblätter",
                    "9,99",
                    "19",
                    "%",
                    "VAT",
                    "(S)",
                    "199,80",
                ],
                None,
            ),
            (&["Versicherungsteuer", "19", "%", "9,99"], None),
            (&["2%", "Skonto", "USt.", "19%", "100,00", "19,00"], None),
        ];
        for (texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            let found = totals_row(&row(&printed));
            assert_eq!(found.as_ref().map(values), expected, "{texts:?}");
        }
        let exempt = words(LEFT, TOP, &["TVA", "0%", "export", "(G)", "100,00", "0,00"]);
        assert_eq!(
            totals_row(&row(&exempt)).as_ref().map(values),
            Some(expected(Some("100"), "0", "0", Some("G")))
        );
        let open = words(LEFT, TOP, &["TVA", "0%", "export", "100,00", "0,00"]);
        assert_eq!(
            totals_row(&row(&open)).as_ref().map(values),
            Some(expected(Some("100"), "0", "0", None))
        );
    }

    /// The rows of a table, each `PITCH` below the one before.
    fn stacked(rows: &[&[&str]]) -> Vec<Vec<Word>> {
        let mut baseline = TOP;
        rows.iter()
            .map(|texts| {
                let printed = words(LEFT, baseline, texts);
                baseline += PITCH;
                printed
            })
            .collect()
    }

    fn read_values(printed: &[Vec<Word>]) -> Vec<Values> {
        let rows: Vec<Row> = printed.iter().map(|words| row(words)).collect();
        read(&rows).iter().map(values).collect()
    }

    #[test]
    fn a_table_is_read_up_to_the_row_that_ends_it() {
        let printed = stacked(&[
            &["Art", "Basisbetrag", "Steuersatz", "Steuerbetrag"],
            &["VAT", "(E)"],
            &["Umsatzsteuerbefreit", "nach", "§4", "Nr.", "10a"],
            &["220,00", "0", "0,00"],
            &["Umsatzsteuer", "(S)", "100,00", "19", "19,00"],
            &["Positionssumme", "320,00"],
            &["100,00", "7", "7,00"],
        ]);
        assert_eq!(
            read_values(&printed),
            [
                expected(Some("220"), "0", "0", Some("E")),
                expected(Some("100"), "19", "19", Some("S")),
            ]
        );
    }

    #[test]
    fn a_table_with_two_rows_of_one_rate_and_category_is_no_breakdown() {
        const HEADING: &[&str] = &["USt.", "Satz", "Nettopreis", "Ust.-Betrag"];
        let both = stacked(&[
            HEADING,
            &["7%", "160,00", "11,20"],
            &["19%", "336,00", "63,84"],
        ]);
        assert_eq!(
            read_values(&both),
            [
                expected(Some("160"), "7", "11.2", Some("S")),
                expected(Some("336"), "19", "63.84", Some("S")),
            ]
        );
        let exempt_and_zero_rated = stacked(&[
            HEADING,
            &["(E)", "0%", "100,00", "0,00"],
            &["(Z)", "0%", "50,00", "0,00"],
        ]);
        assert_eq!(
            read_values(&exempt_and_zero_rated),
            [
                expected(Some("100"), "0", "0", Some("E")),
                expected(Some("50"), "0", "0", Some("Z")),
            ]
        );
        let lines = stacked(&[
            HEADING,
            &["19%", "100,00", "19,00"],
            &["7%", "100,00", "7,00"],
            &["19%", "200,00", "38,00"],
        ]);
        assert_eq!(read_values(&lines), []);
        let open = stacked(&[HEADING, &["0%", "100,00", "0,00"], &["0%", "50,00", "0,00"]]);
        assert_eq!(read_values(&open), []);
    }

    #[test]
    fn the_end_of_a_table_follows_its_table() {
        let table: [(&[&str], bool); 5] = [
            (&["Positionssumme", "320,00"], true),
            (&["Käufer"], true),
            (&["Taxe", "Base", "Montant"], true),
            (&["VAT", "(E)"], false),
            (&["befreit", "von", "der", "Steuer"], false),
        ];
        for (texts, ends) in table {
            let printed = words(LEFT, TOP, texts);
            let printed = row(&printed);
            assert_eq!(ends_table(&printed, &printed.words), ends, "{texts:?}");
        }
    }

    #[test]
    fn what_stands_right_of_a_table_is_not_in_it() {
        let mut printed = stacked(&[
            &["Taxe", "Base", "Montant"],
            &["TVA", "20,0%", "81,90", "16,38"],
        ]);
        let far_right = LEFT + 400.0;
        printed[0].extend(words(far_right, TOP, &["Total", "HT", "624,90"]));
        printed[1].extend(words(far_right, TOP + PITCH, &["Total", "TTC", "671,15"]));
        assert_eq!(
            read_values(&printed),
            [expected(Some("81.9"), "20", "16.38", Some("S"))]
        );
    }

    #[test]
    fn rows_of_the_totals_count_only_without_a_table_and_once() {
        let table_and_totals = stacked(&[
            &["VAT", "Net", "amount", "VAT", "amount"],
            &["7%", "160,00", "11,20"],
            &["Total", "VAT", "11,20"],
            &["VAT", "19%", "100,00", "19,00"],
        ]);
        assert_eq!(
            read_values(&table_and_totals),
            [expected(Some("160"), "7", "11.2", Some("S"))]
        );
        let twice = stacked(&[
            &["USt.", "19%", "100,00", "19,00"],
            &["USt.", "7%", "100,00", "7,00"],
            &["USt.", "19%", "100,00", "19,00"],
        ]);
        assert_eq!(
            read_values(&twice),
            [
                expected(Some("100"), "19", "19", Some("S")),
                expected(Some("100"), "7", "7", Some("S")),
            ]
        );
    }

    fn total(text: &str) -> Entry {
        Entry {
            term: 0,
            within: Vec::new(),
            text: text.to_owned(),
            origin: Origin::Rule,
            location: None,
        }
    }

    fn borne_out_values(
        rows: &[&[&str]],
        without_vat: Option<&str>,
        vat_total: Option<&str>,
    ) -> Vec<Values> {
        let printed = stacked(rows);
        let rows: Vec<Row> = printed.iter().map(|words| row(words)).collect();
        let (without_vat, vat_total) = (without_vat.map(total), vat_total.map(total));
        borne_out(read(&rows), without_vat.as_ref(), vat_total.as_ref())
            .iter()
            .map(values)
            .collect()
    }

    #[test]
    fn rows_are_borne_out_as_their_table_says() {
        let whole: &[&[&str]] = &[&["USt.", "19%", "100,00", "19,00"], &["USt.", "7%", "7,00"]];
        assert_eq!(
            borne_out_values(whole, None, None),
            [expected(Some("100"), "19", "19", Some("S"))]
        );
        let one: &[&[&str]] = &[&["USt.", "(19", "%)", "16,61"]];
        assert_eq!(
            borne_out_values(one, Some("87.39"), None),
            [expected(Some("87.39"), "19", "16.61", Some("S"))]
        );
        assert_eq!(
            borne_out_values(one, Some("90"), Some("16.61")),
            [expected(None, "19", "16.61", Some("S"))]
        );
        assert_eq!(borne_out_values(one, Some("90"), Some("17.00")), []);
        assert_eq!(borne_out_values(one, None, None), []);
        let two: &[&[&str]] = &[&["USt.", "19%", "19,00"], &["USt.", "7%", "7,00"]];
        assert_eq!(
            borne_out_values(two, Some("200.00"), Some("26.00")),
            [
                expected(None, "19", "19", Some("S")),
                expected(None, "7", "7", Some("S")),
            ]
        );
        assert_eq!(borne_out_values(two, Some("200.00"), Some("19")), []);
        assert_eq!(borne_out_values(&[], Some("200.00"), Some("0")), []);
    }
}
