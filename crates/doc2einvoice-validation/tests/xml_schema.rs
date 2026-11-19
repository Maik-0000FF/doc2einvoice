// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Conformance of the XML Schema validation: for every document the verdict
//! must be the verdict of the reference XML Schema processor.
//!
//! The documents are the conformance corpus and, for every document of it that
//! is valid, variants that differ from it in one small way: an element removed,
//! renamed, duplicated or moved, a value replaced by one of the wrong type.
//! The corpus alone would not do: its invalid documents are small fragments
//! that any schema refuses.
//!
//! The expected verdicts under `tests/conformance/expected` come from the XML
//! Schema processor of the JDK, the processor the official validators use.
//! They are produced by `tools/regenerate-conformance.sh`.
//!
//! Compared is the verdict, valid or invalid. The wording and the location of
//! what the processor reports are not compared.

mod support;

use doc2einvoice_validation::{Syntax, XmlSchemaInfo};
use support::{Fault, SchemaCase};

/// How many differing documents are listed before the rest is only counted.
const LISTED_DIFFERENCES: usize = 20;

/// Every document of the comparison: the cases of the corpus and the variants
/// of those the reference found valid.
fn documents(syntax: Syntax, verdicts: &support::SchemaVerdicts) -> Result<Vec<SchemaCase>, Fault> {
    let cases = support::schema_cases(syntax)?;
    let mut variants = Vec::new();
    for case in &cases {
        if verdicts.get(&case.name).copied().unwrap_or(false) {
            variants.extend(support::schema_variants(case));
        }
    }
    Ok(cases.into_iter().chain(variants).collect())
}

fn agrees_with_the_reference(syntax: Syntax) -> Result<(), Fault> {
    let info = XmlSchemaInfo::of(syntax);
    let verdicts = support::schema_verdicts(info.key)?;
    let documents = documents(syntax, &verdicts)?;
    let schema = info.compile()?;

    assert_eq!(
        documents.len(),
        verdicts.len(),
        "the documents and the expected verdicts of {} differ in number",
        info.key
    );

    let mut differences = Vec::new();
    for document in &documents {
        let Some(&expected) = verdicts.get(&document.name) else {
            differences.push(format!("{}: no expected verdict", document.name));
            continue;
        };
        let verdict = schema.validate(&document.xml)?.is_empty();
        if verdict != expected {
            differences.push(format!(
                "{}: valid = {verdict}, the reference says {expected}",
                document.name
            ));
        }
    }

    assert!(
        differences.is_empty(),
        "{}: {} differences in {} documents:\n{}",
        info.key,
        differences.len(),
        documents.len(),
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
fn ubl_agrees_with_the_reference() -> Result<(), Fault> {
    agrees_with_the_reference(Syntax::Ubl)
}

#[test]
fn cii_agrees_with_the_reference() -> Result<(), Fault> {
    agrees_with_the_reference(Syntax::Cii)
}

/// The corpus must contain both kinds of documents for the comparison to mean
/// anything.
#[test]
fn the_expected_verdicts_contain_valid_and_invalid_documents() -> Result<(), Fault> {
    for syntax in [Syntax::Ubl, Syntax::Cii] {
        let verdicts = support::schema_verdicts(XmlSchemaInfo::of(syntax).key)?;

        assert!(verdicts.values().any(|valid| *valid));
        assert!(verdicts.values().any(|valid| !*valid));
    }
    Ok(())
}
