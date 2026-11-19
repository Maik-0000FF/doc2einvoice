// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The validator as a whole: the XML Schema and the rule sets of a profile,
//! applied to one document.
//!
//! Input table. Each row is one test below.
//!
//! | document                                         | expected                                     |
//! |--------------------------------------------------|----------------------------------------------|
//! | every document of the corpus, as XRechnung       | schema verdict as the reference; one result  |
//! |                                                  | per rule set of the profile, in its order    |
//! | the published XRechnung test invoices            | some conform: schema valid and rules met     |
//! | a UBL invoice with an element the schema lacks   | not schema valid, finding names a `cvc` rule |
//! | the same invoice                                 | the rules are applied all the same           |
//! | an empty invoice, as EN 16931                    | only the rule set of the standard is applied |
//! | a document that is no invoice                    | `UnsupportedDocument`                        |
//! | text that is not XML                             | `Malformed`                                  |

mod support;

use doc2einvoice_validation::{
    Profile, RuleSet, Syntax, ValidationError, Validator, XmlSchemaInfo,
};
use support::Fault;

/// Prefix of the codes of the validation rules XML Schema defines.
const SCHEMA_RULE_PREFIX: &str = "cvc-";

const INVOICE_WITH_UNKNOWN_ELEMENT: &str = r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"><Unheard/></Invoice>"#;
const EMPTY_INVOICE: &str =
    r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#;

#[test]
fn the_corpus_is_judged_as_its_parts_are() -> Result<(), Fault> {
    let mut validator = Validator::new();
    let mut conforming = 0_usize;

    for syntax in Syntax::ALL {
        let verdicts = support::schema_verdicts(XmlSchemaInfo::of(syntax).key)?;
        for case in support::schema_cases(syntax)? {
            let report = validator.validate(&case.xml, Profile::XRechnung)?;

            assert_eq!(
                Some(&report.is_schema_valid()),
                verdicts.get(&case.name),
                "schema verdict of {}",
                case.name
            );
            assert_eq!(
                report
                    .results
                    .iter()
                    .map(|result| result.rule_set)
                    .collect::<Vec<_>>(),
                Profile::XRechnung.rule_sets(syntax),
                "rule sets applied to {}",
                case.name
            );
            if report.conforms() {
                conforming += 1;
            }
        }
    }

    assert!(conforming > 0, "no document of the corpus conforms");
    Ok(())
}

#[test]
fn a_document_the_schema_refuses_is_reported_with_the_rule_it_breaks() -> Result<(), Fault> {
    let report = Validator::new().validate(INVOICE_WITH_UNKNOWN_ELEMENT, Profile::En16931)?;

    assert!(!report.is_schema_valid());
    assert!(!report.conforms());
    assert!(
        report
            .schema_findings
            .iter()
            .all(|finding| finding.code.starts_with(SCHEMA_RULE_PREFIX)),
        "{:?}",
        report.schema_findings
    );
    Ok(())
}

#[test]
fn the_rules_are_applied_to_a_document_the_schema_refuses() -> Result<(), Fault> {
    let report = Validator::new().validate(INVOICE_WITH_UNKNOWN_ELEMENT, Profile::En16931)?;

    assert!(!report.satisfies_rules());
    assert!(report.findings().next().is_some());
    Ok(())
}

#[test]
fn a_profile_applies_its_rule_sets_only() -> Result<(), Fault> {
    let report = Validator::new().validate(EMPTY_INVOICE, Profile::En16931)?;

    assert_eq!(report.syntax, Syntax::Ubl);
    assert_eq!(
        report
            .results
            .iter()
            .map(|result| result.rule_set)
            .collect::<Vec<_>>(),
        [RuleSet::En16931Ubl]
    );
    Ok(())
}

#[test]
fn what_is_not_an_invoice_is_refused() {
    let mut validator = Validator::new();

    assert!(matches!(
        validator.validate("<html/>", Profile::En16931),
        Err(ValidationError::UnsupportedDocument { element }) if element == "html"
    ));
    assert!(matches!(
        validator.validate("not xml", Profile::En16931),
        Err(ValidationError::Malformed(_))
    ));
}
