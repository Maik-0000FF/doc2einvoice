// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Checking many invoice files without a window, one line of text per file.
//!
//! This is the output of the command line. Its wording is English and fixed,
//! so that scripts can rely on it; the window says the same in its own
//! language files.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use doc2einvoice_validation::Profile;

use super::checker::Checker;
use super::document::{self, INVOICE_EXTENSION};
use super::outcome::{InvoiceCheck, Refusal, RuleCoverageKind, SeverityKind};

/// What follows the line of a file whose invoice names a specification the
/// rules of which were not applied: its verdict covers less than the invoice
/// claims.
const RULES_NOT_APPLIED: &str = "; the rules of the specification it names were not applied";

/// How the lines below a file are indented when its findings are listed.
const FINDING_INDENT: &str = "    ";

/// Sizes of files are stated in mebibytes.
const BYTES_PER_MEBIBYTE: u64 = 1024 * 1024;

/// What to check and how much to say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRequest {
    /// The files and directories to check, in the order given.
    pub paths: Vec<PathBuf>,
    /// The profile every file is checked as. Without it, each file is checked
    /// as the profile its specification identifier suggests.
    pub profile: Option<Profile>,
    /// Whether the findings of each file are listed below its line.
    pub list_findings: bool,
}

/// How many files ended in which way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Files that are invoices and conform.
    pub conforming: usize,
    /// Files that are invoices and do not conform.
    pub failing: usize,
    /// Paths that were not checked: no invoice, not readable, nothing found.
    pub not_checked: usize,
}

impl Tally {
    /// Whether every path led to invoices and every one of them conforms.
    /// A run that checked nothing is no success.
    #[must_use]
    pub fn all_conform(&self) -> bool {
        self.conforming > 0 && self.failing == 0 && self.not_checked == 0
    }
}

/// One thing to report: a file to check, or a path that leads to none.
enum Entry {
    File(PathBuf),
    NotChecked(PathBuf, &'static str),
}

/// Checks every path of the request and writes one line per file, then a line
/// with the totals.
///
/// | Path                                   | What is written                    |
/// |----------------------------------------|------------------------------------|
/// | a file                                 | its verdict                        |
/// | a file that is no invoice              | not checked, with the reason       |
/// | nothing, or a file that is unreadable  | not checked, with the reason       |
/// | a directory                            | the verdict of every file in it or |
/// |                                        | below it whose extension is `xml`, |
/// |                                        | in any case of letters, sorted     |
/// | a directory without such a file        | not checked: nothing found         |
/// | a directory that cannot be listed      | not checked, the others go on      |
/// | a directory with an entry the system   | not checked, once per such entry;  |
/// | fails to list                          | the entries it does list go on     |
/// | a symbolic link to a file              | the file it leads to               |
/// | a symbolic link to a directory, inside | not followed: links can form loops |
/// | a directory                            |                                    |
/// | the same path twice                    | checked twice, as asked            |
///
/// # Errors
///
/// Returns the error of the operating system if a line cannot be written.
pub fn run(request: &BatchRequest, output: &mut impl Write) -> io::Result<Tally> {
    let mut checker = Checker::new();
    let mut tally = Tally::default();

    for path in &request.paths {
        for entry in entries(path) {
            match entry {
                Entry::File(file) => {
                    let result = document::load(&file)
                        .and_then(|document| checker.check(&document, request.profile));
                    match result {
                        Ok(check) => {
                            if check.conforms {
                                tally.conforming += 1;
                            } else {
                                tally.failing += 1;
                            }
                            write_check(output, &file, &check, request.list_findings)?;
                        }
                        Err(refusal) => {
                            tally.not_checked += 1;
                            write_not_checked(output, &file, &reason(&refusal))?;
                        }
                    }
                }
                Entry::NotChecked(path, why) => {
                    tally.not_checked += 1;
                    write_not_checked(output, &path, why)?;
                }
            }
        }
    }
    writeln!(
        output,
        "conform: {}, do not conform: {}, not checked: {}",
        tally.conforming, tally.failing, tally.not_checked
    )?;
    Ok(tally)
}

/// What a path leads to: itself, or the invoice files of a directory.
fn entries(path: &Path) -> Vec<Entry> {
    if !path.is_dir() {
        return vec![Entry::File(path.to_owned())];
    }
    let mut found = Vec::new();
    collect(path, &mut found);
    if found.is_empty() {
        found.push(Entry::NotChecked(
            path.to_owned(),
            "the directory contains no XML file",
        ));
    }
    found
}

/// Adds the invoice files of a directory and of the directories below it, in
/// the order of their names. What cannot be listed is reported, never left
/// out: a file that silently went missing would let a run succeed that did not
/// check everything.
fn collect(directory: &Path, found: &mut Vec<Entry>) {
    let Ok(listing) = fs::read_dir(directory) else {
        found.push(Entry::NotChecked(
            directory.to_owned(),
            "the directory cannot be read",
        ));
        return;
    };
    let mut paths = Vec::new();
    for entry in listing {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(_) => found.push(Entry::NotChecked(
                directory.to_owned(),
                "an entry of the directory cannot be read",
            )),
        }
    }
    paths.sort();

    for path in paths {
        // A link to a directory is not followed; a link to a file is a file.
        if path.is_dir() {
            if !path.is_symlink() {
                collect(&path, found);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(INVOICE_EXTENSION))
        {
            found.push(Entry::File(path));
        }
    }
}

fn write_check(
    output: &mut impl Write,
    path: &Path,
    check: &InvoiceCheck,
    list_findings: bool,
) -> io::Result<()> {
    let count = |severity: SeverityKind| {
        check
            .rule_sets
            .iter()
            .flat_map(|rules| &rules.findings)
            .filter(|finding| finding.severity == severity)
            .count()
    };
    writeln!(
        output,
        "{}: {} (profile: {}, schema violations: {}, errors: {}, warnings: {}, notes: {}){}",
        path.display(),
        if check.conforms {
            "conforms"
        } else {
            "does not conform"
        },
        check.profile.name(),
        check.schema_findings.len(),
        count(SeverityKind::Fatal),
        count(SeverityKind::Warning),
        count(SeverityKind::Information),
        if check.rule_coverage == RuleCoverageKind::NotApplied {
            RULES_NOT_APPLIED
        } else {
            ""
        },
    )?;
    if !list_findings {
        return Ok(());
    }
    // One finding per line, whatever line breaks its texts contain.
    for finding in &check.schema_findings {
        writeln!(
            output,
            "{FINDING_INDENT}schema\t{}\t{}\t{}",
            finding.code,
            finding.location,
            single_line(&finding.message)
        )?;
    }
    for finding in check.rule_sets.iter().flat_map(|rules| &rules.findings) {
        write!(
            output,
            "{FINDING_INDENT}{}\t{}\t{}\t{}",
            severity_name(finding.severity),
            finding.rule_id,
            finding.location,
            single_line(&finding.message)
        )?;
        // A rule that could not be evaluated is no ordinary violation: a last
        // column says so and names the error.
        if let Some(code) = &finding.evaluation_error {
            write!(output, "\tnot evaluated: {code}")?;
        }
        writeln!(output)?;
    }
    Ok(())
}

fn write_not_checked(output: &mut impl Write, path: &Path, why: &str) -> io::Result<()> {
    writeln!(output, "{}: not checked: {why}", path.display())
}

/// A text on one line: rule files and schema processors break theirs freely.
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn severity_name(severity: SeverityKind) -> &'static str {
    match severity {
        SeverityKind::Fatal => "error",
        SeverityKind::Warning => "warning",
        SeverityKind::Information => "note",
    }
}

/// Why a file was not checked, in the words of the command line.
fn reason(refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotAFile => "this is not a file".to_owned(),
        Refusal::Unreadable => "the file cannot be read".to_owned(),
        Refusal::TooLarge { limit_bytes } => format!(
            "the file is larger than {} MiB",
            limit_bytes / BYTES_PER_MEBIBYTE
        ),
        Refusal::NotUtf8 => "the file is not text in UTF-8".to_owned(),
        Refusal::Malformed { detail } => format!("the file is not well-formed XML: {detail}"),
        Refusal::UnsupportedDocument { element } => {
            format!("the document element `{element}` is no invoice in UBL or CII")
        }
        Refusal::NothingToCheck => "there is nothing to check".to_owned(),
        Refusal::Internal => "the check failed inside the program".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use doc2einvoice_validation::Profile;
    use tempfile::TempDir;

    use super::{BatchRequest, Tally, run, single_line, write_check};
    use crate::app_info::Publication;
    use crate::checking::outcome::{
        InvoiceCheck, ProfileKind, RuleCoverageKind, RuleNote, RuleSetCheck, SchemaNote,
        SeverityKind, SyntaxKind,
    };

    const EMPTY_INVOICE: &str =
        r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#;

    fn request(paths: &[&Path]) -> BatchRequest {
        BatchRequest {
            paths: paths.iter().map(|path| (*path).to_owned()).collect(),
            profile: None,
            list_findings: false,
        }
    }

    fn written(directory: &Path, name: &str, content: &str) -> io::Result<PathBuf> {
        let path = directory.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    fn lines(request: &BatchRequest) -> io::Result<(Vec<String>, Tally)> {
        let mut output = Vec::new();
        let tally = run(request, &mut output)?;
        let text = String::from_utf8_lossy(&output).into_owned();
        Ok((text.lines().map(str::to_owned).collect(), tally))
    }

    #[test]
    fn a_file_gets_one_line_and_the_totals_follow() -> io::Result<()> {
        let directory = TempDir::new()?;
        let invoice = written(directory.path(), "a.xml", EMPTY_INVOICE)?;

        let (lines, tally) = lines(&request(&[&invoice]))?;

        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with(&format!(
            "{}: does not conform (profile: en16931",
            invoice.display()
        )));
        assert_eq!(lines[1], "conform: 0, do not conform: 1, not checked: 0");
        assert_eq!(tally.failing, 1);
        assert!(!tally.all_conform());
        Ok(())
    }

    #[test]
    fn what_is_no_invoice_is_not_checked_and_says_why() -> io::Result<()> {
        let directory = TempDir::new()?;
        let order = written(directory.path(), "order.xml", "<Order/>")?;
        let missing = directory.path().join("missing.xml");

        let (lines, tally) = lines(&request(&[&order, &missing]))?;

        assert!(
            lines[0]
                .ends_with("not checked: the document element `Order` is no invoice in UBL or CII")
        );
        assert!(lines[1].ends_with("not checked: the file cannot be read"));
        assert_eq!(tally.not_checked, 2);
        Ok(())
    }

    #[test]
    fn a_directory_stands_for_its_xml_files_in_the_order_of_their_names() -> io::Result<()> {
        let directory = TempDir::new()?;
        written(directory.path(), "b/second.XML", EMPTY_INVOICE)?;
        written(directory.path(), "a/first.xml", EMPTY_INVOICE)?;
        written(directory.path(), "a/notes.txt", "not an invoice")?;
        written(directory.path(), "c.xml", EMPTY_INVOICE)?;

        let (lines, tally) = lines(&request(&[directory.path()]))?;

        // Every line but the last one, which states the totals, names a file.
        let files = lines.split_last().map_or(&[][..], |(_, files)| files);
        let names: Vec<&str> = files
            .iter()
            .filter_map(|line| line.split(": ").next())
            .filter_map(|path| Path::new(path).file_name()?.to_str())
            .collect();
        assert_eq!(names, ["first.xml", "second.XML", "c.xml"]);
        assert_eq!(tally.failing, 3);
        Ok(())
    }

    #[test]
    fn a_directory_without_an_xml_file_is_not_a_success() -> io::Result<()> {
        let directory = TempDir::new()?;
        written(directory.path(), "notes.txt", "nothing")?;

        let (lines, tally) = lines(&request(&[directory.path()]))?;

        assert!(lines[0].ends_with("not checked: the directory contains no XML file"));
        assert_eq!(tally.not_checked, 1);
        assert!(!tally.all_conform());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_directory_is_not_followed_and_a_link_to_a_file_is() -> io::Result<()> {
        let directory = TempDir::new()?;
        let inside = directory.path().join("inside");
        let invoice = written(&inside, "a.xml", EMPTY_INVOICE)?;
        // A link back to the directory above would never end if it were followed.
        std::os::unix::fs::symlink(directory.path(), inside.join("loop"))?;
        std::os::unix::fs::symlink(&invoice, inside.join("linked.xml"))?;

        let (_, tally) = lines(&request(&[&inside]))?;

        assert_eq!(tally.failing, 2);
        Ok(())
    }

    #[test]
    fn rules_that_were_not_applied_are_named_on_the_line() -> io::Result<()> {
        let directory = TempDir::new()?;
        let named = |identifier: &str| {
            format!(
                r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
                    xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
                    ><cbc:CustomizationID>{identifier}</cbc:CustomizationID></Invoice>"#
            )
        };
        let extended = written(
            directory.path(),
            "extended.xml",
            &named("urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended"),
        )?;
        let standard = written(
            directory.path(),
            "standard.xml",
            &named("urn:cen.eu:en16931:2017"),
        )?;

        let (lines, _) = lines(&request(&[&extended, &standard]))?;

        assert!(lines[0].ends_with("; the rules of the specification it names were not applied"));
        assert!(lines[1].ends_with(')'));

        // An XRechnung checked as the standard alone was not checked against
        // the rules it names either.
        let xrechnung = written(
            directory.path(),
            "xrechnung.xml",
            &named("urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0"),
        )?;
        let mut as_the_standard = request(&[&xrechnung]);
        as_the_standard.profile = Some(Profile::En16931);
        let (lines, _) = super::tests::lines(&as_the_standard)?;
        assert!(lines[0].ends_with("; the rules of the specification it names were not applied"));

        let (lines, _) = super::tests::lines(&request(&[&xrechnung]))?;
        assert!(lines[0].ends_with(')'));
        Ok(())
    }

    #[test]
    fn the_same_path_twice_is_checked_twice() -> io::Result<()> {
        let directory = TempDir::new()?;
        let invoice = written(directory.path(), "a.xml", EMPTY_INVOICE)?;

        let (_, tally) = lines(&request(&[&invoice, &invoice]))?;

        assert_eq!(tally.failing, 2);
        Ok(())
    }

    #[test]
    fn the_profile_of_the_request_overrides_the_suggestion() -> io::Result<()> {
        let directory = TempDir::new()?;
        let invoice = written(directory.path(), "a.xml", EMPTY_INVOICE)?;
        let mut request = request(&[&invoice]);
        request.profile = Some(Profile::XRechnung);

        let (lines, _) = lines(&request)?;

        assert!(lines[0].contains("(profile: xrechnung,"));
        Ok(())
    }

    #[test]
    fn findings_are_listed_below_their_file_one_per_line() -> io::Result<()> {
        let directory = TempDir::new()?;
        let invoice = written(directory.path(), "a.xml", EMPTY_INVOICE)?;
        let mut request = request(&[&invoice]);
        request.list_findings = true;

        let (lines, _) = lines(&request)?;

        let findings: Vec<&String> = lines
            .iter()
            .filter(|line| line.starts_with("    "))
            .collect();
        assert!(findings.iter().any(|line| line.starts_with("    schema\t")));
        assert!(
            findings
                .iter()
                .any(|line| line.starts_with("    error\tBR-"))
        );
        assert_eq!(lines.len(), findings.len() + 2);
        Ok(())
    }

    #[test]
    fn every_finding_is_one_line_and_an_unevaluated_rule_says_so() -> io::Result<()> {
        let check = InvoiceCheck {
            file_name: "a.xml".to_owned(),
            syntax: SyntaxKind::Ubl,
            profile: ProfileKind::En16931,
            specification_identifier: None,
            rule_coverage: RuleCoverageKind::NotNamed,
            conforms: false,
            schema_findings: vec![SchemaNote {
                code: "cvc-1".to_owned(),
                message: "first\nsecond".to_owned(),
                location: "/Invoice[1]".to_owned(),
            }],
            rule_sets: vec![RuleSetCheck {
                publication: Publication {
                    title: "Rules".to_owned(),
                    version: None,
                    published: "2026-01-01".to_owned(),
                    license: None,
                    source: "https://example.org".to_owned(),
                },
                findings: vec![
                    RuleNote {
                        rule_id: "BR-01".to_owned(),
                        severity: SeverityKind::Fatal,
                        message: "broken\n  rule".to_owned(),
                        location: "/Invoice[1]".to_owned(),
                        evaluation_error: None,
                    },
                    RuleNote {
                        rule_id: "BR-02".to_owned(),
                        severity: SeverityKind::Fatal,
                        message: "rule".to_owned(),
                        location: "/Invoice[1]".to_owned(),
                        evaluation_error: Some("FORG0001".to_owned()),
                    },
                ],
            }],
        };
        let mut output = Vec::new();

        write_check(&mut output, Path::new("a.xml"), &check, true)?;

        let text = String::from_utf8_lossy(&output);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[1..],
            [
                "    schema\tcvc-1\t/Invoice[1]\tfirst second",
                "    error\tBR-01\t/Invoice[1]\tbroken rule",
                "    error\tBR-02\t/Invoice[1]\trule\tnot evaluated: FORG0001",
            ]
        );
        Ok(())
    }

    #[test]
    fn a_run_that_checked_nothing_is_no_success() {
        assert!(!Tally::default().all_conform());
        assert!(
            Tally {
                conforming: 1,
                ..Tally::default()
            }
            .all_conform()
        );
    }

    #[test]
    fn the_wording_of_a_rule_is_put_on_one_line() {
        assert_eq!(single_line(" a\n   b\tc "), "a b c");
    }
}
