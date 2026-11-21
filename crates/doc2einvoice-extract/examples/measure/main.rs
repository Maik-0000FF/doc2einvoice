// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Measures the extraction against invoices whose PDF carries the invoice as
//! an embedded Cross Industry Invoice: for each business term, how many of
//! the values the XML holds the draft found, and how many it got wrong.
//!
//! ```text
//! cargo run --example measure -- <folder>...
//! ```
//!
//! Every PDF below the folders is read. The report goes to standard output
//! as Markdown; the invoices themselves stay where they are.

mod compare;
mod embedded;
mod expected;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use doc2einvoice_extract::extract;
use doc2einvoice_model::{Draft, term_by_number};
use doc2einvoice_pdf::PageLayout;
use expected::Expected;

/// The extension of the files read.
const PDF_EXTENSION: &str = "pdf";

/// What happened to the files.
#[derive(Default)]
struct Files {
    measured: usize,
    without_invoice: usize,
    unreadable: Vec<PathBuf>,
    /// How often the path of each term could not be evaluated.
    unevaluated: BTreeMap<u16, usize>,
}

/// The counts of one business term over all measured files.
#[derive(Default, Clone, Copy)]
struct Count {
    expected: usize,
    found: usize,
    correct: usize,
}

fn main() -> ExitCode {
    let folders: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if folders.is_empty() {
        eprintln!("usage: measure <folder>...");
        return ExitCode::FAILURE;
    }
    let expected = Expected::new();
    let mut files = Files::default();
    let mut counts: BTreeMap<u16, Count> = BTreeMap::new();
    for path in folders.iter().flat_map(|folder| pdf_files(folder)) {
        match measure(&path, &expected) {
            Measured::Values(found, unevaluated) => {
                files.measured += 1;
                for term in unevaluated {
                    *files.unevaluated.entry(term).or_default() += 1;
                }
                for (term, count) in found {
                    let total = counts.entry(term).or_default();
                    total.expected += count.expected;
                    total.found += count.found;
                    total.correct += count.correct;
                }
            }
            Measured::WithoutInvoice => files.without_invoice += 1,
            Measured::Unreadable => files.unreadable.push(path),
        }
    }
    print!("{}", report(&files, &counts));
    ExitCode::SUCCESS
}

enum Measured {
    Values(Vec<(u16, Count)>, Vec<u16>),
    WithoutInvoice,
    Unreadable,
}

/// Every PDF below `folder`, in a stable order.
fn pdf_files(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(pdf_files(&path));
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(PDF_EXTENSION))
        {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn measure(path: &Path, expected: &Expected) -> Measured {
    let Ok(bytes) = std::fs::read(path) else {
        return Measured::Unreadable;
    };
    let Some(xml) = embedded::invoice(&bytes) else {
        return Measured::WithoutInvoice;
    };
    let Ok(pages) = doc2einvoice_pdf::read(&bytes) else {
        return Measured::Unreadable;
    };
    let layouts: Vec<PageLayout> = pages.iter().map(PageLayout::of).collect();
    let draft = extract(&layouts);
    let Some(values) = expected.values(&xml) else {
        return Measured::Unreadable;
    };
    let counts = values
        .found
        .into_iter()
        .map(|(term, values)| (term, count(&draft, term, &values)))
        .collect();
    Measured::Values(counts, values.unevaluated)
}

/// How many of `expected` the draft holds for `term`, each value of the draft
/// matched at most once.
fn count(draft: &Draft, term: u16, expected: &[String]) -> Count {
    let Some(data_type) = term_by_number(term).map(|term| term.data_type) else {
        return Count::default();
    };
    let found: Vec<&str> = draft
        .values_of(term)
        .map(|entry| entry.text.as_str())
        .collect();
    let mut used = vec![false; found.len()];
    let mut correct = 0;
    for value in expected {
        let wanted = compare::canonical(data_type, value);
        let matched = found.iter().enumerate().position(|(index, candidate)| {
            !used[index] && compare::same(data_type, candidate, &wanted)
        });
        if let Some(index) = matched {
            used[index] = true;
            correct += 1;
        }
    }
    Count {
        expected: expected.len(),
        found: found.len(),
        correct,
    }
}

fn report(files: &Files, counts: &BTreeMap<u16, Count>) -> String {
    let total = counts.values().fold(Count::default(), |sum, count| Count {
        expected: sum.expected + count.expected,
        found: sum.found + count.found,
        correct: sum.correct + count.correct,
    });
    let mut out = String::new();
    out.push_str("# Extraction measured\n\n");
    let _ = write!(
        out,
        "Files: {} measured, {} without an embedded invoice, {} unreadable.\n\n",
        files.measured,
        files.without_invoice,
        files.unreadable.len()
    );
    let _ = write!(
        out,
        "Values: {} expected, {} found, {} correct, {} wrong, {} missing.\n\n",
        total.expected,
        total.found,
        total.correct,
        total.found - total.correct,
        total.expected - total.correct
    );
    out.push_str("| Term | Name | Expected | Found | Correct | Wrong | Missing |\n|---|---|---|---|---|---|---|\n");
    for (term, count) in counts {
        let name = term_by_number(*term).map_or("", |term| term.name);
        let _ = writeln!(
            out,
            "| BT-{term} | {name} | {} | {} | {} | {} | {} |",
            count.expected,
            count.found,
            count.correct,
            count.found - count.correct,
            count.expected - count.correct
        );
    }
    for (term, times) in &files.unevaluated {
        let _ = write!(out, "\nNot evaluated: BT-{term} in {times} files");
    }
    for path in &files.unreadable {
        let _ = write!(out, "\nUnreadable: {}", path.display());
    }
    out.push('\n');
    out
}
