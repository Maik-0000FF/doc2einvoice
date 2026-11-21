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

use doc2einvoice_model::{Draft, Entry, Occurrence, Origin};
use doc2einvoice_pdf::{PageLayout, Word};

use crate::address::{self, Address};
use crate::labels::{
    self, AMOUNT_DUE, BUYER_ADDRESS_LINES, BUYER_CITY, BUYER_NAME, BUYER_POST_CODE,
    BUYER_VAT_IDENTIFIER, CURRENCY, DELIVERY_DATE, DUE_DATE, Found, INVOICE_NUMBER, ISSUE_DATE,
    Names, PAYMENT_ACCOUNT, PURCHASE_ORDER, Party, SELLER_ADDRESS_LINES, SELLER_CITY, SELLER_NAME,
    SELLER_POST_CODE, SELLER_VAT_IDENTIFIER, Scope, Section, TOTAL_WITH_VAT, TOTAL_WITHOUT_VAT,
    VAT_TOTAL, Weight,
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

/// The kinds of terms, which differ in where their values stand. The
/// columns are the methods below: a total is not looked for below its
/// label, where a column stands, nor where a table heading ends with it.
///
/// | Kind   | Below | Wrapped line | Whole cell | Past words | Of several |
/// |--------|-------|--------------|------------|------------|------------|
/// | header | yes   | yes          | no         | no         | first      |
/// | name   | yes   | no           | yes        | no         | first      |
/// | total  | no    | no           | no         | yes        | last       |
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Header,
    Name,
    Total,
}

impl Kind {
    /// Whether the value may stand below its label.
    fn below(self) -> bool {
        self != Self::Total
    }

    /// Whether the value may start the next line where the sentence that
    /// ends with its label wraps.
    fn wraps(self) -> bool {
        self == Self::Header
    }

    /// Whether the value beside its label in another block is that whole
    /// block, as a name set over two lines in its cell.
    fn cell(self) -> bool {
        self == Self::Name
    }

    /// Whether the value is looked for beside the label in another block
    /// when the words after the label in its line are not one.
    fn past_words(self) -> bool {
        self == Self::Total
    }

    /// Which of several values found counts.
    fn choice(self) -> Choice {
        if self == Self::Total {
            Choice::Last
        } else {
            Choice::First
        }
    }
}

/// How a business term is read.
struct Rule {
    term: u16,
    read: fn(&[&str]) -> Option<Read>,
    kind: Kind,
    /// The repeated group the term stands in, read as its first occurrence:
    /// the first credit transfer for its account.
    group: Option<u16>,
}

/// The repeated group of a credit transfer (BG-17).
const CREDIT_TRANSFER: u16 = 17;

/// The terms read, the invoice number first: the issue date may follow it.
const RULES: [Rule; 15] = [
    header(INVOICE_NUMBER, values::identifier),
    header(ISSUE_DATE, values::date),
    header(CURRENCY, values::currency),
    header(DUE_DATE, values::date),
    header(DELIVERY_DATE, values::date),
    header(PURCHASE_ORDER, values::identifier),
    name(SELLER_NAME),
    name(BUYER_NAME),
    header(SELLER_VAT_IDENTIFIER, values::vat_identifier),
    header(BUYER_VAT_IDENTIFIER, values::vat_identifier),
    header_in(PAYMENT_ACCOUNT, values::iban, CREDIT_TRANSFER),
    total(TOTAL_WITHOUT_VAT),
    total(VAT_TOTAL),
    total(TOTAL_WITH_VAT),
    total(AMOUNT_DUE),
];

const fn header(term: u16, read: fn(&[&str]) -> Option<Read>) -> Rule {
    Rule {
        term,
        read,
        kind: Kind::Header,
        group: None,
    }
}

/// A header term that stands in the repeated group `group`.
const fn header_in(term: u16, read: fn(&[&str]) -> Option<Read>, group: u16) -> Rule {
    Rule {
        group: Some(group),
        ..header(term, read)
    }
}

/// A name of a party: a text, which may fill its cell over several lines.
const fn name(term: u16) -> Rule {
    Rule {
        term,
        read: values::text,
        kind: Kind::Name,
        group: None,
    }
}

const fn total(term: u16) -> Rule {
    Rule {
        term,
        read: values::monetary,
        kind: Kind::Total,
        group: None,
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
    /// The section the line stands in: which one of the document, counted
    /// from its heading, and whose.
    section: Option<(usize, Party)>,
}

/// The values the rules find on `pages`.
pub fn draft(pages: &[PageLayout]) -> Draft {
    let lines = page::lines(pages);
    let labelled = labelled(&lines);
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
    // Each party takes its address from the first place that has one: an
    // address made of the parts of two is no address.
    let mut addressed: Vec<Party> = Vec::new();
    for (party, found) in addresses(&lines, &labelled) {
        if !addressed.contains(&party) {
            add_address(&mut draft, party, &found);
            addressed.push(party);
        }
    }
    draft
}

/// The terms of the parts of the address of a party whose address is read:
/// its name, its three address lines, its place and its postal code.
///
/// | Party  | Name  | Lines                 | Place | Postal code |
/// |--------|-------|-----------------------|-------|-------------|
/// | seller | BT-27 | BT-35, BT-36, BT-162  | BT-37 | BT-38       |
/// | buyer  | BT-44 | BT-50, BT-51, BT-163  | BT-52 | BT-53       |
struct AddressTerms {
    name: u16,
    lines: [u16; 3],
    place: u16,
    postcode: u16,
}

const SELLER_ADDRESS: AddressTerms = AddressTerms {
    name: SELLER_NAME,
    lines: SELLER_ADDRESS_LINES,
    place: SELLER_CITY,
    postcode: SELLER_POST_CODE,
};

const BUYER_ADDRESS: AddressTerms = AddressTerms {
    name: BUYER_NAME,
    lines: BUYER_ADDRESS_LINES,
    place: BUYER_CITY,
    postcode: BUYER_POST_CODE,
};

fn address_terms(party: Party) -> Option<&'static AddressTerms> {
    match party {
        Party::Seller => Some(&SELLER_ADDRESS),
        Party::Buyer => Some(&BUYER_ADDRESS),
        Party::DeliverTo | Party::Payee | Party::TaxRepresentative | Party::Debited => None,
    }
}

/// The addresses printed without labels and whose they are: the first in
/// the section of each party, the one in the address window as the
/// buyer's, and the return address as the seller's.
fn addresses<'a>(lines: &[Line<'a>], labelled: &[Labelled]) -> Vec<(Party, Address<'a>)> {
    let mut found = Vec::new();
    let mut sections: Vec<(usize, Party, Vec<usize>)> = Vec::new();
    for line in labelled {
        let Some((section, party)) = line.section else {
            continue;
        };
        match sections.last_mut() {
            Some((open, _, region)) if *open == section => region.push(line.line),
            _ => sections.push((section, party, vec![line.line])),
        }
    }
    for (_, party, region) in sections {
        if let Some(address) = address::in_region(lines, &region, &address::WHOLE_PAGE) {
            found.push((party, address));
        }
    }
    let window = address::window(lines);
    if let Some(address) = address::in_region(lines, &window, &address::window_across()) {
        found.push((Party::Buyer, address));
    }
    if let Some(address) = address::return_address(lines) {
        found.push((Party::Seller, address));
    }
    found
}

/// The parts of `found` as the entries of `party`, each where the draft has
/// no value for its term yet.
///
/// Which of the lines above the postal code and place is the name and which
/// are the lines of the address is told by their number. With more than two
/// it is open: a name set over two lines, a contact person and a second line
/// of the address all stand between a name and its street. Those lines are
/// then left to the user; the place and postal code are read all the same.
///
/// | Lines above the place               | Read                                   |
/// |-------------------------------------|----------------------------------------|
/// | name, street                        | name; first line                       |
/// | name                                | name                                   |
/// | name, street, more                  | none of them                           |
/// | street, more, with a labelled name  | first, second, third line              |
fn add_address(draft: &mut Draft, party: Party, found: &Address) {
    let Some(terms) = address_terms(party) else {
        return;
    };
    let labelled_name = draft.values_of(terms.name).next().is_some();
    let mut above: Vec<&Span> = found.name.iter().chain(&found.lines).collect();
    let mut entries: Vec<(u16, &Span, String)> = Vec::new();
    if !labelled_name {
        if above.len() > UNAMBIGUOUS_LINES {
            above.clear();
        }
        if let Some(name) = (!above.is_empty()).then(|| above.remove(0)) {
            entries.push((terms.name, name, name.texts().join(" ")));
        }
    }
    for (term, line) in terms.lines.iter().zip(above) {
        entries.push((*term, line, line.texts().join(" ")));
    }
    entries.push((terms.place, &found.place, found.place.texts().join(" ")));
    entries.push((
        terms.postcode,
        &found.postcode,
        found.postcode_digits.clone(),
    ));
    for (term, span, text) in entries {
        if draft.values_of(term).next().is_some() || span.words.is_empty() {
            continue;
        }
        draft.entries.push(Entry {
            term,
            within: Vec::new(),
            text,
            origin: Origin::Rule,
            location: span.location(0, span.words.len()),
        });
    }
}

/// How many lines above the postal code and place tell without doubt which
/// is the name and which the street: two, the name above the street.
const UNAMBIGUOUS_LINES: usize = 2;

/// The labels of every line. A field of a party, as "Name", takes the term
/// of the party whose section it stands in, or the term it has outside any
/// section. A section opens with the heading of a party, or with a line that
/// says the amount is taken by direct debit. It ends with the heading of
/// another section, a line above its heading, a line further below it than
/// the gap between two sections, or the end of its page.
fn labelled<'a>(lines: &[Line<'a>]) -> Vec<Labelled<'a>> {
    let mut section: Option<OpenSection> = None;
    let mut opened = 0;
    let mut labelled = Vec::with_capacity(lines.len());
    for (line, printed) in lines.iter().enumerate() {
        let words = printed.texts();
        section = section.filter(|open| open.holds(printed));
        match labels::heading(&words) {
            Some(Section::Party(party)) => {
                opened += 1;
                section = OpenSection::at(printed, party);
            }
            Some(Section::Other) => section = None,
            None if labels::says_direct_debit(&words) => {
                opened += 1;
                section = OpenSection::at(printed, Party::Debited);
            }
            None => {}
        }
        if let Some(open) = section.as_mut() {
            open.extend(printed);
        }
        let party = section.as_ref().map(|open| open.party);
        let mut found = labels::find(&words);
        for label in &mut found {
            if let Names::PartyField(field) = label.names {
                let term = party.map_or_else(
                    || labels::unsectioned_term(field),
                    |party| labels::party_term(field, party),
                );
                if let Some(term) = term {
                    label.names = Names::Term(term);
                }
            }
        }
        labelled.push(Labelled {
            line,
            found,
            words,
            section: party.map(|party| (opened, party)),
        });
    }
    labelled
}

/// How far below the lowest line of a section the next line may start and
/// still belong to it, as a multiple of its font size: the lines of a
/// section follow at the distance of a line, sections stand further apart.
const SECTION_GAP: f64 = 3.0;

/// The section of a party as far as it has been read.
struct OpenSection {
    party: Party,
    page: usize,
    /// The first word of its heading.
    heading: Word,
    /// The lowest baseline of its lines so far.
    bottom: f64,
}

impl OpenSection {
    fn at(heading: &Line, party: Party) -> Option<Self> {
        let head = heading.words.first()?;
        Some(Self {
            party,
            page: heading.page,
            heading: head.clone(),
            bottom: head.baseline,
        })
    }

    /// Whether `line` still belongs to the section: on its page, not above
    /// its heading, and not further below it than a gap between sections.
    fn holds(&self, line: &Line) -> bool {
        line.words.first().is_some_and(|head| {
            line.page == self.page
                && !page::lies_below(&self.heading, head)
                && head.baseline <= self.bottom + head.font_size * SECTION_GAP
        })
    }

    fn extend(&mut self, line: &Line) {
        if let Some(head) = line.words.first() {
            self.bottom = self.bottom.max(head.baseline);
        }
    }
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
            if found.names != Names::Term(rule.term) {
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
    match rule.kind.choice() {
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
        return entry(rule, &after, &read).map(|entry| Candidate {
            standing: Standing::Elsewhere,
            nearness: Nearness::Beside,
            line,
            end_in_line: Some(end_in_line),
            entry,
        });
    }
    let only_separators = after.texts().iter().all(|text| *text == SEPARATING_WORD);
    if !only_separators && !rule.kind.past_words() {
        return None;
    }
    // Only a label that closes a line with text before it ends a sentence
    // that may wrap: a label alone in its line heads a column.
    let closes_sentence = rule.kind.wraps() && found.first > 0 && end == printed.words.len();
    let places = [
        (
            Nearness::Beside,
            Some(if rule.kind.cell() {
                page::cell_right_of(lines, line, last)
            } else {
                page::right_of(lines, line, last)
            }),
        ),
        (
            Nearness::Wrapped,
            closes_sentence
                .then(|| page::wrapped(lines, line))
                .flatten(),
        ),
        (
            Nearness::Below,
            rule.kind
                .below()
                .then(|| page::below(lines, line, first, last)),
        ),
    ];
    places.into_iter().find_map(|(nearness, span)| {
        let span = span?;
        let read = read_after_separators(rule, &span)?;
        entry(rule, &span, &read).map(|entry| Candidate {
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

fn entry(rule: &Rule, span: &Span, read: &Read) -> Option<Entry> {
    Some(Entry {
        term: rule.term,
        within: rule
            .group
            .map(|group| Occurrence { group, index: 0 })
            .into_iter()
            .collect(),
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
        AMOUNT_DUE, BUYER_ADDRESS_LINES, BUYER_CITY, BUYER_NAME, BUYER_POST_CODE,
        BUYER_VAT_IDENTIFIER, CURRENCY, DELIVERY_DATE, DUE_DATE, INVOICE_NUMBER, ISSUE_DATE,
        PAYMENT_ACCOUNT, PURCHASE_ORDER, SELLER_ADDRESS_LINES, SELLER_CITY, SELLER_NAME,
        SELLER_POST_CODE, SELLER_VAT_IDENTIFIER, TOTAL_WITH_VAT, TOTAL_WITHOUT_VAT, VAT_TOTAL,
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
    fn a_name_takes_the_party_of_its_section_and_its_whole_cell() {
        let pages = [page(vec![
            block(50.0, 80.0, &["Name:", "Niemand"]),
            block(50.0, 100.0, &["Verkäufer"]),
            wrapped_block(
                300.0,
                120.0 - LINE_PITCH,
                &[&["MVM", "Musterhafter"], &["Versicherungsverein", "a.G."]],
            ),
            block(50.0, 120.0, &["Name:"]),
            block(50.0, 160.0, &["Abweichender", "Warenempfänger"]),
            block(50.0, 180.0, &["Name:", "Lager", "Nord"]),
            block(50.0, 200.0, &["Käufer/Leistungsempfänger:"]),
            block(50.0, 220.0, &["Name:", "Kunden", "AG"]),
        ])];
        assert_eq!(
            value(&pages, SELLER_NAME).as_deref(),
            Some("MVM Musterhafter Versicherungsverein a.G.")
        );
        assert_eq!(value(&pages, BUYER_NAME).as_deref(), Some("Kunden AG"));
    }

    #[test]
    fn the_name_of_the_party_delivered_to_is_not_the_buyer() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Käufer:"]),
            block(50.0, 120.0, &["Abweichender", "Warenempfänger"]),
            block(50.0, 140.0, &["Name:", "Lager", "Nord"]),
        ])];
        assert_eq!(value(&pages, BUYER_NAME), None);
    }

    #[test]
    fn the_heading_of_another_section_ends_that_of_a_party() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Verkäufer"]),
            block(50.0, 120.0, &["Bankverbindung"]),
            block(50.0, 140.0, &["Name:", "Sparkasse", "Musterstadt"]),
        ])];
        assert_eq!(value(&pages, SELLER_NAME), None);
    }

    #[test]
    fn a_name_beside_its_label_leaves_out_the_address_beside_other_labels() {
        let labels = wrapped_block(50.0, 120.0, &[&["Name:"], &["Straße:"], &["Ort:"]]);
        let values = wrapped_block(
            300.0,
            120.0,
            &[
                &["Muster", "GmbH"],
                &["Hauptstr.", "1"],
                &["12345", "Stadt"],
            ],
        );
        let pages = [page(vec![
            block(50.0, 100.0, &["Verkäufer"]),
            labels,
            values,
        ])];
        assert_eq!(value(&pages, SELLER_NAME).as_deref(), Some("Muster GmbH"));
    }

    #[test]
    fn a_section_ends_at_a_gap_wider_than_between_its_lines() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Käufer:"]),
            block(50.0, 100.0 + LINE_PITCH, &["Kunden", "AG"]),
            block(50.0, 200.0, &["USt-IdNr:", "DE136695976"]),
        ])];
        assert_eq!(value(&pages, BUYER_VAT_IDENTIFIER), None);
        let close = [page(vec![
            block(50.0, 100.0, &["Käufer:"]),
            block(50.0, 100.0 + LINE_PITCH, &["USt-IdNr:", "DE136695976"]),
        ])];
        assert_eq!(
            value(&close, BUYER_VAT_IDENTIFIER).as_deref(),
            Some("DE136695976")
        );
    }

    #[test]
    fn references_and_the_account_are_read() {
        let pages = [page(vec![
            block(
                50.0,
                100.0,
                &["Rechnung", "gemäß", "Bestellung", "Nr.", "2018-471331"],
            ),
            block(
                50.0,
                120.0,
                &["IBAN", ":", "DE02", "1203", "0000", "0000", "2020", "51"],
            ),
            block(50.0, 140.0, &["N°", "TVA", "client", "FR19787878784"]),
        ])];
        assert_eq!(
            value(&pages, PURCHASE_ORDER).as_deref(),
            Some("2018-471331")
        );
        assert_eq!(
            value(&pages, BUYER_VAT_IDENTIFIER).as_deref(),
            Some("FR19787878784")
        );
        let found = draft(&pages);
        let account = found.values_of(PAYMENT_ACCOUNT).next();
        assert_eq!(
            account.map(|entry| entry.text.as_str()),
            Some("DE02120300000000202051")
        );
        let within = account.map(|entry| entry.within.clone());
        assert_eq!(
            within,
            Some(vec![doc2einvoice_model::Occurrence {
                group: 17,
                index: 0
            }])
        );
    }

    #[test]
    fn a_line_above_the_heading_ends_its_section() {
        let pages = [page(vec![
            block(50.0, 300.0, &["Verkäufer"]),
            block(300.0, 100.0, &["USt-IdNr.:", "DE136695976"]),
        ])];
        assert_eq!(value(&pages, SELLER_VAT_IDENTIFIER), None);
    }

    #[test]
    fn the_account_of_a_direct_debit_is_not_the_one_to_transfer_to() {
        let debit = [page(vec![
            block(50.0, 100.0, &["Der", "Betrag", "wird", "abgebucht", "von:"]),
            block(
                50.0,
                100.0 + LINE_PITCH,
                &["IBAN:", "DE02120300000000202051"],
            ),
        ])];
        assert_eq!(value(&debit, PAYMENT_ACCOUNT), None);
        let buyer = [page(vec![
            block(50.0, 100.0, &["Käufer"]),
            block(
                50.0,
                100.0 + LINE_PITCH,
                &["IBAN:", "DE02120300000000202051"],
            ),
        ])];
        assert_eq!(value(&buyer, PAYMENT_ACCOUNT), None);
        let footer = [page(vec![block(
            50.0,
            100.0,
            &["IBAN:", "DE02120300000000202051"],
        )])];
        assert_eq!(
            value(&footer, PAYMENT_ACCOUNT).as_deref(),
            Some("DE02120300000000202051")
        );
    }

    #[test]
    fn a_direct_debit_only_offered_leaves_the_account_to_transfer_to() {
        for offer in [
            &["Zahlbar", "per", "Überweisung", "oder", "SEPA-Lastschrift"][..],
            &["Eine", "Lastschrift", "ist", "nicht", "möglich."],
        ] {
            let pages = [page(vec![
                block(50.0, 100.0, offer),
                block(
                    50.0,
                    100.0 + LINE_PITCH,
                    &["IBAN:", "DE02120300000000202051"],
                ),
            ])];
            assert_eq!(
                value(&pages, PAYMENT_ACCOUNT).as_deref(),
                Some("DE02120300000000202051"),
                "{offer:?}"
            );
        }
    }

    /// Millimetres as points, for places on a page given as DIN 5008 gives
    /// them.
    fn mm(millimetres: f64) -> f64 {
        millimetres * crate::address::POINTS_PER_MM
    }

    fn values(pages: &[PageLayout], terms: &[u16]) -> Vec<Option<String>> {
        terms.iter().map(|term| value(pages, *term)).collect()
    }

    #[test]
    fn an_address_in_the_section_of_a_party_is_read_without_its_labelled_lines() {
        let pages = [page(vec![wrapped_block(
            50.0,
            100.0,
            &[
                &["Verkäufer:"],
                &["Globale", "Nummer:", "4000001123452"],
                &["Lieferant", "GmbH"],
                &["Lieferantenstraße", "20"],
                &["DE", "99199", "Musterhausen"],
            ],
        )])];
        assert_eq!(
            values(
                &pages,
                &[
                    SELLER_NAME,
                    SELLER_ADDRESS_LINES[0],
                    SELLER_CITY,
                    SELLER_POST_CODE
                ]
            ),
            [
                Some("Lieferant GmbH".to_owned()),
                Some("Lieferantenstraße 20".to_owned()),
                Some("Musterhausen".to_owned()),
                Some("99199".to_owned()),
            ]
        );
    }

    #[test]
    fn more_than_two_lines_above_the_place_leave_name_and_lines_open() {
        let pages = [page(vec![wrapped_block(
            50.0,
            100.0,
            &[
                &["Käufer:"],
                &["Taxi-Gast", "AG"],
                &["Hans", "Mustermann"],
                &["Kundenstraße", "15"],
                &["69876", "Frankfurt"],
            ],
        )])];
        assert_eq!(
            values(&pages, &[BUYER_NAME, BUYER_ADDRESS_LINES[0]]),
            [None, None]
        );
        assert_eq!(value(&pages, BUYER_POST_CODE).as_deref(), Some("69876"));
    }

    #[test]
    fn the_address_window_holds_the_buyer_and_the_return_address_the_seller() {
        let window = |top: f64, rows: &[&[&str]]| wrapped_block(mm(25.0), mm(top), rows);
        let pages = [page(vec![
            block(
                mm(25.0),
                mm(55.0),
                &[
                    "Muster",
                    "GmbH",
                    "·",
                    "Hauptstr.",
                    "1",
                    "·",
                    "80333",
                    "München",
                ],
            ),
            window(
                60.0,
                &[
                    &["Kunden", "AG"],
                    &["Bahnstr.", "42"],
                    &["88802", "Spielkreis"],
                ],
            ),
        ])];
        assert_eq!(
            values(
                &pages,
                &[
                    BUYER_NAME,
                    BUYER_ADDRESS_LINES[0],
                    BUYER_CITY,
                    BUYER_POST_CODE
                ]
            ),
            [
                Some("Kunden AG".to_owned()),
                Some("Bahnstr. 42".to_owned()),
                Some("Spielkreis".to_owned()),
                Some("88802".to_owned()),
            ]
        );
        assert_eq!(
            values(
                &pages,
                &[
                    SELLER_NAME,
                    SELLER_ADDRESS_LINES[0],
                    SELLER_CITY,
                    SELLER_POST_CODE
                ]
            ),
            [
                Some("Muster GmbH".to_owned()),
                Some("Hauptstr. 1".to_owned()),
                Some("München".to_owned()),
                Some("80333".to_owned()),
            ]
        );
    }

    #[test]
    fn a_return_address_needs_a_place_geonames_knows() {
        let pages = [page(vec![block(
            mm(25.0),
            mm(50.0),
            &[
                "Muster",
                "GmbH",
                "·",
                "Ecke",
                "12",
                "·",
                "12345",
                "Stadthausen",
            ],
        )])];
        assert_eq!(value(&pages, SELLER_NAME), None);
    }

    #[test]
    fn a_labelled_name_makes_the_lines_above_the_place_lines_of_the_address() {
        let pages = [page(vec![
            block(50.0, 100.0, &["Verkäufer"]),
            block(50.0, 100.0 + LINE_PITCH, &["Name:"]),
            block(300.0, 100.0 + LINE_PITCH, &["MUSTERLIEFERANT", "GMBH"]),
            block(50.0, 100.0 + 2.0 * LINE_PITCH, &["Anschrift:"]),
            block(300.0, 100.0 + 2.0 * LINE_PITCH, &["BAHNHOFSTRASSE", "99"]),
            block(300.0, 100.0 + 3.0 * LINE_PITCH, &["Hof", "3"]),
            block(
                300.0,
                100.0 + 4.0 * LINE_PITCH,
                &["DE", "99199", "MUSTERHAUSEN"],
            ),
        ])];
        assert_eq!(
            values(
                &pages,
                &[
                    SELLER_NAME,
                    SELLER_ADDRESS_LINES[0],
                    SELLER_ADDRESS_LINES[1]
                ]
            ),
            [
                Some("MUSTERLIEFERANT GMBH".to_owned()),
                Some("BAHNHOFSTRASSE 99".to_owned()),
                Some("Hof 3".to_owned()),
            ]
        );
    }

    /// A window address of a letter: a name, a street and a place, from the
    /// top at `top` millimetres.
    fn window_address(top: f64) -> Block {
        wrapped_block(
            mm(25.0),
            mm(top),
            &[
                &["Kunden", "AG"],
                &["Bahnstr.", "42"],
                &["88802", "Spielkreis"],
            ],
        )
    }

    #[test]
    fn the_information_block_beside_the_window_leaves_its_address_whole() {
        let pages = [page(vec![
            window_address(60.0),
            block(mm(130.0), mm(60.0), &["Kundennummer:", "4711"]),
        ])];
        assert_eq!(value(&pages, BUYER_NAME).as_deref(), Some("Kunden AG"));
        assert_eq!(
            value(&pages, BUYER_ADDRESS_LINES[0]).as_deref(),
            Some("Bahnstr. 42")
        );
    }

    #[test]
    fn a_letterhead_above_the_address_zone_is_not_the_buyer() {
        let letterhead = wrapped_block(
            mm(25.0),
            mm(20.0),
            &[
                &["Muster", "GmbH"],
                &["Hauptstr.", "1"],
                &["80333", "München"],
            ],
        );
        let pages = [page(vec![letterhead, window_address(60.0)])];
        assert_eq!(value(&pages, BUYER_NAME).as_deref(), Some("Kunden AG"));
    }

    #[test]
    fn a_return_address_ends_where_the_window_ends() {
        let pages = [page(vec![
            block(
                mm(25.0),
                mm(55.0),
                &["Muster", "GmbH", "·", "80333", "München"],
            ),
            block(mm(140.0), mm(55.0), &["Datum:", "21.11.2026"]),
        ])];
        assert_eq!(value(&pages, SELLER_CITY).as_deref(), Some("München"));
    }

    #[test]
    fn separators_in_the_footer_are_no_return_address() {
        let pages = [page(vec![block(
            mm(25.0),
            mm(280.0),
            &[
                "Muster",
                "GmbH",
                "·",
                "Hauptstr.",
                "1",
                "·",
                "80333",
                "München",
            ],
        )])];
        assert_eq!(value(&pages, SELLER_NAME), None);
    }

    #[test]
    fn a_heading_without_a_colon_is_not_the_name() {
        let pages = [page(vec![wrapped_block(
            50.0,
            100.0,
            &[
                &["Verkäufer"],
                &["Lieferant", "GmbH"],
                &["Hauptstr.", "1"],
                &["80333", "München"],
            ],
        )])];
        assert_eq!(
            value(&pages, SELLER_NAME).as_deref(),
            Some("Lieferant GmbH")
        );
    }

    #[test]
    fn a_party_takes_its_address_from_one_place_only() {
        let section = wrapped_block(
            50.0,
            mm(150.0),
            &[&["Käufer"], &["Kunden", "AG"], &["80333", "München"]],
        );
        let pages = [page(vec![window_address(60.0), section])];
        assert_eq!(value(&pages, BUYER_NAME).as_deref(), Some("Kunden AG"));
        assert_eq!(value(&pages, BUYER_CITY).as_deref(), Some("München"));
        assert_eq!(value(&pages, BUYER_ADDRESS_LINES[0]), None);
    }

    #[test]
    fn a_section_ends_with_its_page() {
        let pages = [
            page(vec![block(50.0, 100.0, &["Verkäufer"])]),
            page(vec![block(50.0, 100.0, &["Name:", "Kunden", "AG"])]),
        ];
        assert_eq!(value(&pages, SELLER_NAME), None);
    }

    #[test]
    fn the_due_date_is_read_and_a_discount_date_is_not() {
        let pages = [page(vec![
            block(
                50.0,
                100.0,
                &["Zahlbar", "mit", "2%", "Skonto", "bis", "14.12.2025"],
            ),
            block(
                50.0,
                120.0,
                &["Zahlbar", "ohne", "Abschlag", "bis", "31.12.2025"],
            ),
            block(
                50.0,
                140.0,
                &["Liefer-", "und", "Leistungsdatum:", "05.08.2026"],
            ),
        ])];
        assert_eq!(value(&pages, DUE_DATE).as_deref(), Some("2025-12-31"));
        assert_eq!(value(&pages, DELIVERY_DATE).as_deref(), Some("2026-08-05"));
        let column = [page(vec![
            block(226.0, 100.0, &["Fälligkeit"]),
            block(226.0, 100.0 + LINE_PITCH, &["04.04.2026"]),
        ])];
        assert_eq!(value(&column, DUE_DATE).as_deref(), Some("2026-04-04"));
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
