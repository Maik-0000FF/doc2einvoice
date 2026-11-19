// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conformance: every rule set must report, for every document of the corpus,
//! the findings the reference implementation reports: the same rules, on the
//! same nodes, the same number of times.
//!
//! The expected results under `tests/conformance/expected` come from running
//! the published rules with Saxon (Java), the processor the official
//! validators use. They are produced by `tools/regenerate-conformance.sh`.
//!
//! What is compared and what is not:
//!
//! - Compared: the rule, the node and the number of findings.
//! - Not compared: the text of a finding and its flag.
//! - Where the reference stops at a rule it cannot evaluate, it yields no
//!   report. For those documents the rules the interpreter cannot evaluate are
//!   pinned instead; that expectation is checked by hand, not by the reference.
//!
//! The target is complete agreement. One difference fails the test.

mod support;

use std::collections::BTreeSet;

use doc2einvoice_validation::RuleSet;
use doc2einvoice_validation::schematron::Report;
use support::{Expected, Fault, Located};

/// How many differences are listed before the rest is only counted.
const LISTED_DIFFERENCES: usize = 20;

/// How the findings of a document differ from what is expected, if they do.
fn difference_from_expected(report: &Report, expected: &Expected) -> Option<String> {
    match expected {
        Expected::Reference(reference) => {
            let found = support::located_findings(report);
            (found != *reference).then(|| {
                format!(
                    "not in the reference {:?}, missing {:?}",
                    only_in(&found, reference),
                    only_in(reference, &found)
                )
            })
        }
        Expected::ReferenceStopped(pinned) => {
            let found = support::unevaluable_findings(report);
            (found != *pinned)
                .then(|| format!("rules that cannot be evaluated: {found:?}, pinned {pinned:?}"))
        }
    }
}

/// The findings of `left` that `right` does not have, counting repetitions:
/// a finding that `left` has three times and `right` twice is listed once.
fn only_in(left: &[Located], right: &[Located]) -> Vec<Located> {
    let mut remaining = right.to_vec();
    left.iter()
        .filter(
            |finding| match remaining.iter().position(|other| other == *finding) {
                Some(index) => {
                    remaining.swap_remove(index);
                    false
                }
                None => true,
            },
        )
        .cloned()
        .collect()
}

fn agrees_with_the_reference(rule_set: RuleSet) -> Result<(), Fault> {
    let info = rule_set.info();
    let expected = support::expected_results(info.key)?;
    let schema = rule_set.compile()?;
    let known: BTreeSet<&str> = schema.assertion_ids().collect();
    let cases = support::cases(info.syntax)?;

    assert_eq!(
        cases
            .iter()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>(),
        expected.keys().map(String::as_str).collect::<Vec<_>>(),
        "the corpus and the expected results of {} name different cases",
        info.key
    );

    let mut differences = Vec::new();
    for case in &cases {
        let report = schema.validate(&case.xml)?;
        if let Some(difference) = difference_from_expected(&report, &expected[&case.name]) {
            differences.push(format!("{}: {difference}", case.name));
        }

        // The outcomes the publisher states for its own test documents, for
        // the rules that belong to this rule set.
        let reported: BTreeSet<&str> = report
            .findings()
            .iter()
            .map(|finding| finding.assertion_id.as_str())
            .collect();
        for rule in case
            .must_pass
            .iter()
            .filter(|rule| known.contains(rule.as_str()))
        {
            if reported.contains(rule.as_str()) {
                differences.push(format!(
                    "{}: {rule} is reported, the publisher expects it to pass",
                    case.name
                ));
            }
        }
        for rule in case
            .must_fire
            .iter()
            .filter(|rule| known.contains(rule.as_str()))
        {
            if !reported.contains(rule.as_str()) {
                differences.push(format!(
                    "{}: {rule} is not reported, the publisher expects it",
                    case.name
                ));
            }
        }
    }

    assert!(
        differences.is_empty(),
        "{}: {} differences in {} documents:\n{}",
        info.key,
        differences.len(),
        cases.len(),
        differences
            .iter()
            .take(LISTED_DIFFERENCES)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

#[test]
fn en16931_ubl_agrees_with_the_reference() -> Result<(), Fault> {
    agrees_with_the_reference(RuleSet::En16931Ubl)
}

#[test]
fn en16931_cii_agrees_with_the_reference() -> Result<(), Fault> {
    agrees_with_the_reference(RuleSet::En16931Cii)
}

#[test]
fn xrechnung_ubl_agrees_with_the_reference() -> Result<(), Fault> {
    agrees_with_the_reference(RuleSet::XRechnungUbl)
}

#[test]
fn xrechnung_cii_agrees_with_the_reference() -> Result<(), Fault> {
    agrees_with_the_reference(RuleSet::XRechnungCii)
}

/// The versions stated in the code are the versions the corpus and the
/// reference run are pinned to.
#[test]
fn the_rule_sets_are_the_pinned_versions() -> Result<(), Fault> {
    let sources = support::sources()?;
    let pinned = |key: &str| sources.get(key).map(String::as_str);

    for rule_set in [RuleSet::En16931Ubl, RuleSet::En16931Cii] {
        assert_eq!(Some(rule_set.info().version), pinned("EN16931_VERSION"));
        assert_eq!(Some(rule_set.info().published), pinned("EN16931_PUBLISHED"));
    }
    for rule_set in [RuleSet::XRechnungUbl, RuleSet::XRechnungCii] {
        assert_eq!(
            Some(rule_set.info().version),
            pinned("XRECHNUNG_RULES_VERSION")
        );
        assert_eq!(
            Some(rule_set.info().published),
            pinned("XRECHNUNG_RULES_PUBLISHED")
        );
    }
    Ok(())
}
