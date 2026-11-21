// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The labels invoices print in front of their values, in German, English
//! and French, and where in a line they are found.
//!
//! The table also holds labels of terms that are not read yet. They take
//! their words away from shorter labels: "Gesamtbetrag der Zuschläge" is not
//! the total "Gesamtbetrag", and "Date d'échéance" is not the issue date
//! "Date". Labels of other numbers and dates end the words a label of a term
//! read here may take its value from: in "Rechnung Kunden Nr. 4711" the
//! number belongs to the customer.

use Scope::{AfterNumber, Anywhere, LineStart};
use Word::{EndsWith, Is};

/// One word of a label, compared with a printed word in lower case and
/// without a colon or point at its end.
#[derive(Debug, Clone, Copy)]
pub enum Word {
    /// The word itself.
    Is(&'static str),
    /// Any word ending so, as "Handelsrechnung" and "Teilrechnung" end in
    /// "rechnung".
    EndsWith(&'static str),
}

/// Where in a line a label counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Anywhere.
    Anywhere,
    /// Only as the first words of a line: a title such as "Rechnung" or
    /// "Facture", which elsewhere is a word of a sentence.
    LineStart,
    /// Only after the invoice number in the same line, as "vom" in
    /// "Rechnung Nr. 17 vom 16.09.2026": a word too common to mean the issue
    /// date anywhere else.
    AfterNumber,
}

/// How much a label says about its term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weight {
    /// It names the term, as "Rechnungsdatum".
    Specific,
    /// It names the term only where nothing more specific is printed, as
    /// "Datum", or a title such as "Rechnung" with the number after it.
    General,
}

/// A label: its words, the business term it names, where it counts and how
/// much it says.
#[derive(Debug, Clone, Copy)]
pub struct Label {
    pub words: &'static [Word],
    pub term: u16,
    pub scope: Scope,
    pub weight: Weight,
}

/// The business terms of the labels.
pub const INVOICE_NUMBER: u16 = 1;
pub const ISSUE_DATE: u16 = 2;
pub const CURRENCY: u16 = 5;
pub const DUE_DATE: u16 = 9;
const PURCHASE_ORDER: u16 = 13;
const PRECEDING_INVOICE: u16 = 25;
pub const DELIVERY_DATE: u16 = 72;
const LINE_TOTAL: u16 = 106;
const ALLOWANCES: u16 = 107;
const CHARGES: u16 = 108;
pub const TOTAL_WITHOUT_VAT: u16 = 109;
pub const VAT_TOTAL: u16 = 110;
pub const TOTAL_WITH_VAT: u16 = 112;
const PAID: u16 = 113;
/// A label that names no business term, as "Druckdatum" or "Bestelldatum",
/// dates the standard has no term for: it only keeps its words from the
/// labels read here.
const NOT_A_TERM: u16 = 0;

/// The label "Name" before a section says whose it is.
const UNSECTIONED_NAME: u16 = u16::MAX;
/// Terms whose labels only keep their words from the labels read here.
const BUYER_IDENTIFIER: u16 = 46;
const SELLER_TAX_NUMBER: u16 = 32;
const DESPATCH_ADVICE: u16 = 16;
const SALES_ORDER: u16 = 14;
const SELLER_VAT_IDENTIFIER: u16 = 31;
pub const AMOUNT_DUE: u16 = 115;

const fn label(words: &'static [Word], term: u16, scope: Scope) -> Label {
    Label {
        words,
        term,
        scope,
        weight: Weight::Specific,
    }
}

const fn general(words: &'static [Word], term: u16, scope: Scope) -> Label {
    Label {
        words,
        term,
        scope,
        weight: Weight::General,
    }
}

/// Every label.
pub const LABELS: &[Label] = &[
    // The invoice number (BT-1).
    label(&[Is("rechnungsnummer")], INVOICE_NUMBER, Anywhere),
    label(&[Is("rechnungs-nr")], INVOICE_NUMBER, Anywhere),
    label(&[Is("rechnungsnr")], INVOICE_NUMBER, Anywhere),
    label(&[Is("rechnung-nr")], INVOICE_NUMBER, Anywhere),
    label(&[EndsWith("rechnung"), Is("nr")], INVOICE_NUMBER, Anywhere),
    label(&[EndsWith("rechnung"), Is("#")], INVOICE_NUMBER, Anywhere),
    label(
        &[Is("rechnungskorrektur"), Is("nr")],
        INVOICE_NUMBER,
        Anywhere,
    ),
    label(
        &[EndsWith("gutschrift"), Is("nr")],
        INVOICE_NUMBER,
        Anywhere,
    ),
    label(&[Is("gutschriftsnummer")], INVOICE_NUMBER, Anywhere),
    general(&[EndsWith("rechnung")], INVOICE_NUMBER, LineStart),
    general(&[Is("gutschrift")], INVOICE_NUMBER, LineStart),
    label(&[Is("invoice"), Is("number")], INVOICE_NUMBER, Anywhere),
    label(&[Is("invoice"), Is("no")], INVOICE_NUMBER, Anywhere),
    label(&[Is("invoice"), Is("nr")], INVOICE_NUMBER, Anywhere),
    label(&[Is("invoice"), Is("#")], INVOICE_NUMBER, Anywhere),
    general(&[Is("invoice")], INVOICE_NUMBER, LineStart),
    label(
        &[Is("credit"), Is("note"), Is("number")],
        INVOICE_NUMBER,
        Anywhere,
    ),
    general(&[Is("credit"), Is("note")], INVOICE_NUMBER, LineStart),
    label(
        &[Is("numéro"), Is("de"), Is("facture")],
        INVOICE_NUMBER,
        Anywhere,
    ),
    label(
        &[Is("n°"), Is("de"), Is("facture")],
        INVOICE_NUMBER,
        Anywhere,
    ),
    label(&[Is("facture"), Is("n°")], INVOICE_NUMBER, Anywhere),
    general(&[Is("facture")], INVOICE_NUMBER, LineStart),
    label(&[Is("avoir"), Is("n°")], INVOICE_NUMBER, Anywhere),
    general(&[Is("avoir")], INVOICE_NUMBER, LineStart),
    // The issue date (BT-2).
    label(&[Is("rechnungsdatum")], ISSUE_DATE, Anywhere),
    label(&[Is("gutschriftsdatum")], ISSUE_DATE, Anywhere),
    general(&[Is("datum")], ISSUE_DATE, Anywhere),
    label(&[Is("invoice"), Is("date")], ISSUE_DATE, Anywhere),
    label(&[Is("date"), Is("of"), Is("issue")], ISSUE_DATE, Anywhere),
    label(&[Is("issue"), Is("date")], ISSUE_DATE, Anywhere),
    general(&[Is("date")], ISSUE_DATE, Anywhere),
    label(&[Is("date"), Is("de"), Is("facture")], ISSUE_DATE, Anywhere),
    label(&[Is("date"), Is("d'émission")], ISSUE_DATE, Anywhere),
    label(&[Is("vom")], ISSUE_DATE, AfterNumber),
    label(&[Is("von")], ISSUE_DATE, AfterNumber),
    label(&[Is("issued"), Is("at")], ISSUE_DATE, AfterNumber),
    label(&[Is("issued"), Is("on")], ISSUE_DATE, AfterNumber),
    label(&[Is("dated")], ISSUE_DATE, AfterNumber),
    label(&[Is("of")], ISSUE_DATE, AfterNumber),
    label(&[Is("du")], ISSUE_DATE, AfterNumber),
    // The invoice currency (BT-5).
    label(&[Is("währung")], CURRENCY, Anywhere),
    label(&[Is("rechnungswährung")], CURRENCY, Anywhere),
    label(&[Is("currency")], CURRENCY, Anywhere),
    label(&[Is("invoice"), Is("currency")], CURRENCY, Anywhere),
    label(&[Is("devise")], CURRENCY, Anywhere),
    // The total without VAT (BT-109).
    label(
        &[Is("rechnungssumme"), Is("ohne"), Is("ust")],
        TOTAL_WITHOUT_VAT,
        Anywhere,
    ),
    label(&[Is("summe"), Is("netto")], TOTAL_WITHOUT_VAT, Anywhere),
    label(&[Is("nettosumme")], TOTAL_WITHOUT_VAT, Anywhere),
    label(
        &[Is("gesamtsumme"), Is("netto")],
        TOTAL_WITHOUT_VAT,
        Anywhere,
    ),
    label(
        &[Is("gesamtbetrag"), Is("netto")],
        TOTAL_WITHOUT_VAT,
        Anywhere,
    ),
    label(&[Is("nettobetrag")], TOTAL_WITHOUT_VAT, Anywhere),
    label(&[Is("net"), Is("total")], TOTAL_WITHOUT_VAT, Anywhere),
    label(&[Is("total"), Is("net")], TOTAL_WITHOUT_VAT, Anywhere),
    label(
        &[Is("total"), Is("before"), Is("tax")],
        TOTAL_WITHOUT_VAT,
        Anywhere,
    ),
    label(
        &[Is("total"), Is("excl"), Is("vat")],
        TOTAL_WITHOUT_VAT,
        Anywhere,
    ),
    label(
        &[Is("total"), Is("without"), Is("vat")],
        TOTAL_WITHOUT_VAT,
        Anywhere,
    ),
    label(&[Is("total"), Is("ht")], TOTAL_WITHOUT_VAT, Anywhere),
    // The VAT total (BT-110).
    label(&[Is("steuerbetrag")], VAT_TOTAL, Anywhere),
    label(&[Is("umsatzsteuer"), Is("gesamt")], VAT_TOTAL, Anywhere),
    label(&[Is("ust"), Is("gesamt")], VAT_TOTAL, Anywhere),
    label(&[Is("mwst"), Is("gesamt")], VAT_TOTAL, Anywhere),
    label(&[Is("summe"), Is("ust")], VAT_TOTAL, Anywhere),
    label(&[Is("summe"), Is("mwst")], VAT_TOTAL, Anywhere),
    label(&[Is("vat"), Is("total")], VAT_TOTAL, Anywhere),
    label(&[Is("total"), Is("vat")], VAT_TOTAL, Anywhere),
    label(&[Is("tax"), Is("total")], VAT_TOTAL, Anywhere),
    label(&[Is("total"), Is("taxes")], VAT_TOTAL, Anywhere),
    label(&[Is("total"), Is("tva")], VAT_TOTAL, Anywhere),
    label(&[Is("montant"), Is("tva")], VAT_TOTAL, Anywhere),
    // The total with VAT (BT-112).
    label(&[Is("bruttosumme")], TOTAL_WITH_VAT, Anywhere),
    label(&[Is("gesamtbetrag")], TOTAL_WITH_VAT, Anywhere),
    label(&[Is("rechnungsbetrag")], TOTAL_WITH_VAT, Anywhere),
    label(&[Is("gesamtsumme"), Is("brutto")], TOTAL_WITH_VAT, Anywhere),
    label(&[Is("summe"), Is("brutto")], TOTAL_WITH_VAT, Anywhere),
    label(
        &[Is("rechnungssumme"), Is("mit"), Is("ust")],
        TOTAL_WITH_VAT,
        Anywhere,
    ),
    label(&[Is("invoice"), Is("total")], TOTAL_WITH_VAT, Anywhere),
    label(&[Is("total"), Is("amount")], TOTAL_WITH_VAT, Anywhere),
    label(&[Is("grand"), Is("total")], TOTAL_WITH_VAT, Anywhere),
    label(
        &[Is("total"), Is("incl"), Is("vat")],
        TOTAL_WITH_VAT,
        Anywhere,
    ),
    label(&[Is("total"), Is("ttc")], TOTAL_WITH_VAT, Anywhere),
    // The amount due (BT-115).
    label(&[Is("zahlbetrag")], AMOUNT_DUE, Anywhere),
    label(&[Is("zu"), Is("zahlen")], AMOUNT_DUE, Anywhere),
    label(
        &[Is("zu"), Is("zahlender"), Is("betrag")],
        AMOUNT_DUE,
        Anywhere,
    ),
    label(&[Is("fälliger"), Is("betrag")], AMOUNT_DUE, Anywhere),
    label(&[Is("offener"), Is("betrag")], AMOUNT_DUE, Anywhere),
    label(&[Is("amount"), Is("due")], AMOUNT_DUE, Anywhere),
    label(&[Is("balance"), Is("due")], AMOUNT_DUE, Anywhere),
    label(&[Is("due"), Is("payable")], AMOUNT_DUE, Anywhere),
    label(&[Is("net"), Is("à"), Is("payer")], AMOUNT_DUE, Anywhere),
    label(&[Is("solde"), Is("à"), Is("payer")], AMOUNT_DUE, Anywhere),
    label(&[Is("montant"), Is("à"), Is("payer")], AMOUNT_DUE, Anywhere),
    // The payment due date (BT-9).
    label(&[Is("fälligkeitsdatum")], DUE_DATE, Anywhere),
    label(&[Is("fälligkeit")], DUE_DATE, Anywhere),
    label(&[Is("netto"), Is("bis")], DUE_DATE, Anywhere),
    label(&[Is("ohne"), Is("abzug"), Is("bis")], DUE_DATE, Anywhere),
    label(&[Is("ohne"), Is("abschlag"), Is("bis")], DUE_DATE, Anywhere),
    label(
        &[Is("überweisen"), Is("sie"), Is("bis"), Is("zum")],
        DUE_DATE,
        Anywhere,
    ),
    label(&[Is("fällig"), Is("am")], DUE_DATE, Anywhere),
    label(&[Is("fällig"), Is("bis")], DUE_DATE, Anywhere),
    label(&[Is("zahlbar"), Is("bis")], DUE_DATE, Anywhere),
    label(&[Is("zahlbar"), Is("bis"), Is("zum")], DUE_DATE, Anywhere),
    label(&[Is("due"), Is("date")], DUE_DATE, Anywhere),
    label(&[Is("payment"), Is("due")], DUE_DATE, Anywhere),
    label(&[Is("payable"), Is("by")], DUE_DATE, Anywhere),
    label(&[Is("date"), Is("d'échéance")], DUE_DATE, Anywhere),
    label(&[Is("échéance")], DUE_DATE, Anywhere),
    // The actual delivery date (BT-72).
    label(&[Is("leistungsdatum")], DELIVERY_DATE, Anywhere),
    label(&[Is("lieferdatum")], DELIVERY_DATE, Anywhere),
    label(
        &[Is("liefer-"), Is("und"), Is("leistungsdatum")],
        DELIVERY_DATE,
        Anywhere,
    ),
    label(
        &[Is("date"), Is("of"), Is("delivery")],
        DELIVERY_DATE,
        Anywhere,
    ),
    label(&[Is("delivery"), Is("date")], DELIVERY_DATE, Anywhere),
    label(
        &[Is("date"), Is("de"), Is("livraison")],
        DELIVERY_DATE,
        Anywhere,
    ),
    // Terms not read yet, which keep their words from shorter labels.
    label(&[Is("bestellung"), Is("nr")], PURCHASE_ORDER, Anywhere),
    label(&[Is("bestellnummer")], PURCHASE_ORDER, Anywhere),
    label(&[Is("your"), Is("order")], PURCHASE_ORDER, Anywhere),
    label(&[Is("facture"), Is("liée")], PRECEDING_INVOICE, Anywhere),
    label(&[Is("positionssumme")], LINE_TOTAL, Anywhere),
    label(&[Is("summe"), Is("positionen")], LINE_TOTAL, Anywhere),
    label(
        &[Is("gesamtbetrag"), Is("der"), Is("abschläge")],
        ALLOWANCES,
        Anywhere,
    ),
    label(
        &[Is("gesamtbetrag"), Is("der"), Is("zuschläge")],
        CHARGES,
        Anywhere,
    ),
    label(&[Is("erhaltene"), Is("anzahlungen")], PAID, Anywhere),
    label(&[Is("bereits"), Is("gezahlt")], PAID, Anywhere),
    label(&[Is("paid"), Is("amount")], PAID, Anywhere),
    // "Name" outside any section; a section gives it its term.
    label(NAME, UNSECTIONED_NAME, Anywhere),
    // Other numbers, which keep a title such as "Rechnung" from them.
    label(&[Is("kunden"), Is("nr")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("kunden-nr")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("kundennr")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("kundennummer")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("customer"), Is("no")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("customer"), Is("number")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("n°"), Is("client")], BUYER_IDENTIFIER, Anywhere),
    label(&[Is("bestell-nr")], PURCHASE_ORDER, Anywhere),
    label(&[Is("bestellnr")], PURCHASE_ORDER, Anywhere),
    label(&[Is("order"), Is("no")], PURCHASE_ORDER, Anywhere),
    label(&[Is("order"), Is("number")], PURCHASE_ORDER, Anywhere),
    label(&[Is("auftrags-nr")], SALES_ORDER, Anywhere),
    label(&[Is("auftragsnummer")], SALES_ORDER, Anywhere),
    label(&[Is("auftrag"), Is("nr")], SALES_ORDER, Anywhere),
    label(&[Is("lieferschein-nr")], DESPATCH_ADVICE, Anywhere),
    label(&[Is("lieferscheinnummer")], DESPATCH_ADVICE, Anywhere),
    label(&[Is("lieferschein"), Is("nr")], DESPATCH_ADVICE, Anywhere),
    label(&[Is("steuer-nr")], SELLER_TAX_NUMBER, Anywhere),
    label(&[Is("steuernummer")], SELLER_TAX_NUMBER, Anywhere),
    label(&[Is("steuer"), Is("nr")], SELLER_TAX_NUMBER, Anywhere),
    label(&[Is("ust-idnr")], SELLER_VAT_IDENTIFIER, Anywhere),
    label(&[Is("ust.-id.-nr")], SELLER_VAT_IDENTIFIER, Anywhere),
    label(&[Is("ust-id")], SELLER_VAT_IDENTIFIER, Anywhere),
    label(&[Is("vat"), Is("id")], SELLER_VAT_IDENTIFIER, Anywhere),
    // Other dates, which keep "Datum" and "Date" from them.
    label(&[Is("bestelldatum")], NOT_A_TERM, Anywhere),
    label(&[Is("order"), Is("date")], NOT_A_TERM, Anywhere),
    label(
        &[Is("date"), Is("de"), Is("commande")],
        NOT_A_TERM,
        Anywhere,
    ),
    label(&[Is("auftragsdatum")], NOT_A_TERM, Anywhere),
    label(&[Is("lieferscheindatum")], NOT_A_TERM, Anywhere),
    label(
        &[Is("datum"), Is("der"), Is("lieferung")],
        DELIVERY_DATE,
        Anywhere,
    ),
    label(&[Is("druckdatum")], NOT_A_TERM, Anywhere),
    label(&[Is("print"), Is("date")], NOT_A_TERM, Anywhere),
];

/// The party a section of an invoice is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Party {
    Seller,
    Buyer,
    /// The party the goods are delivered to (BG-13).
    DeliverTo,
    /// A payee other than the seller (BG-10).
    Payee,
    /// The tax representative of the seller (BG-11).
    TaxRepresentative,
}

/// The business terms of the names of the parties.
pub const SELLER_NAME: u16 = 27;
pub const BUYER_NAME: u16 = 44;
const PAYEE_NAME: u16 = 59;
const TAX_REPRESENTATIVE_NAME: u16 = 62;
const DELIVER_TO_NAME: u16 = 70;

/// The label "Name", which names the name of the party whose section it
/// stands in.
const NAME: &[Word] = &[Is("name")];

/// "Name" in the section of each party.
const SELLER_NAME_LABEL: Label = label(NAME, SELLER_NAME, Anywhere);
const BUYER_NAME_LABEL: Label = label(NAME, BUYER_NAME, Anywhere);
const DELIVER_TO_NAME_LABEL: Label = label(NAME, DELIVER_TO_NAME, Anywhere);
const PAYEE_NAME_LABEL: Label = label(NAME, PAYEE_NAME, Anywhere);
const TAX_REPRESENTATIVE_NAME_LABEL: Label = label(NAME, TAX_REPRESENTATIVE_NAME, Anywhere);

/// The headings of the sections of the parties, each a line of its own.
const HEADINGS: &[(&[Word], Party)] = &[
    (&[Is("verkäufer")], Party::Seller),
    (&[Is("lieferant")], Party::Seller),
    (&[Is("rechnungssteller")], Party::Seller),
    (&[Is("seller")], Party::Seller),
    (&[Is("supplier")], Party::Seller),
    (&[Is("vendeur")], Party::Seller),
    (&[Is("fournisseur")], Party::Seller),
    (&[Is("käufer")], Party::Buyer),
    (&[Is("käufer/leistungsempfänger")], Party::Buyer),
    (&[Is("leistungsempfänger")], Party::Buyer),
    (&[Is("rechnungsempfänger")], Party::Buyer),
    (&[Is("buyer")], Party::Buyer),
    (&[Is("bill"), Is("to")], Party::Buyer),
    (&[Is("acheteur")], Party::Buyer),
    (&[Is("warenempfänger")], Party::DeliverTo),
    (
        &[Is("abweichender"), Is("warenempfänger")],
        Party::DeliverTo,
    ),
    (&[Is("lieferanschrift")], Party::DeliverTo),
    (&[Is("lieferadresse")], Party::DeliverTo),
    (&[Is("ship"), Is("to")], Party::DeliverTo),
    (&[Is("deliver"), Is("to")], Party::DeliverTo),
    (
        &[Is("adresse"), Is("de"), Is("livraison")],
        Party::DeliverTo,
    ),
    (&[Is("zahlungsempfänger")], Party::Payee),
    (&[Is("abweichender"), Is("zahlungsempfänger")], Party::Payee),
    (&[Is("payee")], Party::Payee),
    (&[Is("steuervertreter")], Party::TaxRepresentative),
    (&[Is("tax"), Is("representative")], Party::TaxRepresentative),
];

/// The party a line heads, if the line is nothing but the heading of its
/// section.
///
/// | Words                              | Party      |
/// |------------------------------------|------------|
/// | `Verkäufer`                        | seller     |
/// | `Käufer/Leistungsempfänger:`       | buyer      |
/// | `Abweichender` `Warenempfänger`    | deliver to |
/// | `Verkäufer` `Muster` `GmbH`        | none (more than a heading) |
/// | `Name:`                            | none       |
pub fn heading(words: &[&str]) -> Option<Party> {
    let normal: Vec<String> = words.iter().map(|word| normalized(word)).collect();
    HEADINGS
        .iter()
        .find(|(pattern, _)| {
            pattern.len() == normal.len()
                && pattern
                    .iter()
                    .zip(&normal)
                    .all(|(word, printed)| matches(*word, printed))
        })
        .map(|(_, party)| *party)
}

/// The label "Name" stands for in the section of `party`.
pub fn party_name(party: Party) -> &'static Label {
    match party {
        Party::Seller => &SELLER_NAME_LABEL,
        Party::Buyer => &BUYER_NAME_LABEL,
        Party::DeliverTo => &DELIVER_TO_NAME_LABEL,
        Party::Payee => &PAYEE_NAME_LABEL,
        Party::TaxRepresentative => &TAX_REPRESENTATIVE_NAME_LABEL,
    }
}

/// Whether `found` is the label "Name", which only a section gives a term.
pub fn is_party_name(found: &Found) -> bool {
    found.label.term == UNSECTIONED_NAME
}

/// Characters a printed word may end in that do not belong to the word of
/// a label.
const LABEL_ENDINGS: [char; 2] = [':', '.'];

/// Words that say a number follows, as "Nr." in "Bau (877) Nr. 210113",
/// written as [`normalized`] gives them.
pub const NUMBER_MARKERS: [&str; 5] = ["nr", "no", "#", "n°", "nummer"];

/// A printed word as it is compared with the words of labels: in lower case,
/// without a colon or point at its end.
pub fn normalized(word: &str) -> String {
    word.trim_end_matches(LABEL_ENDINGS).to_lowercase()
}

/// Whether a printed word is a code in brackets, as the `(380)` in
/// "Handelsrechnung (380) Nr.", which a label passes over.
fn is_bracketed_code(word: &str) -> bool {
    word.strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .is_some_and(|code| !code.is_empty() && code.chars().all(|c| c.is_ascii_digit()))
}

fn matches(pattern: Word, word: &str) -> bool {
    match pattern {
        Is(expected) => word == expected,
        EndsWith(ending) => word.ends_with(ending),
    }
}

/// A label found in a line: which one, and the words it covers.
#[derive(Debug, Clone, Copy)]
pub struct Found {
    pub label: &'static Label,
    /// The first word of the label.
    pub first: usize,
    /// The word after the label.
    pub end: usize,
}

/// The labels in a line of `words`, from left to right. At each word the
/// longest label that fits is taken, and the words it covers are not looked
/// at again. Codes in brackets inside a label are passed over.
///
/// | Words                                  | Found                          |
/// |----------------------------------------|--------------------------------|
/// | `Rechnungsnummer:` `RE-1`              | BT-1 over word 0               |
/// | `Handelsrechnung` `(380)` `Nr.` `17`   | BT-1 over words 0 to 2         |
/// | `Gesamtbetrag` `der` `Zuschläge` `5,00`| BT-108 (not the total, BT-112) |
/// | `Gesamtbetrag:` `571,04`               | BT-112                         |
/// | `Rechnung` `RE-1` (first in the line)  | BT-1                           |
/// | `zur` `Rechnung` `RE-1`                | nothing (a title only at the start) |
/// | `Nr.` `17` `vom` `16.09.2026`          | BT-2 over word 2 (scope checked later) |
pub fn find(words: &[&str]) -> Vec<Found> {
    let normal: Vec<String> = words.iter().map(|word| normalized(word)).collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < words.len() {
        let best = LABELS
            .iter()
            .filter(|label| label.scope != Scope::LineStart || at == 0)
            .filter_map(|label| covered(label, words, &normal, at).map(|end| (label, end)))
            .max_by_key(|(label, end)| (*end, label.words.len()));
        match best {
            Some((label, end)) => {
                found.push(Found {
                    label,
                    first: at,
                    end,
                });
                at = end;
            }
            None => at += 1,
        }
    }
    found
}

/// The word after `label` if it starts at `at`.
fn covered(label: &Label, words: &[&str], normal: &[String], at: usize) -> Option<usize> {
    let mut index = at;
    for (place, pattern) in label.words.iter().enumerate() {
        while place > 0 && words.get(index).is_some_and(|word| is_bracketed_code(word)) {
            index += 1;
        }
        if !matches(*pattern, normal.get(index)?) {
            return None;
        }
        index += 1;
    }
    Some(index)
}

#[cfg(test)]
mod tests {
    use super::{CHARGES, INVOICE_NUMBER, ISSUE_DATE, Party, TOTAL_WITH_VAT, find, heading};

    fn found(words: &[&str]) -> Vec<(u16, usize, usize)> {
        find(words)
            .iter()
            .map(|found| (found.label.term, found.first, found.end))
            .collect()
    }

    #[test]
    fn labels_are_found_after_their_table() {
        assert_eq!(
            found(&["Rechnungsnummer:", "RE-1"]),
            [(INVOICE_NUMBER, 0, 1)]
        );
        assert_eq!(
            found(&["Handelsrechnung", "(380)", "Nr.", "17"]),
            [(INVOICE_NUMBER, 0, 3)]
        );
        assert_eq!(
            found(&["Gesamtbetrag", "der", "Zuschläge", "5,00"]),
            [(CHARGES, 0, 3)]
        );
        assert_eq!(
            found(&["Gesamtbetrag:", "571,04"]),
            [(TOTAL_WITH_VAT, 0, 1)]
        );
        assert_eq!(found(&["Rechnung", "RE-1"]), [(INVOICE_NUMBER, 0, 1)]);
        assert!(found(&["zur", "Rechnung", "RE-1"]).is_empty());
        assert_eq!(
            found(&["Nr.", "17", "vom", "16.09.2026"]),
            [(ISSUE_DATE, 2, 3)]
        );
    }

    #[test]
    fn headings_follow_their_table() {
        assert_eq!(heading(&["Verkäufer"]), Some(Party::Seller));
        assert_eq!(heading(&["Käufer/Leistungsempfänger:"]), Some(Party::Buyer));
        assert_eq!(
            heading(&["Abweichender", "Warenempfänger"]),
            Some(Party::DeliverTo)
        );
        assert_eq!(heading(&["Verkäufer", "Muster", "GmbH"]), None);
        assert_eq!(heading(&["Name:"]), None);
    }
}
