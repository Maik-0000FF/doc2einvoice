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

use doc2einvoice_model::{Entry, Location, Origin};
use doc2einvoice_pdf::Word;
use rust_decimal::Decimal;

use crate::breakdown;
use crate::labels::{self, Names, TOTALS};
use crate::page::{self, Row, Span};
use crate::table::{self, Figure, Part, Reading, Stated, comes_to, figures, figures_as, part};
use crate::values;

/// One line of an invoice.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The number of the line, where the row starts with one.
    pub identifier: Option<Part>,
    /// The name of the item.
    pub name: Option<Stated>,
    pub quantity: Part,
    /// The net price of one: as printed, or, where a discount is taken off
    /// the price printed, worked out from the two and marked so.
    pub price: Option<Part>,
    /// The gross price of one, where a discount is taken off it.
    pub gross_price: Option<Part>,
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
fn line<'a>(row: &'a Row<'a>, table: &Table) -> Option<Found<'a>> {
    line_of(row, table, &figures(&row.words)).or_else(|| {
        let thousands = line_of(row, table, &figures_as(&row.words, Reading::Thousands));
        let decimals = line_of(row, table, &figures_as(&row.words, Reading::Decimals));
        match (thousands, decimals) {
            (Some(line), None) | (None, Some(line)) => Some(line),
            (Some(_), Some(_)) | (None, None) => None,
        }
    })
}

/// The numbers of a row that make a line.
struct Numbers<'f> {
    quantity: &'f Figure,
    price: &'f Figure,
    net: &'f Figure,
    /// The discount taken off, if the numbers agree only with it.
    discount: Option<&'f Figure>,
}

/// The numbers among those of a row that make a line: the last three
/// standing together, none with the sign of a rate, that are the quantity
/// and the price, in the order of the heading, and the amount, and agree,
/// with one of the printed rates taken off as a discount or without. What
/// stands after them is another column, as the tax or the amount with it,
/// and needs a word of the heading after the name of the amount.
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
/// | 1, 100, 120                        | none        | none (do not agree)               |
/// | 100, 100                           | none        | none (two numbers)                |
/// | 1, 100, 0.025 (no amount of money) | none        | none                              |
fn numbers<'f>(plain: &[&'f Figure], marked: &[&'f Figure], table: &Table) -> Option<Numbers<'f>> {
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
        let discount = if comes_to(gross, net.value) {
            None
        } else {
            let printed = marked
                .iter()
                .find(|rate| table.has(Column::Discount) && comes_to(taken_off(rate), net.value))?;
            Some(*printed)
        };
        Some(Numbers {
            quantity,
            price,
            net,
            discount,
        })
    })
}

/// The line the numbers `figures` of a row make, if they agree.
fn line_of<'a>(row: &'a Row<'a>, table: &Table, figures: &[Figure]) -> Option<Found<'a>> {
    let (plain, marked): (Vec<&Figure>, Vec<&Figure>) =
        figures.iter().partition(|figure| !figure.marked);
    let Numbers {
        quantity,
        price,
        net,
        discount,
    } = numbers(&plain, &marked, table)?;
    let rates: Vec<&Figure> = marked
        .iter()
        .copied()
        .filter(|rate| discount.is_none_or(|discount| discount.first != rate.first))
        .collect();
    let rate = match rates.as_slice() {
        [rate] if table.has(Column::Rate) => Some(part(row, rate)),
        _ => None,
    };
    let identifier = figures.first().filter(|head| {
        table.has(Column::Position)
            && head.first == 0
            && !head.marked
            && head.value.fract().is_zero()
            && [quantity, price, net]
                .iter()
                .all(|taken| taken.first != head.first)
    });
    let mut taken: Vec<&Figure> = vec![quantity, price, net];
    taken.extend(identifier);
    taken.extend(marked.iter().copied());
    let after_quantity = quantity.first + quantity.count;
    let unit = (!table.name_after_quantity()
        && !figures.iter().any(|figure| figure.first == after_quantity))
    .then_some(after_quantity);
    let name = name(row, &taken, unit, price.first.min(net.first), table);
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
        quantity: part(row, quantity),
        price: match discount {
            None => Some(part(row, price)),
            Some(discount) => after_discount(&part(row, price), discount),
        },
        gross_price: discount.is_some().then(|| part(row, price)),
        net: part(row, net),
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
fn table_lines(rows: &[Row], at: usize, table: &Table, printed: &[page::Line]) -> Vec<Line> {
    let mut found: Vec<Found> = Vec::new();
    let mut sum = Decimal::ZERO;
    for row in rows.iter().skip(at + 1) {
        if let Some(line) = line(row, table) {
            sum = sum.checked_add(line.line.net.value).unwrap_or(sum);
            found.push(line);
        } else if heading(&row.words).is_none() && ends_table(&row.words, sum) {
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
pub fn read(rows: &[Row], printed: &[page::Line], totals: [Option<&Entry>; 2]) -> Vec<Line> {
    let sums: Vec<Decimal> = totals
        .iter()
        .flatten()
        .filter_map(|total| Decimal::from_str(&total.text).ok())
        .collect();
    rows.iter()
        .enumerate()
        .filter_map(|(at, row)| {
            heading(&row.words).map(|table| table_lines(rows, at, &table, printed))
        })
        .find(|found| !found.is_empty() && add_up(found, &sums))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use doc2einvoice_model::{Entry, Location, Origin};
    use doc2einvoice_pdf::Word;
    use rust_decimal::Decimal;

    use super::{
        Column, Line, Table, after_discount, around, ends_table, heading, joined, line,
        names_total, read, runs_on,
    };
    use crate::page::{self, Row};
    use crate::table::printed::{LEFT, PITCH, TOP, row, stacked, words};
    use crate::table::{Part, figures, part};

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
        line(&row(&printed), &table).map(|found| values(&found.line))
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
            line(&row(&printed), &table).map(|found| found.line)
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
        let found = table
            .and_then(|table| line(&row(&line_words), &table).and_then(|found| found.line.name));
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
