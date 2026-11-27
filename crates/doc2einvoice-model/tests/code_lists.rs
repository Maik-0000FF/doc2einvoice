// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The closed code lists of the model are those of the published rules of
//! EN 16931, read from the rule files in this repository.

use doc2einvoice_model::{CURRENCY_CODES, INVOICE_TYPE_CODES, UNIT_CODES, VatCategory};

/// The published rules of EN 16931, one file per syntax.
const RULE_FILES: [(&str, &str); 2] = [
    (
        "UBL",
        include_str!("../../../rulesets/en16931/EN16931-UBL-validation-preprocessed.sch"),
    ),
    (
        "CII",
        include_str!("../../../rulesets/en16931/EN16931-CII-validation-preprocessed.sch"),
    ),
];

/// The rule that lists the VAT category codes.
const VAT_CATEGORY_RULE: &str = "BR-CL-18";

/// The rule that lists the currency codes.
const CURRENCY_RULE: &str = "BR-CL-04";

/// The rule that lists the codes of the kinds of invoices and credit notes.
const INVOICE_TYPE_RULE: &str = "BR-CL-01";

/// The rule that lists the codes of units of measure.
const UNIT_RULE: &str = "BR-CL-23";

/// The test of the Schematron assertion with the identifier `rule`.
fn rule_test<'a>(document: &'a roxmltree::Document<'a>, rule: &str) -> Option<&'a str> {
    document
        .descendants()
        .find(|node| node.tag_name().name() == "assert" && node.attribute("id") == Some(rule))
        .and_then(|assertion| assertion.attribute("test"))
}

/// The codes a code-list test allows: the one quoted literal of the XPath
/// expression that holds more than white space, split at white space.
///
/// | Test                                                         | Codes   |
/// |--------------------------------------------------------------|---------|
/// | not(contains(., ' ')) and contains(' AE B ', concat(...))    | AE, B   |
/// | contains( ' S Z ',concat(' ',.,' ') )                        | S, Z    |
/// | contains(., ' ')                                             | none    |
fn allowed_codes(test: &str) -> Vec<String> {
    test.split('\'')
        .skip(1)
        .step_by(2)
        .find(|literal| !literal.trim().is_empty())
        .map(|literal| literal.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The codes of every list of a code-list test, each once: every quoted
/// literal of the XPath expression that holds more than white space, split
/// at white space. A rule may list its codes in parts, as the UBL syntax
/// lists the kinds of invoices and of credit notes.
///
/// | Test                                                    | Codes         |
/// |---------------------------------------------------------|---------------|
/// | contains(' 380 381 ', x) or contains(' 381 396 ', x)    | 380, 381, 396 |
/// | not(contains(., ' ')) and contains(' AE B ', x)         | AE, B         |
/// | contains(., ' ')                                        | none          |
fn every_allowed_code(test: &str) -> Vec<String> {
    let mut codes: Vec<String> = test
        .split('\'')
        .skip(1)
        .step_by(2)
        .flat_map(str::split_whitespace)
        .map(str::to_owned)
        .collect();
    codes.sort_unstable();
    codes.dedup();
    codes
}

/// The codes the rule `rule` lists in each syntax, read with `read` and
/// sorted, each with the name of its syntax.
fn published(rule: &str, read: fn(&str) -> Vec<String>) -> Vec<(&'static str, Vec<String>)> {
    RULE_FILES
        .iter()
        .map(|(syntax, rules)| {
            let document = roxmltree::Document::parse(rules)
                .unwrap_or_else(|error| unreachable!("{syntax}: {error}"));
            let test = rule_test(&document, rule)
                .unwrap_or_else(|| unreachable!("{syntax}: {rule} not found"));
            let mut codes = read(test);
            codes.sort_unstable();
            (*syntax, codes)
        })
        .collect()
}

/// `codes` sorted, as the published ones are compared.
fn sorted(codes: &[&'static str]) -> Vec<&'static str> {
    let mut sorted = codes.to_vec();
    sorted.sort_unstable();
    sorted
}

#[test]
fn the_literal_is_read_as_its_table_says() {
    assert_eq!(
        allowed_codes("not(contains(., ' ')) and contains(' AE B ', concat(' ', ., ' '))"),
        ["AE", "B"]
    );
    assert_eq!(
        allowed_codes("contains( ' S Z ',concat(' ',.,' ') )"),
        ["S", "Z"]
    );
    assert!(allowed_codes("contains(., ' ')").is_empty());
}

#[test]
fn every_literal_is_read_as_its_table_says() {
    assert_eq!(
        every_allowed_code("contains(' 380 381 ', x) or contains(' 381 396 ', x)"),
        ["380", "381", "396"]
    );
    assert_eq!(
        every_allowed_code("not(contains(., ' ')) and contains(' AE B ', x)"),
        ["AE", "B"]
    );
    assert!(every_allowed_code("contains(., ' ')").is_empty());
}

#[test]
fn the_vat_categories_are_those_of_the_rules() {
    let codes: Vec<&str> = VatCategory::ALL
        .iter()
        .map(|category| category.code())
        .collect();
    for (syntax, published) in published(VAT_CATEGORY_RULE, allowed_codes) {
        assert_eq!(sorted(&codes), published, "{syntax}");
    }
}

#[test]
fn the_currencies_are_those_of_the_rules() {
    for (syntax, published) in published(CURRENCY_RULE, allowed_codes) {
        assert_eq!(sorted(&CURRENCY_CODES), published, "{syntax}");
    }
}

#[test]
fn the_invoice_types_are_those_of_the_rules() {
    for (syntax, published) in published(INVOICE_TYPE_RULE, every_allowed_code) {
        assert_eq!(sorted(&INVOICE_TYPE_CODES), published, "{syntax}");
    }
}

#[test]
fn the_units_are_those_of_the_rules() {
    for (syntax, published) in published(UNIT_RULE, allowed_codes) {
        assert_eq!(sorted(&UNIT_CODES), published, "{syntax}");
    }
}
