// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Every business term and business group of EN 16931 has its place in the
//! model, named in the documentation of the field or type that holds it.

/// The sources of the model, where each field names its business term.
const SOURCES: [&str; 6] = [
    include_str!("../src/delivery.rs"),
    include_str!("../src/invoice.rs"),
    include_str!("../src/line.rs"),
    include_str!("../src/party.rs"),
    include_str!("../src/payment.rs"),
    include_str!("../src/totals.rs"),
];

/// The highest number of a business term in EN 16931-1:2017.
const LAST_TERM: u32 = 165;

/// BT-4 is not assigned in the standard.
const UNASSIGNED_TERMS: [u32; 1] = [4];

/// The highest number of a business group.
const LAST_GROUP: u32 = 32;

/// Whether `id` is named in the sources as a whole word, so that BT-1 is not
/// found inside BT-10.
fn is_named(id: &str) -> bool {
    SOURCES.iter().any(|source| {
        source
            .match_indices(id)
            .any(|(at, _)| !source[at + id.len()..].starts_with(|next: char| next.is_ascii_digit()))
    })
}

#[test]
fn every_business_term_has_a_place() {
    let missing: Vec<String> = (1..=LAST_TERM)
        .filter(|number| !UNASSIGNED_TERMS.contains(number))
        .map(|number| format!("BT-{number}"))
        .filter(|id| !is_named(id))
        .collect();
    assert!(missing.is_empty(), "not in the model: {missing:?}");
}

#[test]
fn every_business_group_has_a_place() {
    let missing: Vec<String> = (1..=LAST_GROUP)
        .map(|number| format!("BG-{number}"))
        .filter(|id| !is_named(id))
        .collect();
    assert!(missing.is_empty(), "not in the model: {missing:?}");
}
