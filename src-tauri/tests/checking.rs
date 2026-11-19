// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Checking invoice files from the first byte to the outcome the user
//! interface receives, on documents of the conformance corpus.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use doc2einvoice_lib::checking::outcome::{
    CheckOutcome, InvoiceCheck, ProfileKind, Refusal, SeverityKind, SyntaxKind,
};
use doc2einvoice_lib::checking::service::ValidationService;
use doc2einvoice_lib::checking::{check_again, check_file};
use doc2einvoice_validation::{MAX_NESTING_DEPTH, Profile};
use tempfile::TempDir;

/// Invoices of the XRechnung test suite that conform to XRechnung.
const XRECHNUNG_UBL: &str = "xrechnung/instances/standard/01.01a-INVOICE_ubl.xml";
const XRECHNUNG_CII: &str = "xrechnung/instances/standard/01.01a-INVOICE_uncefact.xml";

/// An example of the EN 16931 artefacts: it conforms to the standard and does
/// not claim to be an XRechnung.
const EN16931_CII: &str = "en16931/examples-cii/CII_business_example_01.xml";

type Fault = Box<dyn std::error::Error>;

fn corpus(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/conformance/corpus")
        .join(file)
}

fn checked(outcome: CheckOutcome) -> Result<InvoiceCheck, Fault> {
    match outcome {
        CheckOutcome::Checked { check } => Ok(check),
        other => Err(format!("not checked: {other:?}").into()),
    }
}

fn refusal(outcome: CheckOutcome) -> Result<Refusal, Fault> {
    match outcome {
        CheckOutcome::Refused { refusal } => Ok(refusal),
        other => Err(format!("not refused: {other:?}").into()),
    }
}

fn written(directory: &TempDir, name: &str, content: &str) -> io::Result<PathBuf> {
    let path = directory.path().join(name);
    fs::write(&path, content)?;
    Ok(path)
}

#[test]
fn an_xrechnung_is_checked_as_the_profile_it_claims() -> Result<(), Fault> {
    let service = ValidationService::start()?;

    for (file, syntax) in [
        (XRECHNUNG_UBL, SyntaxKind::Ubl),
        (XRECHNUNG_CII, SyntaxKind::Cii),
    ] {
        let check = checked(check_file(&service, &corpus(file)))?;

        assert!(check.conforms, "{file}");
        assert_eq!(check.syntax, syntax);
        assert_eq!(check.profile, ProfileKind::XRechnung);
        assert_eq!(check.suggested_profile, ProfileKind::XRechnung);
        assert!(check.specification_identifier.is_some());
        assert!(file.ends_with(&check.file_name));
        // The rules of the standard and the rules of XRechnung.
        assert_eq!(check.rule_sets.len(), 2);
        // A conforming invoice may carry warnings, but nothing fatal.
        assert!(
            check
                .rule_sets
                .iter()
                .flat_map(|rules| &rules.findings)
                .all(|finding| finding.severity != SeverityKind::Fatal)
        );
    }
    Ok(())
}

#[test]
fn an_invoice_that_claims_only_the_standard_is_checked_as_the_standard() -> Result<(), Fault> {
    let service = ValidationService::start()?;

    let check = checked(check_file(&service, &corpus(EN16931_CII)))?;

    assert_eq!(check.profile, ProfileKind::En16931);
    assert_eq!(check.suggested_profile, ProfileKind::En16931);
    assert!(check.conforms);
    Ok(())
}

#[test]
fn the_last_invoice_can_be_checked_again_as_another_profile() -> Result<(), Fault> {
    let service = ValidationService::start()?;
    let standard = checked(check_file(&service, &corpus(EN16931_CII)))?;

    let stricter = checked(check_again(&service, Profile::XRechnung))?;

    assert_eq!(stricter.file_name, standard.file_name);
    assert_eq!(stricter.profile, ProfileKind::XRechnung);
    // The suggestion is what the invoice claims; it does not follow the choice.
    assert_eq!(stricter.suggested_profile, ProfileKind::En16931);
    assert!(stricter.rule_sets.len() > standard.rule_sets.len());
    // An invoice that does not claim to be an XRechnung fails its rules.
    assert!(!stricter.conforms);
    assert!(
        stricter
            .rule_sets
            .iter()
            .any(|rules| !rules.findings.is_empty())
    );
    Ok(())
}

#[test]
fn nothing_can_be_checked_again_before_a_first_check() -> Result<(), Fault> {
    let service = ValidationService::start()?;

    assert_eq!(
        refusal(check_again(&service, Profile::En16931))?,
        Refusal::NothingToCheck
    );
    Ok(())
}

#[test]
fn what_is_no_invoice_is_refused_with_the_reason() -> Result<(), Fault> {
    let service = ValidationService::start()?;
    let directory = TempDir::new()?;

    let no_xml = written(&directory, "notes.xml", "not XML at all")?;
    assert!(matches!(
        refusal(check_file(&service, &no_xml))?,
        Refusal::Malformed { .. }
    ));

    let empty = written(&directory, "empty.xml", "")?;
    assert!(matches!(
        refusal(check_file(&service, &empty))?,
        Refusal::Malformed { .. }
    ));

    let order = written(&directory, "order.xml", "<Order/>")?;
    assert_eq!(
        refusal(check_file(&service, &order))?,
        Refusal::UnsupportedDocument {
            element: "Order".to_owned()
        }
    );

    assert_eq!(
        refusal(check_file(&service, &directory.path().join("missing.xml")))?,
        Refusal::Unreadable
    );
    Ok(())
}

/// The validation runs on a thread of its own, whose stack is smaller than
/// the one of the main thread. A document nested far deeper than any stack
/// allows must be refused there, not end the program.
#[test]
fn a_file_nested_too_deeply_is_refused_and_the_service_goes_on() -> Result<(), Fault> {
    /// Deep enough to exhaust the stack of a thread if a tree were built.
    const LEVELS: usize = 200_000;

    let service = ValidationService::start()?;
    let directory = TempDir::new()?;
    let deep = written(
        &directory,
        "deep.xml",
        &format!("{}{}", "<a>".repeat(LEVELS), "</a>".repeat(LEVELS)),
    )?;

    assert_eq!(
        refusal(check_file(&service, &deep))?,
        Refusal::TooDeeplyNested {
            limit: MAX_NESTING_DEPTH
        }
    );
    // The thread is still there and checks the next file.
    assert!(checked(check_file(&service, &corpus(XRECHNUNG_UBL)))?.conforms);
    Ok(())
}

/// The limit must be one the whole check can bear, not only the reading: the
/// XML Schema step and the rules walk the tree as well, on the smaller stack
/// of the validation thread.
#[test]
fn an_invoice_nested_exactly_to_the_limit_is_checked_in_full() -> Result<(), Fault> {
    let service = ValidationService::start()?;
    let directory = TempDir::new()?;
    // The document element is the first level; the rest lies inside it.
    let inside = MAX_NESTING_DEPTH - 1;
    let nested = format!("{}{}", "<a>".repeat(inside), "</a>".repeat(inside));

    for (name, open, close) in [
        (
            "ubl.xml",
            r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2">"#,
            "</Invoice>",
        ),
        (
            "cii.xml",
            r#"<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100">"#,
            "</rsm:CrossIndustryInvoice>",
        ),
    ] {
        let file = written(&directory, name, &format!("{open}{nested}{close}"))?;

        let check = checked(check_file(&service, &file))?;

        // It is no valid invoice, but it was judged by the schema and by the
        // rules, and the thread lived through it.
        assert!(!check.conforms, "{name}");
        assert!(!check.schema_findings.is_empty(), "{name}");
        assert!(!check.rule_sets.is_empty(), "{name}");
    }
    Ok(())
}

#[test]
fn a_refused_file_does_not_replace_the_invoice_checked_before() -> Result<(), Fault> {
    let service = ValidationService::start()?;
    let directory = TempDir::new()?;
    let invoice = checked(check_file(&service, &corpus(XRECHNUNG_UBL)))?;

    let order = written(&directory, "order.xml", "<Order/>")?;
    refusal(check_file(&service, &order))?;

    let again = checked(check_again(&service, Profile::En16931))?;
    assert_eq!(again.file_name, invoice.file_name);
    Ok(())
}

#[test]
fn an_invoice_that_breaks_rules_does_not_conform_and_names_them() -> Result<(), Fault> {
    let service = ValidationService::start()?;
    let directory = TempDir::new()?;
    let empty_invoice = written(
        &directory,
        "empty-invoice.xml",
        r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#,
    )?;

    let check = checked(check_file(&service, &empty_invoice))?;

    assert!(!check.conforms);
    assert_eq!(check.specification_identifier, None);
    assert!(!check.schema_findings.is_empty());
    let findings: Vec<_> = check
        .rule_sets
        .iter()
        .flat_map(|rules| &rules.findings)
        .collect();
    assert!(!findings.is_empty());
    assert!(findings.iter().all(|finding| !finding.rule_id.is_empty()));
    Ok(())
}

#[test]
fn clones_of_the_service_reach_the_same_validator_from_other_threads() -> Result<(), Fault> {
    let service = ValidationService::start()?;
    checked(check_file(&service, &corpus(XRECHNUNG_UBL)))?;

    let clone = service.clone();
    let outcome = std::thread::spawn(move || check_again(&clone, Profile::En16931))
        .join()
        .map_err(|_| "the thread failed")?;

    assert_eq!(checked(outcome)?.profile, ProfileKind::En16931);
    Ok(())
}

#[test]
fn the_outcome_is_serialized_with_the_names_the_frontend_bindings_use() -> Result<(), Fault> {
    let cancelled = serde_json::to_value(CheckOutcome::Cancelled)?;
    assert_eq!(cancelled["kind"], "cancelled");

    let refused = serde_json::to_value(CheckOutcome::Refused {
        refusal: Refusal::TooLarge { limit_bytes: 1 },
    })?;
    assert_eq!(refused["kind"], "refused");
    assert_eq!(refused["refusal"]["reason"], "tooLarge");
    assert_eq!(refused["refusal"]["limitBytes"], 1);

    assert_eq!(serde_json::to_value(ProfileKind::En16931)?, "en16931");
    assert_eq!(serde_json::to_value(ProfileKind::XRechnung)?, "xrechnung");
    assert_eq!(
        serde_json::from_value::<ProfileKind>("xrechnung".into())?,
        ProfileKind::XRechnung
    );

    let service = ValidationService::start()?;
    let checked = serde_json::to_value(check_file(&service, &corpus(XRECHNUNG_UBL)))?;
    assert_eq!(checked["kind"], "checked");
    for field in [
        "fileName",
        "syntax",
        "profile",
        "suggestedProfile",
        "specificationIdentifier",
        "conforms",
        "schemaFindings",
        "ruleSets",
    ] {
        assert!(checked["check"].get(field).is_some(), "{field}");
    }
    Ok(())
}
