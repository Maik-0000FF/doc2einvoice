// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Every business term and group is written at its place: for each one, the
//! path that holds it in UBL and in CII (`data/bindings.tsv`, alternatives
//! joined with `|`) selects a node in at least one of the written invoices.
//!
//! Passing the validation rules does not show this: a value written to a
//! wrong but permitted element breaks no rule.

mod support;

use doc2einvoice_model::Invoice;
use doc2einvoice_syntax::{UblDocument, write_cii, write_ubl};
use support::{EN16931, Payment, complete, exempt, paid_by_card, with_vat_point_code};
use xee_xpath::context::StaticContextBuilder;
use xee_xpath::{Documents, Item, Queries, Query};

/// For each business term or group: its identifier, its path in a UBL
/// invoice and its path in a Cross Industry Invoice, separated by tabs, after
/// a header line.
const BINDINGS: &str = include_str!("data/bindings.tsv");

/// The prefixes the paths use.
const NAMESPACES: [(&str, &str); 7] = [
    (
        "Invoice",
        "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2",
    ),
    (
        "cac",
        "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
    ),
    (
        "cbc",
        "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
    ),
    (
        "rsm",
        "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100",
    ),
    (
        "ram",
        "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100",
    ),
    (
        "udt",
        "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100",
    ),
    (
        "qdt",
        "urn:un:unece:uncefact:data:standard:QualifiedDataType:100",
    ),
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

fn text(document: Vec<u8>) -> String {
    String::from_utf8(document).unwrap_or_else(|error| unreachable!("{error}"))
}

fn ubl_documents() -> Vec<String> {
    invoices()
        .iter()
        .filter_map(|invoice| write_ubl(invoice, UblDocument::Invoice).ok())
        .map(text)
        .collect()
}

fn cii_documents() -> Vec<String> {
    invoices()
        .iter()
        .map(|invoice| text(write_cii(invoice).unwrap_or_else(|error| unreachable!("{error}"))))
        .collect()
}

/// Whether `path` selects a node in any of `documents`.
fn selects(documents: &[String], path: &str) -> bool {
    let mut context = StaticContextBuilder::default();
    context.namespaces(NAMESPACES);
    let queries = Queries::new(context);
    let query = queries
        .sequence(&format!("exists({path})"))
        .unwrap_or_else(|error| unreachable!("{path}: {error}"));
    documents.iter().any(|document| {
        let mut parsed = Documents::new();
        let handle = parsed
            .add_string_without_uri(document)
            .unwrap_or_else(|error| unreachable!("{error}"));
        let root = parsed
            .document_node(handle)
            .unwrap_or_else(|| unreachable!("no root"));
        let found = query
            .execute(&mut parsed, &Item::Node(root))
            .unwrap_or_else(|error| unreachable!("{path}: {error}"));
        found
            .effective_boolean_value()
            .unwrap_or_else(|error| unreachable!("{path}: {error:?}"))
    })
}

/// The identifiers whose path in column `column` selects nothing.
fn unplaced(documents: &[String], column: usize, skipped: &[&str]) -> Vec<String> {
    BINDINGS
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let id = *fields.first()?;
            let path = *fields.get(column)?;
            (!skipped.contains(&id) && !selects(documents, path)).then(|| id.to_owned())
        })
        .collect()
}

/// The column of the paths in UBL and in CII.
const UBL_COLUMN: usize = 1;
const CII_COLUMN: usize = 2;

/// Business terms and groups that have a row in the table.
const ROWS: usize = 196;

#[test]
fn the_table_has_a_row_for_every_term_and_group() {
    assert_eq!(BINDINGS.lines().skip(1).count(), ROWS);
}

#[test]
fn every_term_has_its_place_in_ubl() {
    let unplaced = unplaced(&ubl_documents(), UBL_COLUMN, &NOT_IN_UBL);
    assert!(
        unplaced.is_empty(),
        "not at their place in UBL: {unplaced:?}"
    );
}

#[test]
fn every_term_has_its_place_in_cii() {
    let unplaced = unplaced(&cii_documents(), CII_COLUMN, &[]);
    assert!(
        unplaced.is_empty(),
        "not at their place in CII: {unplaced:?}"
    );
}
