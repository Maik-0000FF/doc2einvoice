// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Every business term and business group of EN 16931 has its place in the
//! model.
//!
//! A field, variant or type names the business terms and groups it holds in
//! parentheses in the documentation directly above it, such as "(BT-37,
//! BT-52)". Only those count: an identifier mentioned in passing, in a
//! sentence outside parentheses or in the documentation of a module, does not
//! give the term a place.

use doc2einvoice_model::{LAST_GROUP, TERMS, Term, group_id};

/// The sources of the model.
const SOURCES: [&str; 6] = [
    include_str!("../src/delivery.rs"),
    include_str!("../src/invoice.rs"),
    include_str!("../src/line.rs"),
    include_str!("../src/party.rs"),
    include_str!("../src/payment.rs"),
    include_str!("../src/totals.rs"),
];

const DOC_COMMENT: &str = "///";
const ATTRIBUTE: &str = "#[";

/// The identifiers given in parentheses in `documentation`.
///
/// | Documentation                         | Identifiers      |
/// |---------------------------------------|------------------|
/// | The city (BT-37, BT-52).              | BT-37, BT-52     |
/// | All amounts, except BT-111 (BT-5).    | BT-5             |
/// | The rate (BT-119); absent for O.      | BT-119           |
/// | A note (see the group).               | none             |
fn identifiers_in_parentheses(documentation: &str) -> Vec<String> {
    documentation
        .split('(')
        .skip(1)
        .filter_map(|after| after.split(')').next())
        .flat_map(|inside| inside.split(','))
        .map(str::trim)
        .filter(|part| part.starts_with("BT-") || part.starts_with("BG-"))
        .map(str::to_owned)
        .collect()
}

/// The identifiers each item of `source` names in the documentation directly
/// above it. A blank line ends a documentation block; attributes between the
/// documentation and the item do not.
fn named_identifiers(source: &str) -> Vec<String> {
    let mut named = Vec::new();
    let mut block = String::new();
    for line in source.lines().map(str::trim) {
        if let Some(text) = line.strip_prefix(DOC_COMMENT) {
            block.push_str(text);
            block.push(' ');
        } else if !line.starts_with(ATTRIBUTE) {
            if !line.is_empty() {
                named.extend(identifiers_in_parentheses(&block));
            }
            block.clear();
        }
    }
    named
}

fn all_named() -> Vec<String> {
    SOURCES
        .iter()
        .flat_map(|source| named_identifiers(source))
        .collect()
}

fn missing(ids: impl Iterator<Item = String>) -> Vec<String> {
    let named = all_named();
    ids.filter(|id| !named.contains(id)).collect()
}

#[test]
fn the_documentation_is_read_as_its_table_says() {
    assert_eq!(
        identifiers_in_parentheses("The city (BT-37, BT-52)."),
        ["BT-37", "BT-52"]
    );
    assert_eq!(
        identifiers_in_parentheses("All amounts, except BT-111 (BT-5)."),
        ["BT-5"]
    );
    assert_eq!(
        identifiers_in_parentheses("The rate (BT-119); absent for O."),
        ["BT-119"]
    );
    assert!(identifiers_in_parentheses("A note (see the group).").is_empty());
}

#[test]
fn only_documentation_directly_above_an_item_counts() {
    let source = "//! The module (BT-1).\n\n/// Prose naming BT-2 only.\n\n/// The field (BT-3).\n#[must_use]\npub field: u8,\n";
    assert_eq!(named_identifiers(source), ["BT-3"]);
}

#[test]
fn every_business_term_has_a_place() {
    let missing = missing(TERMS.iter().map(Term::id));
    assert!(missing.is_empty(), "not in the model: {missing:?}");
}

#[test]
fn every_business_group_has_a_place() {
    let missing = missing((1..=LAST_GROUP).map(group_id));
    assert!(missing.is_empty(), "not in the model: {missing:?}");
}
