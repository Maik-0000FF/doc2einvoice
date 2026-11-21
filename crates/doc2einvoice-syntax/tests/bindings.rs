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

use doc2einvoice_model::{Code, Invoice};
use doc2einvoice_syntax::bindings::{Binding, bindings};
use doc2einvoice_syntax::namespaces::{
    CII_AGGREGATE, CII_INVOICE, CII_QUALIFIED, CII_UNQUALIFIED, UBL_AGGREGATE, UBL_BASIC,
    UBL_CREDIT_NOTE, UBL_INVOICE,
};
use doc2einvoice_syntax::{Gap, Syntax, UblDocument, WriteError, write_cii, write_ubl};
use support::{CREDIT_NOTE, EN16931, Payment, complete, exempt, paid_by_card, with_vat_point_code};
use xee_xpath::context::StaticContextBuilder;
use xee_xpath::{Documents, Item, Queries, Query};

/// The prefixes the paths use.
const NAMESPACES: [(&str, &str); 8] = [
    ("Invoice", UBL_INVOICE),
    ("CreditNote", UBL_CREDIT_NOTE),
    ("cac", UBL_AGGREGATE),
    ("cbc", UBL_BASIC),
    ("rsm", CII_INVOICE),
    ("ram", CII_AGGREGATE),
    ("udt", CII_UNQUALIFIED),
    ("qdt", CII_QUALIFIED),
];

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
        context.namespaces(NAMESPACES);
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
