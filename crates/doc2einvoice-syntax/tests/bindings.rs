// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Every business term and group is written at its place: for each one, the
//! path that holds it (the bindings of the crate, alternatives joined with `|`)
//! selects a node in at least one of the written documents, in a UBL invoice,
//! a UBL credit note and a Cross Industry Invoice.
//!
//! Passing the validation rules does not show this: a value written to a
//! wrong but permitted element breaks no rule.

mod support;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use doc2einvoice_model::{Code, Invoice};
use doc2einvoice_syntax::bindings::{Binding, PREFIXES, bindings};
use doc2einvoice_syntax::{Gap, Syntax, UblDocument, WriteError, write_cii, write_ubl};
use support::{
    CREDIT_NOTE, EN16931, NAMED_CATEGORIES, NAMED_COUNTRIES, NAMED_CURRENCIES, NAMED_TYPE,
    NAMED_VAT_POINT, NamedPayment, Payment, complete, exempt, named, named_attachment,
    named_invoice, named_with_vat_point_code, paid_by_card, with_vat_point_code,
};
use xee_xpath::context::StaticContextBuilder;
use xee_xpath::{Documents, Item, Queries, Query};

/// What UBL cannot write: a payment card, whose network UBL requires and no
/// business term holds.
const NOT_IN_UBL: [&str; 3] = ["BG-18", "BT-87", "BT-88"];

/// The invoices that, together, give every business term a value: the
/// complete invoice paid by transfer, paid by direct debit, with the VAT
/// point as a code, paid by card, and exempt from VAT.
fn invoices() -> Vec<Invoice> {
    vec![
        complete(Payment::CreditTransfer, EN16931),
        complete(Payment::DirectDebit, EN16931),
        with_vat_point_code(EN16931),
        paid_by_card(EN16931),
        exempt(),
    ]
}

fn as_credit_note(mut invoice: Invoice) -> Invoice {
    invoice.type_code = Code(CREDIT_NOTE.to_owned());
    invoice
}

fn text(document: Vec<u8>) -> String {
    String::from_utf8(document).unwrap_or_else(|error| unreachable!("{error}"))
}

/// Writes each invoice as UBL. Only a payment by card may be refused, and
/// only for the card network.
fn ubl(invoices: &[Invoice], document: UblDocument) -> Vec<String> {
    invoices
        .iter()
        .filter_map(|invoice| match write_ubl(invoice, document) {
            Ok(written) => Some(text(written)),
            Err(WriteError::NotExpressible {
                syntax: Syntax::Ubl,
                gap: Gap::CardNetworkMissing,
            }) => None,
            Err(error) => unreachable!("{error}"),
        })
        .collect()
}

fn cii(invoices: &[Invoice]) -> Vec<String> {
    invoices
        .iter()
        .map(|invoice| text(write_cii(invoice).unwrap_or_else(|error| unreachable!("{error}"))))
        .collect()
}

/// The written documents, parsed once, with the queries compiled against the
/// namespaces of the paths.
struct Parsed {
    documents: Documents,
    roots: Vec<Item>,
}

impl Parsed {
    fn new(texts: &[String]) -> Self {
        let mut documents = Documents::new();
        let roots = texts
            .iter()
            .map(|text| {
                let handle = documents
                    .add_string_without_uri(text)
                    .unwrap_or_else(|error| unreachable!("{error}"));
                let root = documents
                    .document_node(handle)
                    .unwrap_or_else(|| unreachable!("no root"));
                Item::Node(root)
            })
            .collect();
        Self { documents, roots }
    }

    /// The identifiers whose path, as `path` picks it, selects nothing.
    fn unplaced(&mut self, path: fn(&Binding) -> &str, skipped: &[&str]) -> Vec<String> {
        let mut context = StaticContextBuilder::default();
        context.namespaces(PREFIXES);
        let queries = Queries::new(context);
        let mut unplaced = Vec::new();
        for binding in bindings() {
            if skipped.contains(&binding.id) {
                continue;
            }
            let path = path(&binding);
            let query = queries
                .sequence(&format!("exists({path})"))
                .unwrap_or_else(|error| unreachable!("{path}: {error}"));
            let placed = self.roots.iter().any(|root| {
                let found = query
                    .execute(&mut self.documents, root)
                    .unwrap_or_else(|error| unreachable!("{path}: {error}"));
                found
                    .effective_boolean_value()
                    .unwrap_or_else(|error| unreachable!("{path}: {error:?}"))
            });
            if !placed {
                unplaced.push(binding.id.to_owned());
            }
        }
        unplaced
    }
}

impl Parsed {
    /// What is written at `path`, in every document.
    fn written(&mut self, path: &str) -> Vec<String> {
        let mut context = StaticContextBuilder::default();
        context.namespaces(PREFIXES);
        let query = Queries::new(context)
            .many(&format!("({path})/string()"), |_, item| {
                Ok(item.try_into_value::<String>()?)
            })
            .unwrap_or_else(|error| unreachable!("{path}: {error}"));
        let mut written = Vec::new();
        for root in &self.roots {
            written.extend(
                query
                    .execute(&mut self.documents, root)
                    .unwrap_or_else(|error| unreachable!("{path}: {error}")),
            );
        }
        written
    }

    /// The business terms at whose path, as `path` picks it, no document
    /// holds the value of the term, with what is written there instead.
    fn misplaced(
        &mut self,
        path: fn(&Binding) -> &str,
        skipped: &[&str],
        parts: fn(u16, &str) -> Vec<&str>,
    ) -> Vec<String> {
        let mut misplaced = Vec::new();
        for binding in bindings() {
            let Some(term) = term_of(binding.id) else {
                continue;
            };
            if skipped.contains(&binding.id) {
                continue;
            }
            let written = self.written(path(&binding));
            let held = written
                .iter()
                .flat_map(|value| parts(term, value))
                .any(|part| holds(part, term));
            if !held {
                misplaced.push(format!("{}: {written:?}", binding.id));
            }
        }
        misplaced
    }
}

/// The subject code of a note (BT-21) and its text (BT-22), which UBL writes
/// into one element, as `#AAI#the text`.
const UBL_NOTE_TERMS: [u16; 2] = [21, 22];
/// What UBL puts around the subject code of a note.
const UBL_NOTE_MARK: char = '#';

/// What a UBL document writes for the term `term` in `written`: the parts
/// of a note for the two terms that share it, and the whole otherwise.
///
/// | Term  | Written        | Parts            |
/// |-------|----------------|------------------|
/// | BT-21 | `#BT-21#BT-22` | `BT-21`, `BT-22` |
/// | BT-22 | `#BT-21#BT-22` | `BT-21`, `BT-22` |
/// | BT-22 | `BT-22`        | `BT-22`          |
/// | BT-1  | `BT-1#x`       | `BT-1#x`         |
fn ubl_parts(term: u16, written: &str) -> Vec<&str> {
    if UBL_NOTE_TERMS.contains(&term) {
        written
            .split(UBL_NOTE_MARK)
            .filter(|part| !part.is_empty())
            .collect()
    } else {
        vec![written]
    }
}

/// What a Cross Industry Invoice writes for a term: the whole, always.
fn whole(_term: u16, written: &str) -> Vec<&str> {
    vec![written]
}

/// The number of a business term, and none for a business group, which
/// holds no value of its own.
fn term_of(id: &str) -> Option<u16> {
    id.strip_prefix("BT-")?.parse().ok()
}

/// The terms that carry a code and cannot name themselves, with the code
/// each is written as.
fn coded() -> Vec<(u16, String)> {
    let mut coded = vec![
        (3, NAMED_TYPE.to_owned()),
        (8, NAMED_VAT_POINT.to_owned()),
        (125, STANDARD.encode(named_attachment())),
    ];
    coded.extend(
        NAMED_COUNTRIES
            .iter()
            .map(|(term, code)| (*term, (*code).to_owned())),
    );
    coded.extend(
        NAMED_CURRENCIES
            .iter()
            .map(|(term, code)| (*term, (*code).to_owned())),
    );
    coded.extend(
        NAMED_CATEGORIES
            .iter()
            .map(|(term, _, code)| (*term, (*code).to_owned())),
    );
    coded
}

/// Whether `written` is the value of the term `term` in the invoice that
/// names its terms: the code of a coded term, and what names the term
/// otherwise.
fn holds(written: &str, term: u16) -> bool {
    match coded().iter().find(|(coded, _)| *coded == term) {
        Some((_, code)) => written == code,
        None => named::names(written, term),
    }
}

/// The invoices that name their terms and, together, write every one: paid
/// by transfer, by direct debit and by card, and with the VAT point as a
/// code.
fn named_invoices() -> Vec<Invoice> {
    vec![
        named_invoice(NamedPayment::CreditTransfer),
        named_invoice(NamedPayment::DirectDebit),
        named_invoice(NamedPayment::Card),
        named_with_vat_point_code(),
    ]
}

#[test]
fn values_name_their_terms_as_their_table_says() {
    let table = [
        ("BT-27", 27, true),
        ("27.5", 27, true),
        ("27.50", 27, true),
        ("2027-01-01", 27, true),
        ("20270101", 27, true),
        ("BT-28", 27, false),
        ("BT-270", 27, false),
        ("28.5", 27, false),
        ("27", 27, false),
        ("2028-01-01", 27, false),
        ("", 27, false),
    ];
    for (written, term, expected) in table {
        assert_eq!(named::names(written, term), expected, "{written}");
    }
}

#[test]
fn the_parts_of_a_ubl_value_follow_their_table() {
    let table: [(u16, &str, &[&str]); 4] = [
        (21, "#BT-21#BT-22", &["BT-21", "BT-22"]),
        (22, "#BT-21#BT-22", &["BT-21", "BT-22"]),
        (22, "BT-22", &["BT-22"]),
        (1, "BT-1#x", &["BT-1#x"]),
    ];
    for (term, written, expected) in table {
        assert_eq!(ubl_parts(term, written), expected, "{written}");
    }
}

#[test]
fn every_term_holds_its_value_in_a_ubl_invoice() {
    let misplaced = Parsed::new(&ubl(&named_invoices(), UblDocument::Invoice)).misplaced(
        |binding| binding.ubl,
        &NOT_IN_UBL,
        ubl_parts,
    );
    assert!(misplaced.is_empty(), "not their value: {misplaced:#?}");
}

#[test]
fn every_term_holds_its_value_in_a_ubl_credit_note() {
    let misplaced = Parsed::new(&ubl(&named_invoices(), UblDocument::CreditNote)).misplaced(
        |binding| binding.ubl_credit_note,
        &NOT_IN_UBL,
        ubl_parts,
    );
    assert!(misplaced.is_empty(), "not their value: {misplaced:#?}");
}

#[test]
fn every_term_holds_its_value_in_cii() {
    let misplaced =
        Parsed::new(&cii(&named_invoices())).misplaced(|binding| binding.cii, &[], whole);
    assert!(misplaced.is_empty(), "not their value: {misplaced:#?}");
}

#[test]
fn every_term_has_its_place_in_a_ubl_invoice() {
    let unplaced = Parsed::new(&ubl(&invoices(), UblDocument::Invoice))
        .unplaced(|binding| binding.ubl, &NOT_IN_UBL);
    assert!(unplaced.is_empty(), "not at their place: {unplaced:?}");
}

#[test]
fn every_term_has_its_place_in_a_ubl_credit_note() {
    let notes: Vec<Invoice> = invoices().into_iter().map(as_credit_note).collect();
    let unplaced = Parsed::new(&ubl(&notes, UblDocument::CreditNote))
        .unplaced(|binding| binding.ubl_credit_note, &NOT_IN_UBL);
    assert!(unplaced.is_empty(), "not at their place: {unplaced:?}");
}

#[test]
fn every_term_has_its_place_in_cii() {
    let unplaced = Parsed::new(&cii(&invoices())).unplaced(|binding| binding.cii, &[]);
    assert!(unplaced.is_empty(), "not at their place: {unplaced:?}");
}
