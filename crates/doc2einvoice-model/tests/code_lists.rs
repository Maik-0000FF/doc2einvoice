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

/// The codes a code-list rule allows: the one quoted literal of its test that
/// holds more than white space, split at white space.
fn allowed_codes(rules: &str, rule: &str) -> Vec<String> {
    let marker = format!("id=\"{rule}\"");
    let Some(start) = rules.find(&marker) else {
        return Vec::new();
    };
    let Some(test) = rules[start..].split("test=\"").nth(1) else {
        return Vec::new();
    };
    let test = test.split('"').next().unwrap_or_default();
    test.split('\'')
        .skip(1)
        .step_by(2)
        .find(|literal| !literal.trim().is_empty())
        .map(|literal| literal.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[test]
fn the_vat_categories_are_those_of_the_rules() {
    let mut model: Vec<&str> = VatCategory::ALL
        .iter()
        .map(|category| category.code())
        .collect();
    model.sort_unstable();
    for (syntax, rules) in RULE_FILES {
        let mut published = allowed_codes(rules, VAT_CATEGORY_RULE);
        published.sort_unstable();
        assert!(
            !published.is_empty(),
            "{syntax}: {VAT_CATEGORY_RULE} not found"
        );
        assert_eq!(model, published, "{syntax}");
    }
}
