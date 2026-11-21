// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The closed code lists of the model are those of the published rules of
//! EN 16931, read from the rule files in this repository.

use doc2einvoice_model::VatCategory;

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
fn the_vat_categories_are_those_of_the_rules() {
    let mut model: Vec<&str> = VatCategory::ALL
        .iter()
        .map(|category| category.code())
        .collect();
    model.sort_unstable();
    for (syntax, rules) in RULE_FILES {
        let document = roxmltree::Document::parse(rules)
            .unwrap_or_else(|error| unreachable!("{syntax}: {error}"));
        let test = rule_test(&document, VAT_CATEGORY_RULE)
            .unwrap_or_else(|| unreachable!("{syntax}: {VAT_CATEGORY_RULE} not found"));
        let mut published = allowed_codes(test);
        published.sort_unstable();
        assert_eq!(model, published, "{syntax}");
    }
}
