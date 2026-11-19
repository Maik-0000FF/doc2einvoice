// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tool for the reference run that produces the expected conformance results.
//!
//! The reference results come from running the published rules with an XSLT
//! processor, outside this program. This tool hands the corpus to that run and
//! takes its reports back, so that both sides see the same cases:
//!
//! ```text
//! conformance_reference export <directory>
//!     writes every case as <directory>/<syntax>/<number>.xml
//! conformance_reference collect <rule set key> <report directory>
//!     reads the SVRL reports <report directory>/<number>.xml and writes the
//!     expected results of the rule set
//! conformance_reference schema-variants <schema key> <verdicts> <directory>
//!     reads the verdicts of the reference XML Schema processor on the exported
//!     cases and writes the variants of the valid ones as
//!     <directory>/<number>.xml
//! conformance_reference schema-collect <schema key> <verdicts> <variant verdicts>
//!     writes the expected verdicts of the XML Schema, for the cases and their
//!     variants
//! ```
//!
//! It is run by `tools/regenerate-conformance.sh`.

#[path = "../tests/support/mod.rs"]
mod support;

use std::fs;
use std::path::Path;

use doc2einvoice_validation::{RuleSet, Syntax, XmlSchemaInfo};
use support::{Expected, ExpectedResults, Fault, Located, SchemaCase, SchemaVerdicts};

const COMMAND_EXPORT: &str = "export";
const COMMAND_COLLECT: &str = "collect";
const COMMAND_SCHEMA_VARIANTS: &str = "schema-variants";
const COMMAND_SCHEMA_COLLECT: &str = "schema-collect";
const USAGE: &str = "usage: conformance_reference \
    export <directory> \
    | collect <rule set key> <report directory> \
    | schema-variants <schema key> <verdicts> <directory> \
    | schema-collect <schema key> <verdicts> <variant verdicts>";

const SVRL_NAMESPACE: &str = "http://purl.oclc.org/dsdl/svrl";
const SVRL_FAILED_ASSERT: &str = "failed-assert";
const SVRL_ID_ATTRIBUTE: &str = "id";
const SVRL_LOCATION_ATTRIBUTE: &str = "location";

/// The two ways the reference stylesheets qualify a name in a location:
/// `*:Invoice[namespace-uri()='urn:...']` and `Q{urn:...}Invoice`.
const WILDCARD_PREFIX: &str = "*:";
const NAMESPACE_PREDICATE_OPEN: &str = "[namespace-uri()='";
const NAMESPACE_PREDICATE_CLOSE: &str = "']";
const BRACED_NAMESPACE_OPEN: &str = "Q{";
const BRACED_NAMESPACE_CLOSE: char = '}';

fn main() -> Result<(), Fault> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        [COMMAND_EXPORT, directory] => export(Path::new(directory)),
        [COMMAND_COLLECT, key, reports] => collect(key, Path::new(reports)),
        [COMMAND_SCHEMA_VARIANTS, key, verdicts, directory] => {
            schema_variants(key, Path::new(verdicts), Path::new(directory))
        }
        [COMMAND_SCHEMA_COLLECT, key, verdicts, variant_verdicts] => {
            schema_collect(key, Path::new(verdicts), Path::new(variant_verdicts))
        }
        _ => Err(USAGE.into()),
    }
}

fn directory_name(syntax: Syntax) -> &'static str {
    match syntax {
        Syntax::Ubl => "ubl",
        Syntax::Cii => "cii",
    }
}

/// The file a case is exchanged as. Case names contain characters that are not
/// safe in file names, so the position in the ordered corpus is used.
fn file_name(index: usize) -> String {
    format!("{index:05}.xml")
}

fn export(directory: &Path) -> Result<(), Fault> {
    for syntax in [Syntax::Ubl, Syntax::Cii] {
        let target = directory.join(directory_name(syntax));
        fs::create_dir_all(&target)?;
        let cases = support::cases(syntax)?;
        for (index, case) in cases.iter().enumerate() {
            fs::write(target.join(file_name(index)), &case.xml)?;
        }
        println!("{}: {} cases", directory_name(syntax), cases.len());
    }
    Ok(())
}

fn collect(key: &str, reports: &Path) -> Result<(), Fault> {
    let rule_set = RuleSet::ALL
        .into_iter()
        .find(|rule_set| rule_set.info().key == key)
        .ok_or_else(|| format!("unknown rule set `{key}`"))?;
    let schema = rule_set.compile()?;

    let mut results = ExpectedResults::new();
    for (index, case) in support::cases(rule_set.info().syntax)?
        .into_iter()
        .enumerate()
    {
        let expected = if let Some(findings) = reference_findings(&reports.join(file_name(index))) {
            Expected::Reference(findings)
        } else {
            // The reference produced no report. What the interpreter cannot
            // evaluate is recorded and printed, to be checked by hand against
            // the error the reference stopped with.
            let unevaluable = support::unevaluable_findings(&schema.validate(&case.xml)?);
            println!(
                "{key}: no reference result for {}; recorded instead: {unevaluable:?}",
                case.name
            );
            Expected::ReferenceStopped(unevaluable)
        };
        results.insert(case.name, expected);
    }

    support::write_expected_results(key, &results)?;
    println!("{key}: {} cases", results.len());
    Ok(())
}

/// Every finding a report names, with its location, sorted.
///
/// `None` if the reference processor produced no complete report: it stops at
/// the first rule it cannot evaluate.
fn reference_findings(report: &Path) -> Option<Vec<Located>> {
    let text = fs::read_to_string(report).ok()?;
    let parsed = roxmltree::Document::parse(&text).ok()?;
    let mut findings: Vec<Located> = parsed
        .descendants()
        .filter(|node| {
            node.tag_name().name() == SVRL_FAILED_ASSERT
                && node.tag_name().namespace() == Some(SVRL_NAMESPACE)
        })
        .filter_map(|node| {
            Some(Located(
                node.attribute(SVRL_ID_ATTRIBUTE)?.to_owned(),
                plain_location(node.attribute(SVRL_LOCATION_ATTRIBUTE)?),
            ))
        })
        .collect();
    findings.sort();
    Some(findings)
}

/// Reduces a location of the reference to local names and positions, the form
/// the interpreter reports: `/Invoice[1]/InvoiceLine[2]`.
fn plain_location(location: &str) -> String {
    let without_predicates = remove_all_between(
        location,
        NAMESPACE_PREDICATE_OPEN,
        NAMESPACE_PREDICATE_CLOSE,
    );
    let without_braces = remove_all_between(
        &without_predicates,
        BRACED_NAMESPACE_OPEN,
        &BRACED_NAMESPACE_CLOSE.to_string(),
    );
    without_braces.replace(WILDCARD_PREFIX, "")
}

/// Removes every stretch of text from `open` up to and including the next
/// `close`.
fn remove_all_between(text: &str, open: &str, close: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(close) else {
            // Not closed: keep the text as it is.
            result.push_str(&rest[start..]);
            return result;
        };
        rest = &after_open[end + close.len()..];
    }
    result.push_str(rest);
    result
}

/// The syntax whose XML Schema has the given key.
fn syntax_of_schema(key: &str) -> Result<Syntax, Fault> {
    [Syntax::Ubl, Syntax::Cii]
        .into_iter()
        .find(|syntax| XmlSchemaInfo::of(*syntax).key == key)
        .ok_or_else(|| format!("unknown XML Schema `{key}`").into())
}

/// Reads the verdicts the reference wrote for exported files and returns them
/// in the order of the files.
fn verdicts_by_position(file: &Path, count: usize) -> Result<Vec<bool>, Fault> {
    let by_file: SchemaVerdicts = serde_json::from_str(&fs::read_to_string(file)?)?;
    (0..count)
        .map(|index| {
            by_file
                .get(&file_name(index))
                .copied()
                .ok_or_else(|| format!("no verdict for {}", file_name(index)).into())
        })
        .collect()
}

/// The variants of every case the reference found valid, in a fixed order.
fn variants_of_valid_cases(syntax: Syntax, verdicts: &Path) -> Result<Vec<SchemaCase>, Fault> {
    let cases = support::schema_cases(syntax)?;
    let valid = verdicts_by_position(verdicts, cases.len())?;
    Ok(cases
        .iter()
        .zip(valid)
        .filter(|(_, valid)| *valid)
        .flat_map(|(case, _)| support::schema_variants(case))
        .collect())
}

fn schema_variants(key: &str, verdicts: &Path, directory: &Path) -> Result<(), Fault> {
    let variants = variants_of_valid_cases(syntax_of_schema(key)?, verdicts)?;
    fs::create_dir_all(directory)?;
    for (index, variant) in variants.iter().enumerate() {
        fs::write(directory.join(file_name(index)), &variant.xml)?;
    }
    println!("{key}: {} variants", variants.len());
    Ok(())
}

fn schema_collect(key: &str, verdicts: &Path, variant_verdicts: &Path) -> Result<(), Fault> {
    let syntax = syntax_of_schema(key)?;
    let cases = support::schema_cases(syntax)?;
    let variants = variants_of_valid_cases(syntax, verdicts)?;

    let mut expected = SchemaVerdicts::new();
    for (case, valid) in cases
        .iter()
        .zip(verdicts_by_position(verdicts, cases.len())?)
    {
        expected.insert(case.name.clone(), valid);
    }
    for (variant, valid) in variants
        .iter()
        .zip(verdicts_by_position(variant_verdicts, variants.len())?)
    {
        expected.insert(variant.name.clone(), valid);
    }
    support::write_schema_verdicts(key, &expected)?;
    println!(
        "{key}: {} documents, {} valid",
        expected.len(),
        expected.values().filter(|valid| **valid).count()
    );
    Ok(())
}
