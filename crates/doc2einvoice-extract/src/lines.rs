// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The lines of an invoice (BG-25), printed as a table with one row for each
//! line under a heading that names its columns.
//!
//! A row counts only if its numbers agree: the quantity times the price,
//! less a discount where one is printed, is the amount of the line. The
//! lines count only if their amounts add up to a total the page prints. Rows
//! between the lines, as the number of an item, are passed over; the lines
//! a name runs on over are read with it.

use std::str::FromStr;

use doc2einvoice_model::{AMOUNT_DECIMAL_PLACES, Entry, Location, Origin};
use doc2einvoice_pdf::Word;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::breakdown;
use crate::labels::{self, Names, TOTALS};
use crate::page::{self, Row, Span};
use crate::table::{self, Figure, Part, Reading, Stated, comes_to, figures, figures_as, part};
use crate::units;
use crate::values;

/// One line of an invoice.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The number of the line, where the row starts with one.
    pub identifier: Option<Part>,
    /// The name of the item.
    pub name: Option<Stated>,
    pub quantity: Part,
    /// The code of the unit of the quantity, looked up for the word printed
    /// after it and marked as worked out: the page prints the word, not the
    /// code. None where the word is no unit the table knows.
    pub unit: Option<Stated>,
    /// The net price: as printed, or, where a discount is taken off the
    /// price printed, worked out from the two and marked so. None where the
    /// row prints no price: the amount by the quantity is not the price where
    /// allowances are taken off the line.
    pub price: Option<Part>,
    /// The gross price of one, where a discount is taken off it.
    pub gross_price: Option<Part>,
    /// The quantity the price is for, where the numbers of the line agree
    /// only with a price for a hundred or a thousand: worked out and marked
    /// so, as the page does not print it.
    pub price_base: Option<Part>,
    /// The code of the unit of that quantity: the unit of the quantity of
    /// the line, as the standard has the two be the same, marked as worked
    /// out. None where no quantity the price is for is worked out, or the
    /// line has no unit the table knows.
    pub price_base_unit: Option<Stated>,
    /// The amount of the line.
    pub net: Part,
    /// The rate of VAT, where the table has a column for it.
    pub rate: Option<Part>,
    /// The code of the VAT category: as printed, or worked out from the
    /// rate and marked so.
    pub category: Option<Stated>,
}

/// A column of a table of lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Position,
    Description,
    Quantity,
    Price,
    Discount,
    Rate,
    Total,
    /// "Amount", which heads the amount of the line, or the quantity where
    /// the heading was translated from "Menge".
    Amount,
}

/// The names the columns of a table of lines are headed with, as
/// [`labels::normalized`] gives their words. The name of the item is headed
/// with one word.
const COLUMN_NAMES: &[(&[&str], Column)] = &[
    (&["pos"], Column::Position),
    (&["position"], Column::Position),
    (&["beschreibung"], Column::Description),
    (&["bezeichnung"], Column::Description),
    (&["produkt"], Column::Description),
    (&["artikel"], Column::Description),
    (&["leistung"], Column::Description),
    (&["description"], Column::Description),
    (&["product"], Column::Description),
    (&["désignation"], Column::Description),
    (&["menge"], Column::Quantity),
    (&["anzahl"], Column::Quantity),
    (&["quantity"], Column::Quantity),
    (&["qty"], Column::Quantity),
    (&["quantité"], Column::Quantity),
    (&["qté"], Column::Quantity),
    (&["preis"], Column::Price),
    (&["einzelpreis"], Column::Price),
    (&["nettopreis"], Column::Price),
    (&["stückpreis"], Column::Price),
    (&["price"], Column::Price),
    (&["unit", "price"], Column::Price),
    (&["prix", "unitaire"], Column::Price),
    (&["rabatt"], Column::Discount),
    (&["discount"], Column::Discount),
    (&["disc"], Column::Discount),
    (&["remise"], Column::Discount),
    (&["rem"], Column::Discount),
    (&["ust"], Column::Rate),
    (&["mwst"], Column::Rate),
    (&["steuersatz"], Column::Rate),
    (&["steuer"], Column::Rate),
    (&["vat"], Column::Rate),
    (&["tva"], Column::Rate),
    (&["total"], Column::Total),
    (&["netto"], Column::Total),
    (&["nettobetrag"], Column::Total),
    (&["nettowert"], Column::Total),
    (&["betrag"], Column::Total),
    (&["gesamtbetrag"], Column::Total),
    (&["gesamtpreis"], Column::Total),
    (&["gesamt"], Column::Total),
    (&["entgelt"], Column::Total),
    (&["summe"], Column::Total),
    (&["prix", "ht"], Column::Total),
    (&["montant"], Column::Total),
    (&["amount"], Column::Amount),
];

/// How many numbers make a line: the quantity, the price and the amount.
const LINE_FIGURES: usize = 3;

/// The quantities a price may be for without the page saying so: by the
/// hundred or the thousand. A price by the ten is none a page leaves unsaid.
const PRICE_BASES: [u32; 2] = [100, 1000];

/// The least amount of a line from which a price for a hundred or a
/// thousand is told: one unit of the currency. Below it nearly any quantity
/// and price come to the amount once divided and rounded.
const PRICE_BASE_AMOUNT_LEAST: Decimal = Decimal::ONE;

/// How far left of the heading of its column the name of an item may
/// start, as a multiple of the font size of the heading.
const NAME_SLACK: f64 = 1.0;

/// What the label of a field of its own below a name ends with, or is
/// followed by: "GTIN:", "Bruttopreis :".
const FIELD_ENDING: char = ':';

/// The characters a word broken at the end of its line ends with, as in
/// "Personen-" above "Unfallversicherung".
const WORD_BREAKS: [char; 2] = ['-', '/'];

/// What the heading of a table of lines says about its rows.
#[derive(Debug, Clone, PartialEq)]
struct Table {
    /// The columns the heading names, from left to right.
    columns: Vec<Column>,
    /// Where the names of the items start at the earliest, if a column
    /// stands left of theirs: what stands further left belongs to it.
    name_from: Option<f64>,
    /// Where the column right of the names starts, if one is named: what
    /// starts there or further right belongs to it.
    name_until: Option<f64>,
    /// How many words the heading holds after the name of the amount: the
    /// columns that numbers after the amount of a line may stand in. Zero
    /// where the amount is not named.
    after_amount: usize,
}

impl Table {
    /// Whether `word` starts in the column of the names.
    fn holds_name(&self, word: &Word) -> bool {
        self.name_from.is_none_or(|from| word.left >= from)
            && self.name_until.is_none_or(|until| word.left < until)
    }

    fn has(&self, column: Column) -> bool {
        self.columns.contains(&column)
    }

    fn place(&self, column: Column) -> Option<usize> {
        self.columns.iter().position(|other| *other == column)
    }

    /// Whether the quantity stands before the price: where the price is
    /// not named, it follows the quantity.
    fn quantity_first(&self) -> bool {
        self.place(Column::Price)
            .is_none_or(|price| self.place(Column::Quantity) < Some(price))
    }

    /// Whether the name of the item follows the quantity, where otherwise
    /// its unit stands.
    fn name_after_quantity(&self) -> bool {
        self.place(Column::Quantity)
            .and_then(|quantity| self.columns.get(quantity + 1))
            == Some(&Column::Description)
    }
}

/// The table the words of a row head, if they name the quantity of a line
/// once, and its price or its amount or both, each no more than once. A
/// heading set over two lines names one of the two on its other line.
///
/// | Row                                              | Table                                 |
/// |--------------------------------------------------|---------------------------------------|
/// | `Pos` `Produkt` `Beschreibung` `Menge` `Preis` `Netto` | quantity first, names from "Produkt" on |
/// | `Pos` `Beschreibung` `Nettopreis` `Menge` `Steuersatz` `Nettobetrag` | price first, names from "Beschreibung" on |
/// | `Amount` `Product` `VAT` `Price` `Total`         | "Amount" the quantity, the name after it |
/// | `Description` `Qty` `Unit` `Price` `Amount`      | "Amount" the amount, names from anywhere |
/// | `Description` `Quantité` `Rem.` `Prix` `HT`      | quantity first, the price not named   |
/// | `VAT` `Net` `amount` `VAT` `amount`              | none (no quantity, the amount twice)  |
/// | `Art` `Prozent` `Basisbetrag` `Steuer` `Betrag`  | none (no quantity)                    |
/// | `Menge` `Preis` `Preis` `Betrag`                 | none (the price twice)                |
/// | `Pos` `Beschreibung` `Nettopreis` `Menge` `USt.` | price first, the amount not named     |
/// | `Menge` `Preis` `Betrag` `Summe`                 | none (the amount twice)               |
/// | `Menge` `Lieferdatum`                            | none (neither price nor amount)       |
fn heading(words: &[&Word]) -> Option<Table> {
    let named = table::named(COLUMN_NAMES, words);
    let heads = |column: Column| named.iter().any(|(other, _)| *other == column);
    let translated = !heads(Column::Quantity) && heads(Column::Total);
    let columns: Vec<Column> = named
        .iter()
        .map(|(column, _)| match column {
            Column::Amount if translated => Column::Quantity,
            Column::Amount => Column::Total,
            other => *other,
        })
        .collect();
    let count = |column: Column| columns.iter().filter(|other| **other == column).count();
    let (prices, totals) = (count(Column::Price), count(Column::Total));
    if count(Column::Quantity) != 1 || prices > 1 || totals > 1 || prices + totals == 0 {
        return None;
    }
    let description = columns
        .iter()
        .position(|column| *column == Column::Description);
    let head_of = |place: usize| words.get(named.get(place)?.1.start).copied();
    let name_from = description
        .filter(|place| *place > 0)
        .and_then(head_of)
        .map(|head| head.left - head.font_size * NAME_SLACK);
    let name_until = description
        .and_then(|place| {
            (place + 1..columns.len()).find(|next| columns[*next] != Column::Description)
        })
        .and_then(head_of)
        .map(|head| head.left);
    let after_amount = columns
        .iter()
        .position(|column| *column == Column::Total)
        .and_then(|place| named.get(place))
        .map_or(0, |(_, name)| words.len().saturating_sub(name.end));
    Some(Table {
        columns,
        name_from,
        name_until,
        after_amount,
    })
}

/// A line found in a row, and the words of the row that are the name of
/// its item.
struct Found<'a> {
    line: Line,
    row: &'a Row<'a>,
    name: Vec<usize>,
}

/// The line a row of the table is: its last three numbers without the sign
/// of a rate are the quantity and the price, in the order of the heading,
/// and the amount, and they agree, with a printed discount taken off or
/// without. The number of the line is written as printed.
///
/// | Heading                   | Row                                                 | Read                                   |
/// |---------------------------|-----------------------------------------------------|----------------------------------------|
/// | `Pos` `Produkt` `Menge` `Preis` `Netto` | `02` `IP` `Server` `57811` `1` `0,8403` `€` `0,84` `€` | line 02, 1 at 0.8403, 0.84, "IP Server 57811" |
/// | `Pos` `Beschreibung` `Nettopreis` `Menge` `Steuersatz` `Nettobetrag` | `1` `Trennblätter` `9,99` `20` `Stk` `19` `%` `VAT` `(S)` `199,80` | line 1, 20 at 9.99, 199.80, 19 %, `S`, "Trennblätter" |
/// | `Amount` `Product` `VAT` `Price` `Total` | `1.00` `Design` `(hours)` `7%` `160.00` `€` `160.00` `€` | 1 at 160, 160, 7 %, `S` worked out, "Design (hours)" |
/// | `Description` `Quantité` `Rem.` `Prix` `HT` | `Nougat` `250g` `20` `Unit(s)` `4,55` `€` `10%` `81,90` `€` | 20 at a gross price of 4.55, net 4.095 worked out, 81.90, "Nougat 250g" |
/// | `Description` `Quantité` `Rem.` `Prix` `HT` | `Biscuits` `15` `Unit(s)` `3,20` `€` `48,00` `€` | 15 at 3.20, 48, "Biscuits" |
/// | `Text` `Menge` `Preis` `Betrag` | `Nougat` `20` `4,55` `10%` `81,90`            | none (no column for a discount)        |
/// | `Text` `Menge` `Preis` `Betrag` | `Miete` `1` `100,00` `120,00`                 | none (do not agree)                    |
/// | `Text` `Menge` `Preis` `Betrag` | `Miete` `100,00` `100,00`                     | none (two numbers)                     |
/// | `Text` `Menge` `Preis` `Betrag` | `Summe` `1` `100,00` `19%`                    | none (two numbers)                     |
/// | `Text` `Menge` `Preis` `Betrag` `USt.` | `Miete` `1` `100,00` `100,00` `7%` `19%` | 1 at 100, 100, no rate (two printed)   |
///
/// A number whose one separator could group thousands or mark decimals is
/// read the way with which the row agrees, if only one does:
///
/// | Heading                         | Row                                | Read                        |
/// |---------------------------------|------------------------------------|-----------------------------|
/// | `Text` `Preis` `Menge` `Betrag` | `Strom` `0,0520` `1.000` `kWh` `52,00` | 1000 at 0.052, 52       |
/// | `Text` `Menge` `Preis` `Betrag` | `Gas` `1.000` `kWh` `52,00` `52,00` | 1 at 52, 52                |
/// | `Text` `Menge` `Preis` `Betrag` | `Gas` `1.000` `0,00` `0,00`        | none (both readings agree)  |
/// | `Text` `Menge` `Preis` `Betrag` | `Gas` `1.000` `5,00` `70,00`       | none (neither agrees)       |
/// | `Text` `Menge` `Preis` `Betrag` | `Gas` `1.000` `kWh` `52,00` `52,00`, read as a thousand at a price for a thousand | not that reading: it needs a price base besides |
fn line<'a>(row: &'a Row<'a>, table: &Table, proof: Proof) -> Option<Found<'a>> {
    let strict = figures(&row.words);
    let as_printed = line_of(row, table, &strict, Printed::WithPrice(proof));
    let either_way = || {
        let read = |open: Reading| {
            line_of(
                row,
                table,
                &figures_as(&row.words, open),
                Printed::WithPrice(proof),
            )
        };
        // A reading that needs a price for a hundred besides is one guess
        // on another: only a reading that agrees as it stands counts.
        let as_it_stands =
            |open: Reading| read(open).filter(|found| found.line.price_base.is_none());
        match (
            as_it_stands(Reading::Thousands),
            as_it_stands(Reading::Decimals),
        ) {
            (Some(line), None) | (None, Some(line)) => Some(line),
            (Some(_), Some(_)) | (None, None) => None,
        }
    };
    as_printed.or_else(either_way)
}

/// The line a row is that prints no price, if it has the shape of one.
fn line_without_price<'a>(row: &'a Row<'a>, table: &Table) -> Option<Found<'a>> {
    line_of(row, table, &figures(&row.words), Printed::WithoutPrice)
}

/// What bears a line out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Proof {
    /// The numbers of its row: the quantity times the price, less a printed
    /// discount, is the amount.
    Row,
    /// Only the sum of the lines: its row agrees with a price for a hundred
    /// or a thousand, which the page does not print, or prints no price.
    Sum,
}

/// Whether a row is read as a line that prints its price, with what bears
/// it out, or as one without.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Printed {
    WithPrice(Proof),
    WithoutPrice,
}

/// The numbers of a row that make a line.
struct Numbers<'f> {
    quantity: &'f Figure,
    /// The price, where the row prints one.
    price: Option<&'f Figure>,
    net: &'f Figure,
    /// The discount taken off, if the numbers agree only with it.
    discount: Option<&'f Figure>,
    /// The quantity the price is for, if the numbers agree only with it.
    base: Option<Decimal>,
}

/// The quantity a price is for if `quantity` at `price` comes to `net` only
/// so: a hundred or a thousand, as screws are priced by the hundred. The
/// page does not print it. An amount below one unit of the currency tells
/// nothing: nearly anything comes to it once divided and rounded.
///
/// | Quantity | Price | Amount | The price is for          |
/// |----------|-------|--------|---------------------------|
/// | 400      | 32.76 | 131.04 | 100                       |
/// | -150     | 60.76 | -91.14 | 100                       |
/// | 150      | 49.29 | 73.94  | 100 (73.935, rounded)     |
/// | 1000     | 5.20  | 5.20   | 1000                      |
/// | 1000     | 0.052 | 5.20   | none (a price for ten)    |
/// | 400      | 32.76 | 131.05 | none (not to the cent)    |
/// | 400      | 32.76 | 262.08 | none (a price for 50)     |
/// | 3        | 3.00  | 0.01   | none (an amount too small to tell) |
/// | 1        | 2.00  | 0.00   | none (an amount of nothing) |
fn price_base(quantity: Decimal, price: Decimal, net: Decimal) -> Option<Decimal> {
    if net.abs() < PRICE_BASE_AMOUNT_LEAST {
        return None;
    }
    let gross = quantity.checked_mul(price)?;
    PRICE_BASES.into_iter().map(Decimal::from).find(|base| {
        gross.checked_div(*base).is_some_and(|shared| {
            let rounded = shared.round_dp_with_strategy(
                AMOUNT_DECIMAL_PLACES,
                RoundingStrategy::MidpointAwayFromZero,
            );
            rounded == net
        })
    })
}

/// The numbers among those of a row that make a line: the last three
/// standing together, none with the sign of a rate, that are the quantity
/// and the price, in the order of the heading, and the amount, and agree:
/// as they stand, with one of the printed rates taken off as a discount, or,
/// where the sum of the lines is to bear them out, with the price being one
/// for a hundred or a thousand and the quantity having its unit after it.
/// What stands after them is another column, as the tax or the amount with
/// it, and needs a word of the heading after the name of the amount.
///
/// | Numbers of the row, quantity first | Rates       | The line                          |
/// |------------------------------------|-------------|-----------------------------------|
/// | 1, 100, 100                        | none        | 1 at 100, 100                     |
/// | 2, 57811, 1, 0.8403, 0.84          | none        | 1 at 0.8403, 0.84 (the last three) |
/// | 1, 100, 100, 19, 119, two columns after the amount | none | 1 at 100, 100 (tax and gross amount after) |
/// | 5, 1, 5.00, 7.00, no column after the amount | none | none (5, 1 and 5.00 agree, but 7.00 has no column) |
/// | 5, 1, 5.00, 7.00, one column after the amount | 19 % after them | none (two numbers for one column) |
/// | 20, 4.55, 81.90                    | 10 %, a column for discounts | 20 at 4.55, 81.90, less 10 % |
/// | 20, 4.55, 81.90                    | 10 %, no such column | none                     |
/// | 400, 32.76, 131.04, to be borne out by the sum | none | 400 at 32.76 for 100, 131.04 |
/// | 400, 32.76, 131.04, to be borne out in the row | none | none                     |
/// | 2, 100, 2.00, to be borne out by the sum, no unit after the 2 | none | none (2 per cent of 100, the sign not printed) |
/// | 1, 100, 120                        | none        | none (do not agree)               |
/// | 100, 100                           | none        | none (two numbers)                |
/// | 1, 100, 0.025 (no amount of money) | none        | none                              |
fn numbers<'f>(
    plain: &[&'f Figure],
    marked: &[&'f Figure],
    table: &Table,
    proof: Proof,
) -> Option<Numbers<'f>> {
    let windows = plain.windows(LINE_FIGURES).rev().enumerate();
    windows.into_iter().find_map(|(passed_over, window)| {
        let [first, second, net] = window else {
            return None;
        };
        // Numbers after the amount need columns of their own: without them
        // three numbers further left that happen to agree are no line.
        let rates_after = marked.iter().filter(|rate| rate.first > net.first).count();
        if passed_over > 0 && passed_over + rates_after > table.after_amount {
            return None;
        }
        let (quantity, price) = if table.quantity_first() {
            (*first, *second)
        } else {
            (*second, *first)
        };
        if !net.monetary {
            return None;
        }
        let gross = quantity.value.checked_mul(price.value);
        let taken_off = |rate: &Figure| {
            let share = Decimal::ONE_HUNDRED.checked_sub(rate.value)?;
            gross?.checked_mul(share)?.checked_div(Decimal::ONE_HUNDRED)
        };
        let as_they_stand = comes_to(gross, net.value);
        let discount = marked
            .iter()
            .copied()
            .filter(|_| !as_they_stand && table.has(Column::Discount))
            .find(|rate| comes_to(taken_off(rate), net.value));
        // A quantity priced by the hundred has its unit after it. A number
        // with another number after it may as well be a percentage printed
        // without its sign, as the 2 in "Skonto 2 100,00 2,00".
        let after_quantity = quantity.first + quantity.count;
        let unit_follows = plain
            .iter()
            .chain(marked)
            .all(|figure| figure.first != after_quantity);
        let base = (!as_they_stand && discount.is_none() && proof == Proof::Sum && unit_follows)
            .then(|| price_base(quantity.value, price.value, net.value))
            .flatten();
        (as_they_stand || discount.is_some() || base.is_some()).then_some(Numbers {
            quantity,
            price: Some(price),
            net,
            discount,
            base,
        })
    })
}

/// The numbers of a row that prints a line without its price: a quantity,
/// its unit, then nothing but a rate or the mark of a currency, and the
/// amount, printed with decimals, as the last word of the row; and no other
/// number but the number of the line. The sum of the lines bears the amount
/// out, and the unit the quantity: the word after it must be a unit of
/// measure the table of units knows, or the number before it is no
/// quantity. No price is worked out for it: the page may print
/// one elsewhere, with allowances taken off the line, and the amount by the
/// quantity would speak against it.
///
/// | Row, the heading with positions                | The line                     |
/// |------------------------------------------------|------------------------------|
/// | `1` `Stahlcoil` `1` `Stk` `0%` `1.000,00`      | line 1, 1, 1000              |
/// | `Stahlcoil` `10` `Stk` `19%` `850,00` `€`      | 10, 850                      |
/// | `Stahlcoil` `10` `Stk` `EUR` `850,00`          | 10, 850                      |
/// | `Stahlcoil` `10` `Rollen` `850,00`             | none (no unit the table knows: nothing bears the quantity out) |
/// | `Bauteil` `2026` `Version` `850,00`            | none (the same)              |
/// | `2` `Inklusiv-Kilometer` `waren:` `873`        | none (a word after the unit, an amount without decimals) |
/// | `2` `Kilometer` `gefahren` `873,00`            | none (a word after the unit) |
/// | `2` `Kilometer` `873`                          | none (an amount without decimals) |
/// | `3` `Monteur` `10,50` `28` `Std` `288,75`      | none (a price that does not agree) |
/// | `77777777777` `200`                            | none (no unit)               |
/// | `Strom` `0,0520` `1.000` `kWh` `52,00`         | none (`1.000` is a number, no unit) |
/// | `4260331811362` `(GLN)` `1250032` `D.` `Muster` | none (the amount not last)  |
/// | `2` `Versand` `4,90`                           | none (`2` is the number of the line) |
/// | `Garantie` `24` `Monate` `0,00`                | none (an amount of nothing, which no sum bears out) |
/// | `Stahlcoil` `0` `Stk` `850,00`                 | none (a quantity of nothing) |
/// | `Stahlcoil` `10` `Stk` `0,025`                 | none (no amount of money)    |
fn without_price<'f>(row: &Row, figures: &'f [Figure], table: &Table) -> Option<Numbers<'f>> {
    let plain: Vec<&Figure> = figures.iter().filter(|figure| !figure.marked).collect();
    let numbered = table.has(Column::Position)
        && plain
            .first()
            .is_some_and(|head| head.first == 0 && head.value.fract().is_zero());
    let (quantity, net) = match plain.as_slice() {
        [quantity, net] => (*quantity, *net),
        [_, quantity, net] if numbered => (*quantity, *net),
        _ => return None,
    };
    let unit = quantity.first + quantity.count;
    let covered = |word: usize| {
        figures
            .iter()
            .any(|figure| (figure.first..figure.first + figure.count).contains(&word))
    };
    let is_mark = |word: usize| values::currency_mark(&row.words[word].text).is_some();
    let unit_follows = unit < net.first
        && !covered(unit)
        && values::open_number(&row.words[unit].text).is_none()
        && units::code(&row.words[unit].text).is_some();
    let only_rates_between = (unit + 1..net.first).all(|word| covered(word) || is_mark(word));
    let last_word = (net.first + net.count..row.words.len()).all(is_mark);
    let amount = net.monetary && values::has_decimals(&row.words[net.first].text);
    // The first word of a row is the number of the line or the start of
    // the name, unless the heading puts the quantity first.
    let in_its_place =
        quantity.first > 0 || (table.name_after_quantity() && !table.has(Column::Position));
    let fits = unit_follows && only_rates_between && last_word && amount && in_its_place;
    (fits && !quantity.value.is_zero() && !net.value.is_zero()).then_some(Numbers {
        quantity,
        price: None,
        net,
        discount: None,
        base: None,
    })
}

/// The line the numbers `figures` of a row make: those of a line with its
/// price, if they agree, or those of a line printed without it.
fn line_of<'a>(
    row: &'a Row<'a>,
    table: &Table,
    figures: &[Figure],
    printed: Printed,
) -> Option<Found<'a>> {
    let (plain, marked): (Vec<&Figure>, Vec<&Figure>) =
        figures.iter().partition(|figure| !figure.marked);
    let Numbers {
        quantity,
        price,
        net,
        discount,
        base,
    } = match printed {
        Printed::WithPrice(proof) => numbers(&plain, &marked, table, proof)?,
        Printed::WithoutPrice => without_price(row, figures, table)?,
    };
    let rates: Vec<&Figure> = marked
        .iter()
        .copied()
        .filter(|rate| discount.is_none_or(|discount| discount.first != rate.first))
        .collect();
    let rate = match rates.as_slice() {
        [rate] if table.has(Column::Rate) => Some(part(row, rate)),
        _ => None,
    };
    let mut taken: Vec<&Figure> = vec![quantity, net];
    taken.extend(price);
    let identifier = figures.first().filter(|head| {
        table.has(Column::Position)
            && head.first == 0
            && !head.marked
            && head.value.fract().is_zero()
            && taken.iter().all(|taken| taken.first != head.first)
    });
    taken.extend(identifier);
    taken.extend(marked.iter().copied());
    let after_quantity = quantity.first + quantity.count;
    let unit = (!table.name_after_quantity()
        && !figures.iter().any(|figure| figure.first == after_quantity))
    .then_some(after_quantity);
    // The name stands before the price and the amount; where no price is
    // printed, before the quantity too, unless the heading has it after.
    let before = match price {
        Some(price) => price.first.min(net.first),
        None if table.name_after_quantity() => net.first,
        None => quantity.first.min(net.first),
    };
    let name = name(row, &taken, unit, before, table);
    let named = Span {
        page: row.page,
        words: name
            .iter()
            .filter_map(|word| row.words.get(*word).copied())
            .collect(),
    };
    // A code of a category is looked for beside the name, not in it: a size
    // in brackets, as in "T-Shirt (M)", is none.
    let beside_name: Vec<&Word> = row
        .words
        .iter()
        .enumerate()
        .filter(|(word, _)| !name.contains(word))
        .map(|(_, word)| *word)
        .collect();
    let (quantity, net) = (part(row, quantity), part(row, net));
    let printed_price = price.map(|price| part(row, price));
    let unit = unit.and_then(|word| {
        let printed = row.span(word, 1)?;
        let code = units::code(printed.texts().first()?)?;
        Some(Stated {
            text: code.to_owned(),
            location: printed.location(0, 1),
            origin: Origin::Derived,
        })
    });
    let line = Line {
        identifier: identifier.map(|head| Part {
            text: row.words[head.first].text.clone(),
            ..part(row, head)
        }),
        name: (!name.is_empty()).then(|| Stated {
            text: named.texts().join(" "),
            location: named.location(0, named.words.len()),
            origin: Origin::Rule,
        }),
        price: match (&printed_price, discount) {
            (Some(price), None) => Some(price.clone()),
            (Some(price), Some(discount)) => after_discount(price, discount),
            (None, _) => None,
        },
        gross_price: printed_price.clone().filter(|_| discount.is_some()),
        price_base: base.zip(printed_price).map(|(base, price)| Part {
            text: base.normalize().to_string(),
            value: base,
            origin: Origin::Derived,
            ..price
        }),
        price_base_unit: unit.clone().filter(|_| base.is_some()),
        unit,
        quantity,
        net,
        category: table::printed_category(row.page, &beside_name)
            .or_else(|| rate.as_ref().and_then(table::told_by_rate)),
        rate,
    };
    Some(Found { line, row, name })
}

/// The net price that is left of a gross price with a discount taken off,
/// marked as worked out and placed at the gross price. It is not rounded:
/// only so does the quantity times the net price come to the amount the
/// page prints, which is what was checked before.
///
/// | Gross price | Discount | Net price           |
/// |-------------|----------|---------------------|
/// | 4.55        | 10 %     | 4.095, worked out   |
/// | 3.33        | 5 %      | 3.1635, worked out  |
/// | 100         | 0 %      | 100, worked out     |
/// | 100         | 100 %    | 0, worked out       |
/// | too large to multiply |  | none              |
fn after_discount(gross: &Part, discount: &Figure) -> Option<Part> {
    let share = Decimal::ONE_HUNDRED.checked_sub(discount.value)?;
    let value = gross
        .value
        .checked_mul(share)?
        .checked_div(Decimal::ONE_HUNDRED)?;
    Some(Part {
        text: value.normalize().to_string(),
        value,
        origin: Origin::Derived,
        ..gross.clone()
    })
}

/// The words of a row that are the name of the item of its line: the words
/// before its price and its amount that are none of its numbers, no mark of
/// a currency and not its unit, and that stand in the column of the names,
/// if one of them holds a letter. A number among them belongs to the name.
///
/// | Heading                  | Row                                         | Name                    |
/// |--------------------------|---------------------------------------------|-------------------------|
/// | `Pos` `Text` `Menge` `Preis` `Betrag` | `2` `IP` `Server` `57811` `1` `0,84` `0,84` | "IP Server 57811" |
/// | `Text` `Menge` `Preis` `Betrag` | `Nougat` `250g` `20` `Unit(s)` `4,55` `91,00` | "Nougat 250g"  |
/// | `Amount` `Product` `Price` `Total` | `1.00` `Design` `(hours)` `160.00` `160.00` | "Design (hours)" |
/// | `Pos` `Text` `Preis` `Menge` `USt.` `Betrag` | `1` `Papier` `9,99` `20` `Stk` `19%` `VAT` `(S)` `199,80` | "Papier" ("VAT (S)" stands after the price) |
/// | `Text` `Menge` `Preis` `Betrag` | `1` `Stk` `100,00` `100,00`          | none ("Stk" is the unit) |
/// | `Text` `Menge` `Preis` `Betrag` | `1` `100,00` `€` `100,00` `€`        | none                    |
/// | `Text` `Menge` `Preis` `Betrag` | `2026` `1` `100,00` `100,00`         | none (no letter)        |
/// | `Pos` `Beschreibung` `Menge` `Preis` `Betrag`, a reference left of "Beschreibung" | reference, name, numbers | the name without the reference |
fn name(
    row: &Row,
    taken: &[&Figure],
    unit: Option<usize>,
    before: usize,
    table: &Table,
) -> Vec<usize> {
    let covers =
        |figure: &Figure, word: usize| (figure.first..figure.first + figure.count).contains(&word);
    let words: Vec<usize> = (0..before.min(row.words.len()))
        .filter(|word| {
            !taken.iter().any(|figure| covers(figure, *word))
                && unit != Some(*word)
                && values::currency_mark(&row.words[*word].text).is_none()
                && table.holds_name(row.words[*word])
        })
        .collect();
    let lettered = words
        .iter()
        .any(|word| row.words[*word].text.chars().any(char::is_alphabetic));
    if lettered { words } else { Vec::new() }
}

/// The lines a name runs on over below its first word: the lines of its
/// block that follow below it in the column of the names, above the row of
/// the next line of the invoice, up to one that holds a field of its own or
/// ends the table. A line of the block on the baseline of the name, or in
/// another column, belongs to a cell beside it.
///
/// | Below "Wartung" in its block               | Runs on over        |
/// |--------------------------------------------|---------------------|
/// | "technischer Anlagen"                      | that line           |
/// | "technischer", "Anlagen"                   | both lines          |
/// | "GTIN: 4000050986428"                      | nothing (a field)   |
/// | "Bruttopreis : 43,20"                      | nothing (a field)   |
/// | "Anlagen", "GTIN: 4000050986428"           | "Anlagen"           |
/// | "Summe netto 120,00"                       | nothing (a total)   |
/// | "Strom", on the row of the next line       | nothing             |
/// | "2 Monate" on the baseline of "Wartung", then "Anlagen" | "Anlagen" (the cell beside it passed over) |
/// | "Anlagen", then "Monate" in the column right of the names | "Anlagen" |
/// | nothing, or a line of another block        | nothing             |
fn runs_on<'a>(
    printed: &[page::Line<'a>],
    first: &Word,
    next: Option<&Word>,
    table: &Table,
) -> Vec<&'a [Word]> {
    let own = printed
        .iter()
        .position(|line| line.words.iter().any(|word| std::ptr::eq(word, first)));
    let Some(own) = own else {
        return Vec::new();
    };
    printed[own + 1..]
        .iter()
        .take_while(|line| line.page == printed[own].page && line.block == printed[own].block)
        .filter(|line| {
            line.words
                .first()
                .is_some_and(|head| page::lies_below(head, first) && table.holds_name(head))
        })
        .take_while(|line| {
            let words: Vec<&Word> = line.words.iter().collect();
            let above_next = words
                .first()
                .is_some_and(|head| next.is_none_or(|next| page::lies_below(next, head)));
            let field = words.iter().any(|word| word.text.ends_with(FIELD_ENDING));
            above_next && !field && !names_total(&words) && !breakdown::heads_table(&words)
        })
        .map(|line| line.words)
        .collect()
}

/// A name with the words of the line it runs on over. A word broken at the
/// end of its line is put together again.
///
/// | Name so far             | Next line            | Together                      |
/// |-------------------------|----------------------|-------------------------------|
/// | "Wartung"               | "technischer Anlagen" | "Wartung technischer Anlagen" |
/// | "Personen-"             | "Unfallversicherung" | "Personen-Unfallversicherung" |
/// | "Gartenpflege/"         | "Schneeräum."        | "Gartenpflege/Schneeräum."    |
/// | "Navigationssystem -"   | "Garantie"           | "Navigationssystem - Garantie" |
fn joined(name: &str, more: &[Word]) -> String {
    let broken = name
        .split_whitespace()
        .next_back()
        .is_some_and(|last| last.ends_with(WORD_BREAKS) && last.chars().count() > 1);
    let more: Vec<&str> = more.iter().map(|word| word.text.as_str()).collect();
    let gap = if broken { "" } else { " " };
    format!("{name}{gap}{}", more.join(" "))
}

/// Whether the words of a row hold the label of a total of the invoice.
/// Other labels are no sign of the totals: lines print references of their
/// own.
///
/// | Words of the row           | A total |
/// |----------------------------|---------|
/// | `Positionssumme` `320,00`  | yes     |
/// | `Net` `total:` `496.00`    | yes     |
/// | `Bestell-Nr.:` `4711`      | no      |
/// | `~2000ml`                  | no      |
fn names_total(words: &[&Word]) -> bool {
    let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
    labels::find(&texts)
        .iter()
        .any(|found| matches!(found.names, Names::Term(term) if TOTALS.contains(&term)))
}

/// Whether the words of a row end a table of lines whose amounts so far add
/// up to `sum`: the heading of a breakdown, or the label of a total with
/// that sum beside it. A label of a total with another amount is a property
/// of an item, as the whole of a cost that is shared out.
///
/// | Words of the row                    | Sum so far | Ends the table           |
/// |-------------------------------------|------------|--------------------------|
/// | `Positionssumme` `320,00`           | 320        | yes                      |
/// | `Net` `total:` `496.00` `€`         | 496        | yes                      |
/// | `Gesamtbetrag:` `1.025,29` `€`      | 320        | no (a property of an item) |
/// | `Positionssumme`                    | 320        | no (no amount)           |
/// | `VAT` `Net` `amount` `VAT` `amount` | any        | yes (a breakdown)        |
/// | `Bestell-Nr.:` `4711`               | 4711       | no (no total)            |
fn ends_table(words: &[&Word], sum: Decimal) -> bool {
    let sums_up = || {
        figures(words)
            .iter()
            .any(|figure| !figure.marked && figure.value == sum)
    };
    breakdown::heads_table(words) || (names_total(words) && sums_up())
}

/// The lines of the table that the row at `at` heads: the rows below it
/// that make one, up to the row that ends the table, each with the lines its
/// name runs on over. A table runs on over the pages, where its heading may
/// be printed again.
fn table_lines(
    rows: &[Row],
    at: usize,
    table: &Table,
    printed: &[page::Line],
    proof: Proof,
    sums: &[Decimal],
) -> Vec<Line> {
    let mut found: Vec<Found> = Vec::new();
    let mut sum = Decimal::ZERO;
    for row in rows.iter().skip(at + 1) {
        // A row is tried as a line without a price only while the lines do
        // not add up to a total yet, and not if it ends the table: the row
        // of a sum has the shape of such a line.
        let ends = heading(&row.words).is_none() && ends_table(&row.words, sum);
        let line = line(row, table, proof).or_else(|| {
            (proof == Proof::Sum && !ends && !sums.contains(&sum))
                .then(|| line_without_price(row, table))
                .flatten()
        });
        if let Some(line) = line {
            sum = sum.checked_add(line.line.net.value).unwrap_or(sum);
            found.push(line);
        } else if ends {
            break;
        }
    }
    // The row of the next line on the same page stands below the name and
    // limits it; on the next page nothing does.
    let limits: Vec<Option<&Word>> = found
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let next = found.get(index + 1)?.row;
            (next.page == line.row.page)
                .then(|| next.words.first().copied())
                .flatten()
        })
        .collect();
    found
        .into_iter()
        .zip(limits)
        .map(|(mut line, next)| {
            let first = line
                .name
                .first()
                .and_then(|start| line.row.words.get(*start));
            if let (Some(first), Some(name)) = (first, line.line.name.as_mut()) {
                for more in runs_on(printed, first, next, table) {
                    let span = Span {
                        page: line.row.page,
                        words: more.iter().collect(),
                    };
                    name.text = joined(&name.text, more);
                    name.location = around(name.location, span.location(0, more.len()));
                }
            }
            line.line
        })
        .collect()
}

/// The place that holds both places, of which either may be missing.
///
/// | First                | Second               | Around both           |
/// |----------------------|----------------------|-----------------------|
/// | 50 to 90, 90 to 100  | 50 to 120, 102 to 112 | 50 to 120, 90 to 112 |
/// | a place              | none                 | the first             |
/// | none                 | a place              | the second            |
/// | none                 | none                 | none                  |
fn around(first: Option<Location>, second: Option<Location>) -> Option<Location> {
    match (first, second) {
        (Some(first), Some(second)) => Some(Location {
            page: first.page,
            left: first.left.min(second.left),
            top: first.top.min(second.top),
            right: first.right.max(second.right),
            bottom: first.bottom.max(second.bottom),
        }),
        (place, None) | (None, place) => place,
    }
}

/// Whether the amounts of `lines` add up to one of `sums`.
fn add_up(lines: &[Line], sums: &[Decimal]) -> bool {
    let sum = lines
        .iter()
        .try_fold(Decimal::ZERO, |sum, line| sum.checked_add(line.net.value));
    sum.is_some_and(|sum| sums.contains(&sum))
}

/// The lines printed on `rows`, whose words stand in the lines `printed`:
/// those of the first table whose lines add up to one of `totals`, the sum
/// of the lines or the total without VAT as read from the page. Lines that
/// add up to neither are not the lines of the invoice: one is missing, or a
/// row below the table was taken for one.
///
/// | Lines found in a table   | Totals read              | Read  |
/// |--------------------------|--------------------------|-------|
/// | 100 and 20               | sum of the lines 120     | both  |
/// | 100 and 20               | total without VAT 120    | both  |
/// | 100 and 20               | 300 (a line was missed)  | none  |
/// | 100, 20 and 5 (a row below the totals) | 120        | none  |
/// | 100 and 20               | none                     | none  |
/// | none                     | any                      | none  |
///
/// The tables are read twice. First only rows count whose numbers bear each
/// other out. If the lines of no table add up so, rows count too that only
/// the sum can bear out: with a price for a hundred or a thousand, or
/// without a price. A table that is read the first way is never read the
/// second.
///
/// | Rows of the table                              | Totals read | Read                         |
/// |------------------------------------------------|-------------|------------------------------|
/// | 1 at 100 is 100; "Zahlbar in 14 Tagen 119,00"  | 100         | the one line                 |
/// | 400 at 32.76 is 131.04                         | 131.04      | it, the price for 100        |
/// | 1 at 100 is 100; 10 Stk, 850.00, no price      | 950         | both                         |
/// | 1 at 100 is 100; "Summe 2 Positionen 100,00"   | 100         | the one line                 |
/// | 1 at 100; 10 Stk, 850.00; "Summe 10 Positionen 950,00" | 950 | the two lines (they add up before the row of the sum) |
pub fn read(rows: &[Row], printed: &[page::Line], totals: [Option<&Entry>; 2]) -> Vec<Line> {
    let sums: Vec<Decimal> = totals
        .iter()
        .flatten()
        .filter_map(|total| Decimal::from_str(&total.text).ok())
        .collect();
    let read_with = |proof: Proof| {
        rows.iter()
            .enumerate()
            .filter_map(|(at, row)| {
                heading(&row.words)
                    .map(|table| table_lines(rows, at, &table, printed, proof, &sums))
            })
            .find(|found| !found.is_empty() && add_up(found, &sums))
    };
    read_with(Proof::Row)
        .or_else(|| read_with(Proof::Sum))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use doc2einvoice_model::{Entry, Location, Origin};
    use doc2einvoice_pdf::Word;
    use rust_decimal::Decimal;

    use super::{
        Column, Found, Line, Proof, Table, after_discount, around, ends_table, heading, joined,
        line, line_without_price, names_total, price_base, read, runs_on,
    };
    use crate::page::{self, Row};
    use crate::table::printed::{LEFT, PITCH, TOP, row, stacked, words};
    use crate::table::{Part, Stated, figures, part};

    /// The line a row is, however it is borne out.
    fn any_line<'a>(row: &'a Row<'a>, table: &Table) -> Option<Found<'a>> {
        line(row, table, Proof::Sum).or_else(|| line_without_price(row, table))
    }

    /// The table `texts` head.
    fn heads(texts: &[&str]) -> Option<Table> {
        let printed = words(LEFT, TOP, texts);
        heading(&row(&printed).words)
    }

    /// Whether the quantity stands first, whether the name follows it, and
    /// whether the names start at a column.
    type Shape = (bool, bool, bool);

    fn shape(table: &Table) -> Shape {
        (
            table.quantity_first(),
            table.name_after_quantity(),
            table.name_from.is_some(),
        )
    }

    #[test]
    fn headings_follow_their_table() {
        let table: [(&[&str], Option<Shape>); 11] = [
            (
                &["Pos", "Produkt", "Beschreibung", "Menge", "Preis", "Netto"],
                Some((true, false, true)),
            ),
            (
                &[
                    "Pos",
                    "Beschreibung",
                    "Nettopreis",
                    "Menge",
                    "Steuersatz",
                    "Nettobetrag",
                ],
                Some((false, false, true)),
            ),
            (
                &["Amount", "Product", "VAT", "Price", "Total"],
                Some((true, true, true)),
            ),
            (
                &["Description", "Qty", "Unit", "Price", "Amount"],
                Some((true, false, false)),
            ),
            (
                &["Description", "Quantité", "Rem.", "Prix", "HT"],
                Some((true, false, false)),
            ),
            (&["VAT", "Net", "amount", "VAT", "amount"], None),
            (&["Art", "Prozent", "Basisbetrag", "Steuer", "Betrag"], None),
            (&["Menge", "Preis", "Preis", "Betrag"], None),
            (
                &["Pos", "Beschreibung", "Nettopreis", "Menge", "USt."],
                Some((false, false, true)),
            ),
            (&["Menge", "Preis", "Betrag", "Summe"], None),
            (&["Menge", "Lieferdatum"], None),
        ];
        for (texts, expected) in table {
            assert_eq!(heads(texts).as_ref().map(shape), expected, "{texts:?}");
        }
        let translated = heads(&["Amount", "Product", "VAT", "Price", "Total"]);
        assert_eq!(
            translated.map(|table| table.columns),
            Some(vec![
                Column::Quantity,
                Column::Description,
                Column::Rate,
                Column::Price,
                Column::Total
            ])
        );
        let plain = heads(&["Description", "Qty", "Unit", "Price", "Amount"]);
        assert_eq!(
            plain.map(|table| table.columns),
            Some(vec![
                Column::Description,
                Column::Quantity,
                Column::Price,
                Column::Total
            ])
        );
    }

    /// The number, the name, the quantity, the price, the amount, the rate
    /// and the category of a line.
    type Values = [Option<String>; 7];

    fn values(line: &Line) -> Values {
        [
            line.identifier.as_ref().map(|part| part.text.clone()),
            line.name.as_ref().map(|name| name.text.clone()),
            Some(line.quantity.text.clone()),
            line.price.as_ref().map(|part| part.text.clone()),
            Some(line.net.text.clone()),
            line.rate.as_ref().map(|part| part.text.clone()),
            line.category.as_ref().map(|code| code.text.clone()),
        ]
    }

    /// The values of the line that `texts` are under `heading`. The names
    /// start anywhere: the words of a test stand closer than its columns.
    fn read_line(heading: &[&str], texts: &[&str]) -> Option<Values> {
        let table = Table {
            name_from: None,
            name_until: None,
            ..heads(heading)?
        };
        let printed = words(LEFT, TOP, texts);
        any_line(&row(&printed), &table).map(|found| values(&found.line))
    }

    fn expected(values: [Option<&str>; 7]) -> Values {
        values.map(|value| value.map(str::to_owned))
    }

    #[test]
    fn lines_that_agree_follow_their_table() {
        assert_eq!(
            read_line(
                &["Pos", "Produkt", "Menge", "Preis", "Netto"],
                &[
                    "02", "IP", "Server", "57811", "1", "0,8403", "€", "0,84", "€"
                ],
            ),
            Some(expected([
                Some("02"),
                Some("IP Server 57811"),
                Some("1"),
                Some("0.8403"),
                Some("0.84"),
                None,
                None,
            ]))
        );
        assert_eq!(
            read_line(
                &[
                    "Pos",
                    "Beschreibung",
                    "Nettopreis",
                    "Menge",
                    "Steuersatz",
                    "Nettobetrag"
                ],
                &[
                    "1",
                    "Trennblätter",
                    "9,99",
                    "20",
                    "Stk",
                    "19",
                    "%",
                    "VAT",
                    "(S)",
                    "199,80"
                ],
            ),
            Some(expected([
                Some("1"),
                Some("Trennblätter"),
                Some("20"),
                Some("9.99"),
                Some("199.8"),
                Some("19"),
                Some("S"),
            ]))
        );
        assert_eq!(
            read_line(
                &["Amount", "Product", "VAT", "Price", "Total"],
                &[
                    "1.00", "Design", "(hours)", "7%", "160.00", "€", "160.00", "€"
                ],
            ),
            Some(expected([
                None,
                Some("Design (hours)"),
                Some("1"),
                Some("160"),
                Some("160"),
                Some("7"),
                Some("S"),
            ]))
        );
    }

    #[test]
    fn lines_with_a_discount_or_without_agreement_follow_their_table() {
        const FRENCH: &[&str] = &["Description", "Quantité", "Rem.", "Prix", "HT"];
        const PLAIN: &[&str] = &["Text", "Menge", "Preis", "Betrag"];
        assert_eq!(
            read_line(
                FRENCH,
                &[
                    "Nougat", "250g", "20", "Unit(s)", "4,55", "€", "10%", "81,90", "€"
                ],
            ),
            Some(expected([
                None,
                Some("Nougat 250g"),
                Some("20"),
                Some("4.095"),
                Some("81.9"),
                None,
                None
            ]))
        );
        assert_eq!(
            read_line(
                FRENCH,
                &["Biscuits", "15", "Unit(s)", "3,20", "€", "48,00", "€"]
            ),
            Some(expected([
                None,
                Some("Biscuits"),
                Some("15"),
                Some("3.2"),
                Some("48"),
                None,
                None
            ]))
        );
        let discounted = heads(FRENCH).and_then(|table| {
            let printed = words(
                LEFT,
                TOP,
                &["Nougat", "20", "4,55", "€", "10%", "81,90", "€"],
            );
            any_line(&row(&printed), &table).map(|found| found.line)
        });
        let prices = discounted.map(|line| {
            let origin = |price: Option<Part>| price.map(|price| (price.text, price.origin));
            (origin(line.gross_price), origin(line.price))
        });
        assert_eq!(
            prices,
            Some((
                Some(("4.55".to_owned(), Origin::Rule)),
                Some(("4.095".to_owned(), Origin::Derived)),
            ))
        );
        assert_eq!(
            read_line(PLAIN, &["Nougat", "20", "4,55", "10%", "81,90"]),
            None
        );
        assert_eq!(read_line(PLAIN, &["Miete", "1", "100,00", "120,00"]), None);
        assert_eq!(read_line(PLAIN, &["Miete", "100,00", "100,00"]), None);
        assert_eq!(read_line(PLAIN, &["Summe", "1", "100,00", "19%"]), None);
        assert_eq!(
            read_line(
                &["Text", "Menge", "Preis", "Betrag", "USt."],
                &["Miete", "1", "100,00", "100,00", "7%", "19%"],
            ),
            Some(expected([
                None,
                Some("Miete"),
                Some("1"),
                Some("100"),
                Some("100"),
                None,
                None
            ]))
        );
    }

    #[test]
    fn the_numbers_of_a_line_follow_their_table() {
        const PLAIN: &[&str] = &["Text", "Menge", "Preis", "Betrag"];
        let numbers = |heading: &[&str], texts: &[&str]| {
            read_line(heading, texts).map(|[_, _, quantity, price, net, ..]| (quantity, price, net))
        };
        let some = |text: &str| Some(text.to_owned());
        let line =
            |quantity: &str, price: &str, net: &str| Some((some(quantity), some(price), some(net)));
        assert_eq!(
            numbers(PLAIN, &["Miete", "1", "100,00", "100,00"]),
            line("1", "100", "100")
        );
        assert_eq!(
            numbers(PLAIN, &["IP", "2", "57811", "1", "0,8403", "0,84"]),
            line("1", "0.8403", "0.84")
        );
        assert_eq!(
            numbers(
                &["Text", "Menge", "Preis", "Betrag", "Steuer", "Brutto"],
                &["Miete", "1", "100,00", "100,00", "19,00", "119,00"]
            ),
            line("1", "100", "100")
        );
        assert_eq!(
            numbers(
                &["Pos", "Text", "Menge", "Preis", "Betrag"],
                &["5", "Miete", "1", "5,00", "7,00"]
            ),
            None
        );
        assert_eq!(
            numbers(
                &["Pos", "Text", "Menge", "Preis", "Betrag", "USt."],
                &["5", "Miete", "1", "5,00", "7,00", "19%"]
            ),
            None
        );
        assert_eq!(numbers(PLAIN, &["Miete", "1", "100,00", "120,00"]), None);
        assert_eq!(numbers(PLAIN, &["Miete", "100,00", "100,00"]), None);
        assert_eq!(numbers(PLAIN, &["Miete", "1", "100,00", "0,025"]), None);
    }

    #[test]
    fn a_price_base_follows_its_table() {
        let number = |text: &str| Decimal::from_str_exact(text).unwrap_or_default();
        let base = |quantity: &str, price: &str, net: &str| {
            price_base(number(quantity), number(price), number(net)).map(|base| base.to_string())
        };
        assert_eq!(base("400", "32.76", "131.04").as_deref(), Some("100"));
        assert_eq!(base("-150", "60.76", "-91.14").as_deref(), Some("100"));
        assert_eq!(base("150", "49.29", "73.94").as_deref(), Some("100"));
        assert_eq!(base("1000", "5.20", "5.20").as_deref(), Some("1000"));
        assert_eq!(base("1000", "0.052", "5.20"), None);
        assert_eq!(base("3", "3.00", "0.01"), None);
        assert_eq!(base("400", "32.76", "131.05"), None);
        assert_eq!(base("400", "32.76", "262.08"), None);
        assert_eq!(base("1", "2.00", "0.00"), None);
    }

    /// The quantity, the net price, the quantity the price is for and the
    /// amount of the line `texts` are, each with whether it is worked out.
    fn worked_out(heading: &[&str], texts: &[&str]) -> Option<[Option<(String, bool)>; 4]> {
        let table = Table {
            name_from: None,
            name_until: None,
            ..heads(heading)?
        };
        let printed = words(LEFT, TOP, texts);
        let found = any_line(&row(&printed), &table)?.line;
        let mark = |part: &Part| (part.text.clone(), part.origin == Origin::Derived);
        Some([
            Some(mark(&found.quantity)),
            found.price.as_ref().map(mark),
            found.price_base.as_ref().map(mark),
            Some(mark(&found.net)),
        ])
    }

    /// A value with whether it is worked out.
    fn marked(text: &str, worked_out: bool) -> (String, bool) {
        (text.to_owned(), worked_out)
    }

    #[test]
    fn a_price_for_a_hundred_is_worked_out_and_marked() {
        const HEADING: &[&str] = &["Pos", "Text", "Preis", "Menge", "Betrag"];
        const QUANTITY_FIRST: &[&str] = &["Text", "Menge", "Preis", "Betrag"];
        assert_eq!(
            worked_out(
                HEADING,
                &["000003", "Schraube", "32,76", "400", "Stk", "131,04"]
            ),
            Some([
                Some(marked("400", false)),
                Some(marked("32.76", false)),
                Some(marked("100", true)),
                Some(marked("131.04", false)),
            ])
        );
        assert_eq!(
            worked_out(HEADING, &["1", "Papier", "9,99", "20", "Stk", "199,80"]),
            Some([
                Some(marked("20", false)),
                Some(marked("9.99", false)),
                None,
                Some(marked("199.8", false))
            ])
        );
        assert_eq!(
            worked_out(HEADING, &["1", "Papier", "9,99", "20", "Stk", "250,00"]),
            None
        );
        // A percentage without its sign is no quantity priced by the
        // hundred: no unit stands after it.
        assert_eq!(
            worked_out(QUANTITY_FIRST, &["Skonto", "2", "100,00", "2,00"]),
            None
        );
        assert_eq!(
            worked_out(
                QUANTITY_FIRST,
                &["Schraube", "400", "Stk", "32,76", "131,04"]
            ),
            Some([
                Some(marked("400", false)),
                Some(marked("32.76", false)),
                Some(marked("100", true)),
                Some(marked("131.04", false)),
            ])
        );
    }

    #[test]
    fn lines_without_a_price_follow_their_table() {
        const HEADING: &[&str] = &["Pos", "Text", "Preis", "Menge", "USt.", "Betrag"];
        assert_eq!(
            worked_out(HEADING, &["1", "Stahlcoil", "1", "Stk", "0%", "1.000,00"]),
            Some([
                Some(marked("1", false)),
                None,
                None,
                Some(marked("1000", false))
            ])
        );
        assert_eq!(
            worked_out(HEADING, &["Stahlcoil", "10", "Stk", "19%", "850,00", "€"]),
            Some([
                Some(marked("10", false)),
                None,
                None,
                Some(marked("850", false))
            ])
        );
        assert_eq!(
            worked_out(HEADING, &["3", "Monteur", "10,50", "28", "Std", "288,75"]),
            None
        );
        assert_eq!(worked_out(HEADING, &["77777777777", "200"]), None);
        assert_eq!(worked_out(HEADING, &["2", "Versand", "4,90"]), None);
        assert_eq!(
            worked_out(&["Text", "Menge", "Betrag"], &["2", "Versand", "4,90"]),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["Garantie", "24", "Monate", "0,00"]),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["Strom", "0,0520", "1.000", "kWh", "52,00"]),
            Some([
                Some(marked("1000", false)),
                Some(marked("0.052", false)),
                None,
                Some(marked("52", false))
            ])
        );
        assert_eq!(
            worked_out(
                HEADING,
                &["4260331811362", "(GLN)", "1250032", "D.", "Muster"]
            ),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["Stahlcoil", "0", "Stk", "850,00"]),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["Stahlcoil", "10", "Rollen", "850,00"]),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["Bauteil", "2026", "Version", "850,00"]),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["Stahlcoil", "10", "Stk", "EUR", "850,00"]),
            Some([
                Some(marked("10", false)),
                None,
                None,
                Some(marked("850", false)),
            ])
        );
        assert_eq!(
            worked_out(HEADING, &["2", "Inklusiv-Kilometer", "waren:", "873"]),
            None
        );
        assert_eq!(
            worked_out(HEADING, &["2", "Kilometer", "gefahren", "873,00"]),
            None
        );
        assert_eq!(worked_out(HEADING, &["2", "Kilometer", "873"]), None);
        assert_eq!(
            worked_out(HEADING, &["Stahlcoil", "10", "Stk", "0,025"]),
            None
        );
        let named = read_line(HEADING, &["1", "Stahlcoil", "1", "Stk", "0%", "1.000,00"]);
        assert_eq!(
            named.map(|[number, name, ..]| (number, name)),
            Some((Some("1".to_owned()), Some("Stahlcoil".to_owned())))
        );
    }

    #[test]
    fn the_unit_after_a_quantity_is_looked_up_and_marked() {
        const HEADING: &[&str] = &["Pos", "Text", "Preis", "Menge", "Betrag"];
        let unit = |heading: &[&str], texts: &[&str]| {
            let table = Table {
                name_from: None,
                name_until: None,
                ..heads(heading)?
            };
            let printed = words(LEFT, TOP, texts);
            let found = any_line(&row(&printed), &table)?.line;
            Some(found.unit.map(|unit| (unit.text, unit.origin)))
        };
        let worked_out = |code: &str| Some(Some((code.to_owned(), Origin::Derived)));
        assert_eq!(
            unit(HEADING, &["1", "Papier", "9,99", "20", "Stk", "199,80"]),
            worked_out("H87")
        );
        assert_eq!(
            unit(HEADING, &["1", "Rasen", "3,00", "3", "m²", "9,00"]),
            worked_out("MTK")
        );
        assert_eq!(
            unit(HEADING, &["2", "Stahlcoil", "10", "Stk", "850,00"]),
            worked_out("H87")
        );
        // A word that is no unit the table knows has no code.
        assert_eq!(
            unit(
                HEADING,
                &["1", "Papier", "9,99", "20", "Paletten", "199,80"]
            ),
            Some(None)
        );
        // No word after the quantity, or the name after it: no unit.
        assert_eq!(
            unit(HEADING, &["1", "Papier", "9,99", "20", "199,80"]),
            Some(None)
        );
        assert_eq!(
            unit(
                &["Amount", "Product", "Price", "Total"],
                &["1.00", "Std", "(hours)", "160.00", "160.00"]
            ),
            Some(None)
        );
    }

    #[test]
    fn a_price_base_has_the_unit_of_the_quantity() {
        const HEADING: &[&str] = &["Pos", "Text", "Preis", "Menge", "Betrag"];
        let units = |texts: &[&str]| {
            let table = Table {
                name_from: None,
                name_until: None,
                ..heads(HEADING)?
            };
            let printed = words(LEFT, TOP, texts);
            let found = any_line(&row(&printed), &table)?.line;
            let code = |unit: Option<Stated>| unit.map(|unit| (unit.text, unit.origin));
            Some((code(found.unit), code(found.price_base_unit)))
        };
        let worked_out = |code: &str| Some((code.to_owned(), Origin::Derived));
        // A price for a hundred: the hundred are pieces as the quantity is.
        assert_eq!(
            units(&["3", "Schraube", "32,76", "400", "Stk", "131,04"]),
            Some((worked_out("H87"), worked_out("H87")))
        );
        // No quantity the price is for: no unit of one.
        assert_eq!(
            units(&["1", "Papier", "9,99", "20", "Stk", "199,80"]),
            Some((worked_out("H87"), None))
        );
        // A price for a hundred of a unit the table does not know.
        assert_eq!(
            units(&["3", "Schraube", "32,76", "400", "Beutel", "131,04"]),
            Some((None, None))
        );
    }

    #[test]
    fn a_code_in_the_name_of_an_item_is_no_category() {
        const HEADING: &[&str] = &["Text", "Menge", "Preis", "Betrag", "USt."];
        let category =
            |texts: &[&str]| read_line(HEADING, texts).and_then(|[.., category]| category);
        assert_eq!(category(&["T-Shirt", "(M)", "2", "10,00", "20,00"]), None);
        assert_eq!(category(&["Kabel", "(L)", "2", "10,00", "20,00"]), None);
        assert_eq!(
            category(&["T-Shirt", "(M)", "2", "10,00", "20,00", "19%", "(S)"]).as_deref(),
            Some("S")
        );
    }

    #[test]
    fn a_net_price_after_a_discount_follows_its_table() {
        let net = |gross: &str, discount: &str| {
            let printed = words(LEFT, TOP, &[gross, discount]);
            let printed = row(&printed);
            let figures = figures(&printed.words);
            let [gross, discount] = figures.as_slice() else {
                return None;
            };
            after_discount(&part(&printed, gross), discount).map(|net| (net.text, net.origin))
        };
        let worked_out = |text: &str| Some((text.to_owned(), Origin::Derived));
        assert_eq!(net("4,55", "10%"), worked_out("4.095"));
        assert_eq!(net("3,33", "5%"), worked_out("3.1635"));
        assert_eq!(net("100,00", "0%"), worked_out("100"));
        assert_eq!(net("100,00", "100%"), worked_out("0"));
    }

    #[test]
    fn names_follow_their_table() {
        const PLAIN: &[&str] = &["Text", "Menge", "Preis", "Betrag"];
        let name = |heading: &[&str], texts: &[&str]| {
            read_line(heading, texts).and_then(|[_, name, ..]| name)
        };
        assert_eq!(
            name(
                &["Pos", "Text", "Menge", "Preis", "Betrag"],
                &["2", "IP", "Server", "57811", "1", "0,84", "0,84"]
            )
            .as_deref(),
            Some("IP Server 57811")
        );
        assert_eq!(
            name(PLAIN, &["Nougat", "250g", "20", "Unit(s)", "4,55", "91,00"]).as_deref(),
            Some("Nougat 250g")
        );
        assert_eq!(
            name(
                &["Amount", "Product", "Price", "Total"],
                &["1.00", "Design", "(hours)", "160.00", "160.00"]
            )
            .as_deref(),
            Some("Design (hours)")
        );
        assert_eq!(
            name(
                &["Pos", "Text", "Preis", "Menge", "USt.", "Betrag"],
                &[
                    "1", "Papier", "9,99", "20", "Stk", "19%", "VAT", "(S)", "199,80"
                ]
            )
            .as_deref(),
            Some("Papier")
        );
        assert_eq!(name(PLAIN, &["2026", "1", "100,00", "100,00"]), None);
        assert_eq!(name(PLAIN, &["1", "Stk", "100,00", "100,00"]), None);
        assert_eq!(name(PLAIN, &["1", "100,00", "€", "100,00", "€"]), None);
    }

    #[test]
    fn a_name_leaves_out_what_stands_left_of_its_column() {
        let heading = [
            words(LEFT, TOP, &["Pos"]),
            words(LEFT + 60.0, TOP, &["Referenzen"]),
            words(LEFT + 160.0, TOP, &["Beschreibung"]),
            words(LEFT + 300.0, TOP, &["Menge", "Preis", "Betrag"]),
        ]
        .concat();
        let table = heading_of(&heading);
        let line_words = [
            words(LEFT, TOP + PITCH, &["1"]),
            words(LEFT + 60.0, TOP + PITCH, &["Art-Nr.:", "SE_KR"]),
            words(LEFT + 160.0, TOP + PITCH, &["Kunstrasen", "grün"]),
            words(LEFT + 300.0, TOP + PITCH, &["2", "10,00", "20,00"]),
        ]
        .concat();
        let found = table.and_then(|table| {
            any_line(&row(&line_words), &table).and_then(|found| found.line.name)
        });
        assert_eq!(
            found.map(|name| name.text).as_deref(),
            Some("Kunstrasen grün")
        );
    }

    fn heading_of(printed: &[Word]) -> Option<Table> {
        heading(&row(printed).words)
    }

    #[test]
    fn names_are_joined_as_their_table_says() {
        let table = [
            (
                "Wartung",
                "technischer Anlagen",
                "Wartung technischer Anlagen",
            ),
            (
                "Personen-",
                "Unfallversicherung",
                "Personen-Unfallversicherung",
            ),
            ("Gartenpflege/", "Schneeräum.", "Gartenpflege/Schneeräum."),
            (
                "Navigationssystem -",
                "Garantie",
                "Navigationssystem - Garantie",
            ),
        ];
        for (name, more, expected) in table {
            let texts: Vec<&str> = more.split(' ').collect();
            assert_eq!(joined(name, &words(LEFT, TOP, &texts)), expected);
        }
    }

    /// The lines of one block below each other, and one of another block.
    fn block_lines<'a>(block: &'a [Vec<Word>], other: &'a [Word]) -> Vec<page::Line<'a>> {
        let line = |block, words| page::Line {
            page: 0,
            block,
            words,
        };
        block
            .iter()
            .map(|words| line(0, words.as_slice()))
            .chain([line(1, other)])
            .collect()
    }

    #[test]
    fn a_name_runs_on_as_its_table_says() {
        let other = words(LEFT, TOP + 5.0 * PITCH, &["Summe"]);
        let names_until = |name_until: Option<f64>| Table {
            columns: Vec::new(),
            name_from: None,
            name_until,
            after_amount: 0,
        };
        let within = |printed: &[Vec<Word>], next: Option<&Word>, table: &Table| -> Vec<String> {
            let lines = block_lines(printed, &other);
            runs_on(&lines, &printed[0][0], next, table)
                .iter()
                .map(|words| {
                    let texts: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
                    texts.join(" ")
                })
                .collect()
        };
        let anywhere = names_until(None);
        let texts = |printed: &[Vec<Word>], next: Option<&Word>| within(printed, next, &anywhere);
        let other_column = [
            words(LEFT, TOP, &["Wartung"]),
            words(LEFT, TOP + PITCH, &["Anlagen"]),
            words(LEFT + 200.0, TOP + 2.0 * PITCH, &["Monate"]),
        ];
        assert_eq!(
            within(&other_column, None, &names_until(Some(LEFT + 200.0))),
            ["Anlagen"]
        );
        assert_eq!(texts(&other_column, None), ["Anlagen", "Monate"]);
        let one = stacked(&[&["Wartung"], &["technischer", "Anlagen"]]);
        assert_eq!(texts(&one, None), ["technischer Anlagen"]);
        let two = stacked(&[&["Wartung"], &["technischer"], &["Anlagen"]]);
        assert_eq!(texts(&two, None), ["technischer", "Anlagen"]);
        let field = stacked(&[&["Wartung"], &["GTIN:", "4000050986428"]]);
        assert_eq!(texts(&field, None), Vec::<String>::new());
        let spaced_field = stacked(&[&["Wartung"], &["Bruttopreis", ":", "43,20"]]);
        assert_eq!(texts(&spaced_field, None), Vec::<String>::new());
        let total = stacked(&[&["Wartung"], &["Summe", "netto", "120,00"]]);
        assert_eq!(texts(&total, None), Vec::<String>::new());
        let then_field = stacked(&[&["Wartung"], &["Anlagen"], &["GTIN:", "4000050986428"]]);
        assert_eq!(texts(&then_field, None), ["Anlagen"]);
        let beside = [
            words(LEFT, TOP, &["Wartung"]),
            words(LEFT + 200.0, TOP, &["2", "Monate"]),
            words(LEFT, TOP + PITCH, &["Anlagen"]),
        ];
        assert_eq!(texts(&beside, None), ["Anlagen"]);
        let next_line = stacked(&[&["Wartung"], &["Strom"]]);
        assert_eq!(
            texts(&next_line, next_line[1].first()),
            Vec::<String>::new()
        );
        let alone = stacked(&[&["Wartung"]]);
        assert_eq!(texts(&alone, None), Vec::<String>::new());
    }

    #[test]
    fn labels_of_totals_follow_their_table() {
        let table: [(&[&str], bool); 4] = [
            (&["Positionssumme", "320,00"], true),
            (&["Net", "total:", "496.00"], true),
            (&["Bestell-Nr.:", "4711"], false),
            (&["~2000ml"], false),
        ];
        for (texts, expected) in table {
            let printed = words(LEFT, TOP, texts);
            assert_eq!(names_total(&row(&printed).words), expected, "{texts:?}");
        }
    }

    #[test]
    fn the_end_of_a_table_of_lines_follows_its_table() {
        let table: [(&[&str], &str, bool); 6] = [
            (&["Positionssumme", "320,00"], "320", true),
            (&["Net", "total:", "496.00", "€"], "496", true),
            (&["Gesamtbetrag:", "1.025,29", "€"], "320", false),
            (&["Positionssumme"], "320", false),
            (&["VAT", "Net", "amount", "VAT", "amount"], "0", true),
            (&["Bestell-Nr.:", "4711"], "4711", false),
        ];
        for (texts, sum, expected) in table {
            let printed = words(LEFT, TOP, texts);
            let sum = Decimal::from_str_exact(sum).unwrap_or_default();
            assert_eq!(ends_table(&row(&printed).words, sum), expected, "{texts:?}");
        }
    }

    #[test]
    fn open_numbers_are_read_the_way_the_row_agrees() {
        const PRICE_FIRST: &[&str] = &["Text", "Preis", "Menge", "Betrag"];
        const PLAIN: &[&str] = &["Text", "Menge", "Preis", "Betrag"];
        let numbers = |heading: &[&str], texts: &[&str]| {
            read_line(heading, texts).map(|[_, _, quantity, price, net, ..]| (quantity, price, net))
        };
        let some = |text: &str| Some(text.to_owned());
        assert_eq!(
            numbers(PRICE_FIRST, &["Strom", "0,0520", "1.000", "kWh", "52,00"]),
            Some((some("1000"), some("0.052"), some("52")))
        );
        assert_eq!(
            numbers(PLAIN, &["Gas", "1.000", "kWh", "52,00", "52,00"]),
            Some((some("1"), some("52"), some("52")))
        );
        assert_eq!(numbers(PLAIN, &["Gas", "1.000", "0,00", "0,00"]), None);
        assert_eq!(numbers(PLAIN, &["Gas", "1.000", "5,00", "70,00"]), None);
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

    /// The names of the lines read from rows, each a block of its own, with
    /// the sum of the lines and the total without VAT as read.
    fn read_names(printed: &[Vec<Word>], totals: [Option<&str>; 2]) -> Vec<Option<String>> {
        let rows: Vec<Row> = printed.iter().map(|words| row(words)).collect();
        let lines: Vec<page::Line> = printed
            .iter()
            .enumerate()
            .map(|(block, words)| page::Line {
                page: 0,
                block,
                words,
            })
            .collect();
        let totals = totals.map(|text| text.map(total));
        read(&rows, &lines, [totals[0].as_ref(), totals[1].as_ref()])
            .into_iter()
            .map(|line| line.name.map(|name| name.text))
            .collect()
    }

    #[test]
    fn a_table_of_lines_is_read_up_to_the_totals() {
        let printed = stacked(&[
            &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"],
            &["1", "Miete", "1", "100,00", "100,00"],
            &["Bestell-Nr.:", "4711"],
            &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"],
            &["2", "Strom", "2", "10,00", "20,00"],
            &["Summe", "netto", "120,00"],
            &["3", "Wasser", "1", "5,00", "5,00"],
        ]);
        let both = [Some("Miete".to_owned()), Some("Strom".to_owned())];
        assert_eq!(read_names(&printed, [Some("120.00"), None]), both);
        assert_eq!(read_names(&printed, [None, Some("120")]), both);
        let none = stacked(&[
            &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"],
            &["Summe"],
        ]);
        assert_eq!(read_names(&none, [Some("0"), None]), []);
    }

    #[test]
    fn lines_count_only_if_they_add_up_to_a_total() {
        let both: &[&[&str]] = &[
            &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"],
            &["1", "Miete", "1", "100,00", "100,00"],
            &["2", "Strom", "2", "10,00", "20,00"],
        ];
        assert_eq!(read_names(&stacked(both), [Some("300"), None]), []);
        assert_eq!(read_names(&stacked(both), [None, None]), []);
        // A line is missed, the table runs past its totals, and a row below
        // is taken for a line.
        let missed = stacked(&[
            &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"],
            &["1", "Miete", "1", "100,00", "100,00"],
            &["2", "Pacht", "1", "200,00", "210,00"],
            &["Summe", "netto", "300,00"],
            &["Versand", "1", "5,00", "5,00"],
        ]);
        assert_eq!(read_names(&missed, [None, Some("300")]), []);
    }

    #[test]
    fn a_table_read_by_its_rows_is_not_read_by_its_sum() {
        const HEADING: &[&str] = &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"];
        let one = [Some("Miete".to_owned())];
        let payment = stacked(&[
            HEADING,
            &["1", "Miete", "1", "100,00", "100,00"],
            &["Zahlbar", "in", "14", "Tagen", "119,00"],
        ]);
        assert_eq!(read_names(&payment, [Some("100"), None]), one);
        let count = stacked(&[
            HEADING,
            &["1", "Miete", "1", "100,00", "100,00"],
            &["Summe", "2", "Positionen", "100,00"],
        ]);
        assert_eq!(read_names(&count, [Some("100"), None]), one);
        // The percentage of a discount without its sign, below the totals.
        let discount = stacked(&[
            HEADING,
            &["1", "Miete", "1", "100,00", "100,00"],
            &["Summe", "netto", "100,00"],
            &["Skonto", "2", "100,00", "2,00"],
        ]);
        assert_eq!(read_names(&discount, [None, Some("100")]), one);
    }

    #[test]
    fn rows_only_the_sum_bears_out_count_where_the_rows_alone_do_not_add_up() {
        const HEADING: &[&str] = &["Pos", "Beschreibung", "Preis", "Menge", "Betrag"];
        let by_the_hundred = stacked(&[
            HEADING,
            &["1", "Schraube", "32,76", "400", "Stk", "131,04"],
            &["Summe", "netto", "131,04"],
        ]);
        assert_eq!(
            read_names(&by_the_hundred, [None, Some("131.04")]),
            [Some("Schraube".to_owned())]
        );
        assert_eq!(read_names(&by_the_hundred, [None, Some("200")]), []);
        let without_price = stacked(&[
            HEADING,
            &["1", "Miete", "100,00", "1", "Monat", "100,00"],
            &["2", "Stahlcoil", "10", "Stk", "850,00"],
            &["Summe", "10", "Positionen", "950,00"],
        ]);
        assert_eq!(
            read_names(&without_price, [Some("950"), None]),
            [Some("Miete".to_owned()), Some("Stahlcoil".to_owned())]
        );
    }

    #[test]
    fn places_are_put_around_each_other_as_their_table_says() {
        let place = |left: f64, right: f64, top: f64, bottom: f64| Location {
            page: 0,
            left,
            top,
            right,
            bottom,
        };
        let first = place(50.0, 90.0, 90.0, 100.0);
        let second = place(50.0, 120.0, 102.0, 112.0);
        assert_eq!(
            around(Some(first), Some(second)),
            Some(place(50.0, 120.0, 90.0, 112.0))
        );
        assert_eq!(around(Some(first), None), Some(first));
        assert_eq!(around(None, Some(second)), Some(second));
        assert_eq!(around(None, None), None);
    }

    #[test]
    fn the_place_of_a_name_holds_the_lines_it_runs_on_over() {
        let heading = words(
            LEFT,
            TOP,
            &["Pos", "Beschreibung", "Menge", "Preis", "Betrag"],
        );
        let line_row: Vec<Word> = [
            words(LEFT, TOP + PITCH, &["18"]),
            words(LEFT + 40.0, TOP + PITCH, &["Wartung"]),
            words(LEFT + 200.0, TOP + PITCH, &["1", "55,49", "55,49"]),
        ]
        .concat();
        let more = words(LEFT + 40.0, TOP + 2.0 * PITCH, &["technischer", "Anlagen"]);
        let rows = [row(&heading), row(&line_row), row(&more)];
        // The name and the line it runs on over are one block, the number
        // of the line and its numbers blocks of their own.
        let block = |block: usize, words| page::Line {
            page: 0,
            block,
            words,
        };
        let lines = [
            block(0, heading.as_slice()),
            block(1, &line_row[..1]),
            block(2, &line_row[1..2]),
            block(2, more.as_slice()),
            block(3, &line_row[2..]),
        ];
        let sum = total("55.49");
        let found = read(&rows, &lines, [Some(&sum), None]);
        let named = found.first().and_then(|line| line.name.clone());
        assert_eq!(
            named.as_ref().map(|name| name.text.as_str()),
            Some("Wartung technischer Anlagen")
        );
        let place = named.and_then(|name| name.location);
        let last = more.last().map(|word| word.right);
        assert_eq!(
            place.map(|at| (at.left, at.bottom, Some(at.right))),
            Some((LEFT + 40.0, TOP + 2.0 * PITCH, last))
        );
    }
}
