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

/// What a label names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Names {
    /// A business term, by its number.
    Term(u16),
    /// Nothing the standard has a term for, as "Druckdatum" or
    /// "Bestelldatum": the label only keeps its words from the labels read
    /// here.
    Nothing,
    /// A field of a party, whichever party the section it stands in is
    /// about: "Name", "USt-IdNr.".
    PartyField(PartyField),
}

/// A field every party has, which its section gives a business term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyField {
    Name,
    VatIdentifier,
    /// The account of the party: the one to transfer to, or the one a
    /// direct debit takes from.
    Account,
}

/// A label: its words, what it names, where it counts and how much it says.
#[derive(Debug, Clone, Copy)]
pub struct Label {
    pub words: &'static [Word],
    pub names: Names,
    pub scope: Scope,
    pub weight: Weight,
}

/// The business terms of the labels.
pub const INVOICE_NUMBER: u16 = 1;
pub const ISSUE_DATE: u16 = 2;
pub const CURRENCY: u16 = 5;
pub const DUE_DATE: u16 = 9;
pub const PURCHASE_ORDER: u16 = 13;
const PRECEDING_INVOICE: u16 = 25;
pub const DELIVERY_DATE: u16 = 72;
const LINE_TOTAL: u16 = 106;
const ALLOWANCES: u16 = 107;
const CHARGES: u16 = 108;
pub const TOTAL_WITHOUT_VAT: u16 = 109;
pub const VAT_TOTAL: u16 = 110;
pub const TOTAL_WITH_VAT: u16 = 112;
const PAID: u16 = 113;
/// The account to transfer to (BT-84), and the account a direct debit
/// takes from (BT-91), which only keeps its words from the other.
pub const PAYMENT_ACCOUNT: u16 = 84;
const DEBITED_ACCOUNT: u16 = 91;
/// Terms whose labels only keep their words from the labels read here.
const BUYER_IDENTIFIER: u16 = 46;
const SELLER_TAX_NUMBER: u16 = 32;
const DESPATCH_ADVICE: u16 = 16;
const SALES_ORDER: u16 = 14;
pub const AMOUNT_DUE: u16 = 115;

const fn label(words: &'static [Word], term: u16, scope: Scope) -> Label {
    Label {
        words,
        names: Names::Term(term),
        scope,
        weight: Weight::Specific,
    }
}

const fn general(words: &'static [Word], term: u16, scope: Scope) -> Label {
    Label {
        words,
        names: Names::Term(term),
        scope,
        weight: Weight::General,
    }
}

/// A label of a field of a party, which the section it stands in gives a
/// term.
const fn party(words: &'static [Word], field: PartyField) -> Label {
    Label {
        words,
        names: Names::PartyField(field),
        scope: Scope::Anywhere,
        weight: Weight::Specific,
    }
}

/// A label that names nothing the standard has a term for.
const fn blocking(words: &'static [Word], scope: Scope) -> Label {
    Label {
        words,
        names: Names::Nothing,
        scope,
        weight: Weight::Specific,
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
    // The purchase order reference (BT-13).
    label(&[Is("bestellung"), Is("nr")], PURCHASE_ORDER, Anywhere),
    label(
        &[Is("bestellung"), Is("nr"), Is("kunde")],
        PURCHASE_ORDER,
        Anywhere,
    ),
    label(&[Is("ihre"), Is("bestellung")], PURCHASE_ORDER, Anywhere),
    label(&[Is("ihre"), Is("bestellnummer")], PURCHASE_ORDER, Anywhere),
    label(&[Is("purchase"), Is("order")], PURCHASE_ORDER, Anywhere),
    label(&[Is("order"), Is("reference")], PURCHASE_ORDER, Anywhere),
    label(&[Is("po"), Is("number")], PURCHASE_ORDER, Anywhere),
    label(&[Is("votre"), Is("commande")], PURCHASE_ORDER, Anywhere),
    label(
        &[Is("n°"), Is("de"), Is("commande")],
        PURCHASE_ORDER,
        Anywhere,
    ),
    label(&[Is("bestellnummer")], PURCHASE_ORDER, Anywhere),
    label(&[Is("your"), Is("order")], PURCHASE_ORDER, Anywhere),
    // The account to transfer to (BT-84), or the account a direct debit
    // takes from (BT-91), by the section the label stands in.
    party(&[Is("iban")], PartyField::Account),
    label(&[Is("verkäufer-iban")], PAYMENT_ACCOUNT, Anywhere),
    label(&[Is("käufer-iban")], DEBITED_ACCOUNT, Anywhere),
    // Terms not read yet, which keep their words from shorter labels.
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
    party(NAME, PartyField::Name),
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
    // The VAT identifier of the party whose section it stands in.
    party(&[Is("ust-idnr")], PartyField::VatIdentifier),
    party(&[Is("ust.-id.-nr")], PartyField::VatIdentifier),
    party(&[Is("ust-id")], PartyField::VatIdentifier),
    party(&[Is("ust-id-nr")], PartyField::VatIdentifier),
    party(&[Is("ust.-idnr")], PartyField::VatIdentifier),
    party(
        &[Is("umsatzsteuer-identifikationsnummer")],
        PartyField::VatIdentifier,
    ),
    party(&[Is("vat"), Is("id")], PartyField::VatIdentifier),
    party(&[Is("vat"), Is("number")], PartyField::VatIdentifier),
    party(&[Is("vat"), Is("no")], PartyField::VatIdentifier),
    party(&[Is("vat"), Is("reg"), Is("no")], PartyField::VatIdentifier),
    party(&[Is("n°"), Is("tva")], PartyField::VatIdentifier),
    party(
        &[Is("numéro"), Is("de"), Is("tva")],
        PartyField::VatIdentifier,
    ),
    party(
        &[Is("tva"), Is("intracommunautaire")],
        PartyField::VatIdentifier,
    ),
    // VAT identifiers whose label says whose they are.
    label(
        &[Is("unsere"), Is("ust-idnr")],
        SELLER_VAT_IDENTIFIER,
        Anywhere,
    ),
    label(
        &[Is("our"), Is("vat"), Is("id")],
        SELLER_VAT_IDENTIFIER,
        Anywhere,
    ),
    label(
        &[Is("ihre"), Is("ust-idnr")],
        BUYER_VAT_IDENTIFIER,
        Anywhere,
    ),
    label(
        &[Is("your"), Is("vat"), Is("id")],
        BUYER_VAT_IDENTIFIER,
        Anywhere,
    ),
    label(
        &[Is("n°"), Is("tva"), Is("client")],
        BUYER_VAT_IDENTIFIER,
        Anywhere,
    ),
    // Other dates, which keep "Datum" and "Date" from them.
    blocking(&[Is("bestelldatum")], Anywhere),
    blocking(&[Is("order"), Is("date")], Anywhere),
    blocking(&[Is("date"), Is("de"), Is("commande")], Anywhere),
    blocking(&[Is("auftragsdatum")], Anywhere),
    blocking(&[Is("lieferscheindatum")], Anywhere),
    label(
        &[Is("datum"), Is("der"), Is("lieferung")],
        DELIVERY_DATE,
        Anywhere,
    ),
    blocking(&[Is("druckdatum")], Anywhere),
    blocking(&[Is("print"), Is("date")], Anywhere),
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
    /// Whoever pays by direct debit (BG-19): a section that says the amount
    /// is taken from an account.
    Debited,
}

/// The business terms of the fields of the parties.
pub const SELLER_NAME: u16 = 27;
pub const SELLER_VAT_IDENTIFIER: u16 = 31;
pub const SELLER_ADDRESS_LINES: [u16; 3] = [35, 36, 162];
pub const SELLER_CITY: u16 = 37;
pub const SELLER_POST_CODE: u16 = 38;
pub const BUYER_NAME: u16 = 44;
pub const BUYER_VAT_IDENTIFIER: u16 = 48;
pub const BUYER_ADDRESS_LINES: [u16; 3] = [50, 51, 163];
pub const BUYER_CITY: u16 = 52;
pub const BUYER_POST_CODE: u16 = 53;
const PAYEE_NAME: u16 = 59;
const TAX_REPRESENTATIVE_NAME: u16 = 62;
const TAX_REPRESENTATIVE_VAT_IDENTIFIER: u16 = 63;
const DELIVER_TO_NAME: u16 = 70;

/// The label "Name", which names the name of the party whose section it
/// stands in.
const NAME: &[Word] = &[Is("name")];

/// What a section of an invoice is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// A party, whose fields the labels in the section name.
    Party(Party),
    /// Something else, as the bank account: it ends the section of a party,
    /// so that the name of the bank is not the name of the seller.
    Other,
}

/// The headings of sections that are not about a party, each a line of its
/// own.
const OTHER_HEADINGS: &[&[Word]] = &[
    &[Is("bankverbindung")],
    &[Is("zahlungsinformationen")],
    &[Is("zahlungsart")],
    &[Is("zahlungsbedingungen")],
    &[Is("bemerkungen")],
    &[Is("positionen")],
    &[Is("rechnungspositionen")],
    &[Is("belegsummen")],
    &[Is("umsatzsteueraufschlüsselung")],
    &[Is("bank"), Is("details")],
    &[Is("payment"), Is("details")],
    &[Is("payment"), Is("information")],
    &[Is("payment"), Is("terms")],
    &[Is("notes")],
    &[Is("remarks")],
    &[Is("coordonnées"), Is("bancaires")],
];

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

/// Whether the words of a line, as compared, are `pattern` and nothing else.
fn is_whole_line(pattern: &[Word], normal: &[String]) -> bool {
    pattern.len() == normal.len() && starts_line(pattern, normal)
}

/// Whether the words of a line, as compared, start with `pattern`.
fn starts_line(pattern: &[Word], normal: &[String]) -> bool {
    pattern.len() <= normal.len()
        && pattern
            .iter()
            .zip(normal)
            .all(|(word, printed)| matches(*word, printed))
}

/// The section a line heads, if the line is nothing but the heading of it.
///
/// | Words                              | Section    |
/// |------------------------------------|------------|
/// | `Verkäufer`                        | seller     |
/// | `Käufer/Leistungsempfänger:`       | buyer      |
/// | `Abweichender` `Warenempfänger`    | deliver to |
/// | `Bankverbindung`                   | other      |
/// | `Bank` `details:`                  | other      |
/// | `Verkäufer` `Muster` `GmbH`        | none (more than a heading) |
/// | `Name:`                            | none       |
pub fn heading(words: &[&str]) -> Option<Section> {
    let normal: Vec<String> = words.iter().map(|word| normalized(word)).collect();
    let is = |pattern: &[Word]| is_whole_line(pattern, &normal);
    let party = HEADINGS
        .iter()
        .find(|(pattern, _)| is(pattern))
        .map(|(_, party)| Section::Party(*party));
    party.or_else(|| {
        OTHER_HEADINGS
            .iter()
            .any(|pattern| is(pattern))
            .then_some(Section::Other)
    })
}

/// The business term `field` is in the section of `party`, if the standard
/// has one: the party delivered to has a name but no VAT identifier.
///
/// | Field          | Seller | Buyer | Deliver to | Payee | Tax representative | Debited |
/// |----------------|--------|-------|------------|-------|--------------------|---------|
/// | name           | BT-27  | BT-44 | BT-70      | BT-59 | BT-62              | none    |
/// | VAT identifier | BT-31  | BT-48 | none       | none  | BT-63              | none    |
/// | account        | BT-84  | BT-91 | none       | BT-84 | none               | BT-91   |
pub fn party_term(field: PartyField, party: Party) -> Option<u16> {
    match (field, party) {
        (PartyField::Name, Party::Seller) => Some(SELLER_NAME),
        (PartyField::Name, Party::Buyer) => Some(BUYER_NAME),
        (PartyField::Name, Party::DeliverTo) => Some(DELIVER_TO_NAME),
        (PartyField::Name, Party::Payee) => Some(PAYEE_NAME),
        (PartyField::Name, Party::TaxRepresentative) => Some(TAX_REPRESENTATIVE_NAME),
        (PartyField::VatIdentifier, Party::Seller) => Some(SELLER_VAT_IDENTIFIER),
        (PartyField::VatIdentifier, Party::Buyer) => Some(BUYER_VAT_IDENTIFIER),
        (PartyField::VatIdentifier, Party::TaxRepresentative) => {
            Some(TAX_REPRESENTATIVE_VAT_IDENTIFIER)
        }
        (PartyField::Account, Party::Seller | Party::Payee) => Some(PAYMENT_ACCOUNT),
        (PartyField::Account, Party::Buyer | Party::Debited) => Some(DEBITED_ACCOUNT),
        (PartyField::Name | PartyField::VatIdentifier, Party::Debited)
        | (PartyField::VatIdentifier, Party::DeliverTo | Party::Payee)
        | (PartyField::Account, Party::DeliverTo | Party::TaxRepresentative) => None,
    }
}

/// The business term a field of a party stands for outside any section,
/// if any: an account is the one to transfer to; a name or a VAT identifier
/// could be anyone's.
pub fn unsectioned_term(field: PartyField) -> Option<u16> {
    match field {
        PartyField::Account => Some(PAYMENT_ACCOUNT),
        PartyField::Name | PartyField::VatIdentifier => None,
    }
}

/// Words that say the amount is taken from an account: the taking itself,
/// or the mandate it is taken under. The mere name of the means, as
/// "Lastschrift", is not among them: "Zahlbar per Überweisung oder
/// SEPA-Lastschrift" offers it, and the IBAN after it is the seller's.
const DIRECT_DEBIT_WORDS: &[Word] = &[
    Is("abgebucht"),
    Is("abbuchen"),
    Is("eingezogen"),
    Is("mandatsreferenz"),
    Is("debited"),
    Is("prélevé"),
];

/// Whether a line says that the amount is taken by direct debit, which
/// opens a section of the account debited.
///
/// | Words                                               | Direct debit |
/// |-----------------------------------------------------|--------------|
/// | `Der` `Betrag` `wird` `abgebucht` `von:`           | yes          |
/// | `Mandatsreferenz:` `M-17`                          | yes          |
/// | `The` `amount` `will` `be` `debited`               | yes          |
/// | `Zahlbar` `per` `Überweisung` `oder` `SEPA-Lastschrift` | no (offered, not taken) |
/// | `Eine` `Lastschrift` `ist` `nicht` `möglich.`      | no           |
/// | `Payment` `by` `direct` `debit`                    | no (the means only) |
/// | `Bitte` `überweisen` `Sie`                         | no           |
pub fn says_direct_debit(words: &[&str]) -> bool {
    words.iter().any(|word| {
        let normal = normalized(word);
        DIRECT_DEBIT_WORDS
            .iter()
            .any(|pattern| matches(*pattern, &normal))
    })
}

/// Notes the zone for additions of a letter window prints above the
/// address. A note of one word is the whole of its line: as the first word
/// of a line, "Personal" may as well begin the name of a firm. A note of
/// several words may start a line with more after it.
const WINDOW_NOTE_WORDS: &[&[Word]] = &[
    &[Is("persönlich")],
    &[Is("vertraulich")],
    &[Is("einschreiben")],
    &[Is("eilzustellung")],
    &[Is("personal")],
    &[Is("confidential")],
    &[Is("registered")],
    &[Is("personnel")],
    &[Is("confidentiel")],
    &[Is("recommandé")],
];
const WINDOW_NOTE_STARTS: &[&[Word]] = &[
    &[Is("bei"), Is("unzustellbarkeit")],
    &[Is("einschreiben"), Is("rückschein")],
    &[Is("einschreiben"), Is("einwurf")],
];

/// Whether a line is a note of the zone for additions of a letter window,
/// which is no line of the address below it.
///
/// | Words                                 | Note |
/// |---------------------------------------|------|
/// | `Persönlich`                          | yes  |
/// | `PERSONAL`                            | yes  |
/// | `Einschreiben` `Rückschein`           | yes  |
/// | `Bei` `Unzustellbarkeit` `zurück`     | yes  |
/// | `Personal` `Partner` `GmbH`           | no (a note of one word is a line alone) |
/// | `Persönliche` `Beratung` `GmbH`       | no   |
/// | `Kunden` `AG`                         | no   |
pub fn is_window_note(words: &[&str]) -> bool {
    let normal: Vec<String> = words.iter().map(|word| normalized(word)).collect();
    let whole = WINDOW_NOTE_WORDS
        .iter()
        .any(|pattern| is_whole_line(pattern, &normal));
    let starts = WINDOW_NOTE_STARTS
        .iter()
        .any(|pattern| starts_line(pattern, &normal));
    whole || starts
}

/// Labels printed beside the lines of an address, which belong to it.
const ADDRESS_LABELS: &[&[Word]] = &[
    &[Is("anschrift")],
    &[Is("adresse")],
    &[Is("address")],
    &[Is("postanschrift")],
];

/// Whether `words` are a label of an address, as "Anschrift:".
///
/// | Words          | Address |
/// |----------------|---------|
/// | `Anschrift:`   | yes     |
/// | `Address`      | yes     |
/// | `Nummer`       | no      |
/// | `:` `549910`   | no      |
pub fn names_address(words: &[&str]) -> bool {
    let normal: Vec<String> = words.iter().map(|word| normalized(word)).collect();
    ADDRESS_LABELS
        .iter()
        .any(|pattern| is_whole_line(pattern, &normal))
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
    /// What the label names where it stands: what the label itself names,
    /// or the term of the field of the party whose section it stands in.
    pub names: Names,
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
                    names: label.names,
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
    use super::{
        CHARGES, INVOICE_NUMBER, ISSUE_DATE, Names, Party, Section, TOTAL_WITH_VAT, find, heading,
        is_window_note, says_direct_debit,
    };

    fn found(words: &[&str]) -> Vec<(u16, usize, usize)> {
        find(words)
            .iter()
            .filter_map(|found| match found.label.names {
                Names::Term(term) => Some((term, found.first, found.end)),
                Names::Nothing | Names::PartyField(_) => None,
            })
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
        let rows: [(&[&str], Option<Section>); 7] = [
            (&["Verkäufer"], Some(Section::Party(Party::Seller))),
            (
                &["Käufer/Leistungsempfänger:"],
                Some(Section::Party(Party::Buyer)),
            ),
            (
                &["Abweichender", "Warenempfänger"],
                Some(Section::Party(Party::DeliverTo)),
            ),
            (&["Bankverbindung"], Some(Section::Other)),
            (&["Bank", "details:"], Some(Section::Other)),
            (&["Verkäufer", "Muster", "GmbH"], None),
            (&["Name:"], None),
        ];
        for (words, expected) in rows {
            assert_eq!(heading(words), expected, "{words:?}");
        }
    }

    #[test]
    fn direct_debit_is_said_after_its_table() {
        let rows: [(&[&str], bool); 7] = [
            (&["Der", "Betrag", "wird", "abgebucht", "von:"], true),
            (&["Mandatsreferenz:", "M-17"], true),
            (&["The", "amount", "will", "be", "debited"], true),
            (
                &["Zahlbar", "per", "Überweisung", "oder", "SEPA-Lastschrift"],
                false,
            ),
            (&["Eine", "Lastschrift", "ist", "nicht", "möglich."], false),
            (&["Payment", "by", "direct", "debit"], false),
            (&["Bitte", "überweisen", "Sie"], false),
        ];
        for (words, expected) in rows {
            assert_eq!(says_direct_debit(words), expected, "{words:?}");
        }
    }

    #[test]
    fn window_notes_follow_their_table() {
        let rows: [(&[&str], bool); 7] = [
            (&["Persönlich"], true),
            (&["PERSONAL"], true),
            (&["Einschreiben", "Rückschein"], true),
            (&["Bei", "Unzustellbarkeit", "zurück"], true),
            (&["Personal", "Partner", "GmbH"], false),
            (&["Persönliche", "Beratung", "GmbH"], false),
            (&["Kunden", "AG"], false),
        ];
        for (words, expected) in rows {
            assert_eq!(is_window_note(words), expected, "{words:?}");
        }
    }
}
