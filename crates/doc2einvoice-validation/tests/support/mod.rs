// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The conformance corpus: the published test documents as individual cases.
//!
//! Shared by the conformance test and by the tool that prepares the reference
//! run, so that both see exactly the same cases under the same names.

// Each user of this module needs a different part of it.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use doc2einvoice_validation::Syntax;
use doc2einvoice_validation::schematron::{Outcome, Report};
use serde::{Deserialize, Serialize};

/// A fault while reading the corpus or its expectations.
pub type Fault = Box<dyn std::error::Error>;

const CORPUS_DIRECTORY: &str = "tests/conformance/corpus";
const EXPECTED_DIRECTORY: &str = "tests/conformance/expected";
const SOURCES_FILE: &str = "sources.env";

const XML_EXTENSION: &str = "xml";
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

/// Elements of the test set format the rule publishers use: a file holds
/// several small documents, each with the outcome expected for named rules.
const TEST_SET_ELEMENT: &str = "testSet";
const TEST_ELEMENT: &str = "test";
const ASSERT_ELEMENT: &str = "assert";
const SUCCESS_ELEMENT: &str = "success";
const ERROR_ELEMENT: &str = "error";
const WARNING_ELEMENT: &str = "warning";

/// Separates the file of a test set from the number of a test inside it.
const CASE_NUMBER_SEPARATOR: char = '#';

/// A finding as the conformance results record it: the rule and the node it
/// was reported on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Located(pub String, pub String);

/// A rule that could not be evaluated: the rule, the node and the error.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Unevaluable(pub String, pub String, pub String);

/// What a rule set is expected to report for one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expected {
    /// Every finding the reference implementation reports, with its location.
    /// A rule reported on three nodes appears three times.
    Reference(Vec<Located>),
    /// The reference implementation stopped at a rule it could not evaluate
    /// and produced no report. Recorded instead are the rules the interpreter
    /// cannot evaluate on this document. They do not come from the reference:
    /// they are checked by hand against the error it stopped with.
    ReferenceStopped(Vec<Unevaluable>),
}

/// The expected results of a rule set, by the name of the case.
pub type ExpectedResults = BTreeMap<String, Expected>;

/// Reads the expected results of a rule set.
pub fn expected_results(rule_set_key: &str) -> Result<ExpectedResults, Fault> {
    Ok(serde_json::from_str(&fs::read_to_string(expected_file(
        rule_set_key,
    ))?)?)
}

/// Writes the expected results of a rule set, one case per line.
pub fn write_expected_results(rule_set_key: &str, results: &ExpectedResults) -> Result<(), Fault> {
    let mut lines = Vec::with_capacity(results.len());
    for (name, expected) in results {
        lines.push(format!(
            "{}: {}",
            serde_json::to_string(name)?,
            serde_json::to_string(expected)?
        ));
    }
    let target = expected_file(rule_set_key);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, format!("{{\n{}\n}}\n", lines.join(",\n")))?;
    Ok(())
}

/// Every finding of a report with its location, sorted.
pub fn located_findings(report: &Report) -> Vec<Located> {
    let mut findings: Vec<Located> = report
        .findings()
        .iter()
        .map(|finding| Located(finding.assertion_id.clone(), finding.location.clone()))
        .collect();
    findings.sort();
    findings
}

/// The findings of a report that are rules which could not be evaluated,
/// sorted.
pub fn unevaluable_findings(report: &Report) -> Vec<Unevaluable> {
    let mut findings: Vec<Unevaluable> = report
        .findings()
        .iter()
        .filter_map(|finding| match &finding.outcome {
            Outcome::EvaluationError { error } => Some(Unevaluable(
                finding.assertion_id.clone(),
                finding.location.clone(),
                error.clone(),
            )),
            Outcome::Violated => None,
        })
        .collect();
    findings.sort();
    findings
}

/// One document of the corpus.
pub struct Case {
    /// Unique name: the path of the file inside the corpus, followed by the
    /// number of the test for a document taken from a test set.
    pub name: String,
    /// The document.
    pub xml: String,
    /// Rules the publisher expects not to be reported for this document.
    pub must_pass: Vec<String>,
    /// Rules the publisher expects to be reported for this document.
    pub must_fire: Vec<String>,
}

/// The root of the repository.
pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The file holding the reference results of a rule set.
pub fn expected_file(rule_set_key: &str) -> PathBuf {
    repository_root()
        .join(EXPECTED_DIRECTORY)
        .join(format!("{rule_set_key}.json"))
}

/// The pinned sources as pairs of key and value.
pub fn sources() -> Result<BTreeMap<String, String>, Fault> {
    let text = fs::read_to_string(repository_root().join(SOURCES_FILE))?;
    Ok(text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect())
}

/// Every case of the corpus written in the given syntax, ordered by name.
pub fn cases(syntax: Syntax) -> Result<Vec<Case>, Fault> {
    let corpus = repository_root().join(CORPUS_DIRECTORY);
    let mut files = Vec::new();
    collect_files(&corpus, &mut files)?;
    files.sort();

    let mut cases = Vec::new();
    for file in files {
        let name = file
            .strip_prefix(&corpus)?
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let text = fs::read_to_string(&file)?;
        for case in cases_of_file(&name, &text)? {
            if Syntax::of(&case.xml).is_ok_and(|found| found == syntax) {
                cases.push(case);
            }
        }
    }
    Ok(cases)
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), Fault> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(XML_EXTENSION))
        {
            files.push(path);
        }
    }
    Ok(())
}

/// A file is either one document or a test set holding several.
fn cases_of_file(name: &str, text: &str) -> Result<Vec<Case>, Fault> {
    let parsed = roxmltree::Document::parse(text)?;
    let root = parsed.root_element();
    if root.tag_name().name() != TEST_SET_ELEMENT {
        return Ok(vec![Case {
            name: name.to_owned(),
            xml: text.to_owned(),
            must_pass: Vec::new(),
            must_fire: Vec::new(),
        }]);
    }

    let mut cases = Vec::new();
    let tests = root
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == TEST_ELEMENT);
    for (index, test) in tests.enumerate() {
        let expectation = test
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == ASSERT_ELEMENT);
        let Some(document) = test
            .children()
            .find(|node| node.is_element() && node.tag_name().name() != ASSERT_ELEMENT)
        else {
            continue;
        };
        let rules_named = |element: &str| -> Vec<String> {
            expectation
                .iter()
                .flat_map(roxmltree::Node::children)
                .filter(|node| node.is_element() && node.tag_name().name() == element)
                .filter_map(|node| node.text())
                .map(|rule| rule.trim().to_owned())
                .collect()
        };
        let mut must_fire = rules_named(ERROR_ELEMENT);
        must_fire.extend(rules_named(WARNING_ELEMENT));

        cases.push(Case {
            name: format!("{name}{CASE_NUMBER_SEPARATOR}{:02}", index + 1),
            // The document is taken from the file as written, so that it
            // reaches the rules exactly as its publisher wrote it.
            xml: format!("{XML_DECLARATION}{}", &text[document.range()]),
            must_pass: rules_named(SUCCESS_ELEMENT),
            must_fire,
        });
    }
    Ok(cases)
}

// ---------------------------------------------------------------------------
// XML Schema conformance: the corpus plus near-valid variants of its documents.
// ---------------------------------------------------------------------------

/// The verdicts of the reference XML Schema processor, by the name of the
/// document: whether the document is valid.
pub type SchemaVerdicts = BTreeMap<String, bool>;

/// Separates the name of a case from the description of a variant of it.
const VARIANT_SEPARATOR: char = '@';
/// Every how many leaf elements of a document one is changed.
const VARIANT_LEAF_STEP: usize = 5;
/// Every how many leaf elements a swap with the next leaf is tried.
const VARIANT_SWAP_STEP: usize = 11;
/// How far apart the two kinds of change applied to one leaf are in the list.
const VARIANT_SECOND_KIND_OFFSET: usize = 3;

/// The ways one leaf element of a valid document is changed. Each tests a
/// different part of a schema: types, required content, order, unknown names.
const VARIANT_KINDS: [&str; 10] = [
    "text",
    "empty",
    "delete",
    "duplicate",
    "rename",
    "attribute",
    "no-attributes",
    "number",
    "date",
    "spaces",
];

/// A document for the XML Schema comparison.
pub struct SchemaCase {
    /// The name of the corpus case, followed for a variant by what was changed.
    pub name: String,
    /// The document.
    pub xml: String,
}

/// The file holding the reference verdicts of the XML Schema of a syntax.
pub fn schema_verdicts_file(schema_key: &str) -> PathBuf {
    repository_root()
        .join(EXPECTED_DIRECTORY)
        .join(format!("xml-schema-{schema_key}.json"))
}

/// Reads the reference verdicts of the XML Schema of a syntax.
pub fn schema_verdicts(schema_key: &str) -> Result<SchemaVerdicts, Fault> {
    Ok(serde_json::from_str(&fs::read_to_string(
        schema_verdicts_file(schema_key),
    )?)?)
}

/// Writes reference verdicts, one document per line.
pub fn write_schema_verdicts(schema_key: &str, verdicts: &SchemaVerdicts) -> Result<(), Fault> {
    let mut lines = Vec::with_capacity(verdicts.len());
    for (name, valid) in verdicts {
        lines.push(format!("{}: {valid}", serde_json::to_string(name)?));
    }
    fs::write(
        schema_verdicts_file(schema_key),
        format!("{{\n{}\n}}\n", lines.join(",\n")),
    )?;
    Ok(())
}

/// The documents of the corpus written in the given syntax, as schema cases.
pub fn schema_cases(syntax: Syntax) -> Result<Vec<SchemaCase>, Fault> {
    Ok(cases(syntax)?
        .into_iter()
        .map(|case| SchemaCase {
            name: case.name,
            xml: case.xml,
        })
        .collect())
}

/// A leaf element as it is written in a document.
struct Leaf<'a> {
    /// Where the element starts and ends in the document.
    range: std::ops::Range<usize>,
    /// The name with its prefix.
    name: &'a str,
    /// The attributes as written, with their leading whitespace.
    attributes: &'a str,
    /// The text between the tags.
    text: &'a str,
}

/// Variants of a valid document that each differ from it in one small way.
/// Most are invalid, some stay valid; the reference decides which.
pub fn schema_variants(case: &SchemaCase) -> Vec<SchemaCase> {
    let Ok(parsed) = roxmltree::Document::parse(&case.xml) else {
        return Vec::new();
    };
    let leaves: Vec<Leaf> = parsed
        .descendants()
        .filter(|node| node.is_element() && node.children().all(|child| child.is_text()))
        .filter_map(|node| leaf_at(&case.xml, node.range()))
        .collect();

    let mut variants = Vec::new();
    let variant_of = |leaf: &Leaf, replacement: &str| {
        format!(
            "{}{replacement}{}",
            &case.xml[..leaf.range.start],
            &case.xml[leaf.range.end..]
        )
    };
    for (index, leaf) in leaves.iter().enumerate().step_by(VARIANT_LEAF_STEP) {
        let turn = index / VARIANT_LEAF_STEP;
        for kind in [
            VARIANT_KINDS[turn % VARIANT_KINDS.len()],
            VARIANT_KINDS[(turn + VARIANT_SECOND_KIND_OFFSET) % VARIANT_KINDS.len()],
        ] {
            variants.push(SchemaCase {
                name: format!("{}{VARIANT_SEPARATOR}{index:04}:{kind}", case.name),
                xml: variant_of(leaf, &changed(leaf, kind, &case.xml)),
            });
        }
    }
    for index in (0..leaves.len().saturating_sub(1)).step_by(VARIANT_SWAP_STEP) {
        let (first, second) = (&leaves[index], &leaves[index + 1]);
        let between = case.xml.get(first.range.end..second.range.start);
        if first.name == second.name || !between.is_some_and(|gap| gap.trim().is_empty()) {
            continue;
        }
        variants.push(SchemaCase {
            name: format!("{}{VARIANT_SEPARATOR}{index:04}:swap", case.name),
            xml: format!(
                "{}{}{}{}{}",
                &case.xml[..first.range.start],
                &case.xml[second.range.clone()],
                between.unwrap_or_default(),
                &case.xml[first.range.clone()],
                &case.xml[second.range.end..]
            ),
        });
    }
    variants
}

/// Reads a leaf element from the text of the document. `None` for an element
/// written as an empty tag, which has no text to change.
fn leaf_at(document: &str, range: std::ops::Range<usize>) -> Option<Leaf<'_>> {
    let written = &document[range.clone()];
    let start_tag_end = start_tag_end(written)?;
    let start_tag = &written[1..start_tag_end];
    if start_tag.ends_with('/') {
        return None;
    }
    let name_end = start_tag
        .find(char::is_whitespace)
        .unwrap_or(start_tag.len());
    let end_tag_start = written.rfind("</")?;
    Some(Leaf {
        range,
        name: &start_tag[..name_end],
        attributes: &start_tag[name_end..],
        text: written.get(start_tag_end + 1..end_tag_start)?,
    })
}

/// The position of the `>` that closes a start tag, skipping any `>` inside an
/// attribute value.
fn start_tag_end(element: &str) -> Option<usize> {
    let mut open_quote: Option<char> = None;
    for (position, character) in element.char_indices() {
        match open_quote {
            Some(quote) => {
                if character == quote {
                    open_quote = None;
                }
            }
            None => match character {
                '"' | '\'' => open_quote = Some(character),
                '>' => return Some(position),
                _ => {}
            },
        }
    }
    None
}

/// The text a leaf is replaced by for one kind of change.
fn changed(leaf: &Leaf, kind: &str, document: &str) -> String {
    let Leaf {
        name,
        attributes,
        text,
        ..
    } = leaf;
    let written = &document[leaf.range.clone()];
    let prefix = name
        .split_once(':')
        .map_or(String::new(), |(prefix, _)| format!("{prefix}:"));
    match kind {
        "text" => format!("<{name}{attributes}>x</{name}>"),
        "empty" => format!("<{name}{attributes}></{name}>"),
        "delete" => String::new(),
        "duplicate" => format!("{written}{written}"),
        "rename" => format!("<{prefix}Bogus{attributes}>{text}</{prefix}Bogus>"),
        "attribute" => format!("<{name}{attributes} bogus=\"1\">{text}</{name}>"),
        "no-attributes" => format!("<{name}>{text}</{name}>"),
        "number" => format!("<{name}{attributes}>12.3.4</{name}>"),
        "date" => format!("<{name}{attributes}>2026-13-45</{name}>"),
        _ => format!("<{name}{attributes}> {text} </{name}>"),
    }
}
