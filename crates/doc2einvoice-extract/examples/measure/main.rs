// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Measures the extraction against invoices whose PDF carries the invoice as
//! an embedded Cross Industry Invoice: for each business term, how many of
//! the values the XML holds the draft found, and how many it got wrong.
//!
//! ```text
//! cargo run --example measure -- [--differences] <folder>...
//! ```
//!
//! With `--differences`, the report also lists each place at which the draft
//! differs from the XML, for the terms the extraction found anywhere.
//!
//! Every PDF below the folders is read; links to folders are not followed.
//! The report goes to standard output as Markdown; the invoices themselves
//! stay where they are. A folder that cannot be read ends the measurement
//! with an error.

mod compare;
mod embedded;
mod expected;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use doc2einvoice_extract::extract;
use doc2einvoice_model::{Draft, Term, term_by_number};
use doc2einvoice_pdf::PageLayout;
use expected::{Expected, Place};

/// The option that lists the differences.
const DIFFERENCES: &str = "--differences";

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

/// The counts of one business term.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Count {
    expected: usize,
    found: usize,
    correct: usize,
}

impl Count {
    fn add(&mut self, other: Self) {
        self.expected += other.expected;
        self.found += other.found;
        self.correct += other.correct;
    }

    /// Values found that the XML does not hold at their place.
    fn wrong(self) -> usize {
        self.found - self.correct
    }

    /// Values the XML holds that were not found at their place.
    fn missing(self) -> usize {
        self.expected - self.correct
    }
}

/// A place at which the draft differs from the XML.
struct Difference {
    place: Place,
    expected: Vec<String>,
    found: Vec<String>,
}

enum Measured {
    Values(BTreeMap<u16, Count>, Vec<Difference>, Vec<u16>),
    WithoutInvoice,
    Unreadable,
}

fn main() -> ExitCode {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let listed = arguments.iter().any(|argument| argument == DIFFERENCES);
    let folders: Vec<PathBuf> = arguments
        .into_iter()
        .filter(|argument| argument != DIFFERENCES)
        .map(PathBuf::from)
        .collect();
    if folders.is_empty() {
        eprintln!("usage: measure [{DIFFERENCES}] <folder>...");
        return ExitCode::FAILURE;
    }
    let mut paths = Vec::new();
    for folder in &folders {
        match pdf_files(folder) {
            Ok(found) => paths.extend(found),
            Err(error) => {
                eprintln!("{}: {error}", folder.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let expected = Expected::new();
    let mut files = Files::default();
    let mut counts: BTreeMap<u16, Count> = BTreeMap::new();
    let mut differences: Vec<(PathBuf, Difference)> = Vec::new();
    for path in paths {
        match measure(&path, &expected) {
            Measured::Values(found, differing, unevaluated) => {
                files.measured += 1;
                for term in unevaluated {
                    *files.unevaluated.entry(term).or_default() += 1;
                }
                for (term, count) in found {
                    counts.entry(term).or_default().add(count);
                }
                differences.extend(
                    differing
                        .into_iter()
                        .map(|difference| (path.clone(), difference)),
                );
            }
            Measured::WithoutInvoice => files.without_invoice += 1,
            Measured::Unreadable => files.unreadable.push(path),
        }
    }
    print!("{}", report(&files, &counts));
    if listed {
        print!("{}", listing(&differences, &counts));
    }
    ExitCode::SUCCESS
}

/// Every PDF below `folder`, in a stable order.
///
/// | Entry below the folder        | Result                   |
/// |-------------------------------|--------------------------|
/// | a file `a.pdf` or `A.PDF`     | listed                   |
/// | a file `a.txt`                | left out                 |
/// | a folder                      | searched                 |
/// | a link to a PDF               | listed                   |
/// | a link to a folder            | left out, not followed   |
/// | a folder that cannot be read  | error                    |
/// | `folder` missing or no folder | error                    |
fn pdf_files(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            found.extend(pdf_files(&path)?);
        } else if (kind.is_file() || (kind.is_symlink() && path.is_file())) && is_pdf(&path) {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PDF_EXTENSION))
}

fn measure(path: &Path, expected: &Expected) -> Measured {
    let Ok(bytes) = std::fs::read(path) else {
        return Measured::Unreadable;
    };
    let xml = match embedded::invoice(&bytes) {
        Ok(Some(xml)) => xml,
        Ok(None) => return Measured::WithoutInvoice,
        Err(_) => return Measured::Unreadable,
    };
    let Ok(pages) = doc2einvoice_pdf::read(&bytes) else {
        return Measured::Unreadable;
    };
    let layouts: Vec<PageLayout> = pages.iter().map(PageLayout::of).collect();
    let draft = extract(&layouts);
    let Some(values) = expected.values(&xml) else {
        return Measured::Unreadable;
    };
    let (counts, differing) = tally(&draft, &values.found);
    Measured::Values(counts, differing, values.unevaluated)
}

/// The counts per business term, over every place the XML or the draft has
/// a value at. A draft value counts at the outermost repeated group of its
/// entry, and matches at most one value of the XML at the same place.
///
/// | XML at a place   | Draft at the same place | Expected | Found | Correct |
/// |------------------|-------------------------|----------|-------|---------|
/// | `A`              | `A`                     | 1        | 1     | 1       |
/// | `A`              | nothing                 | 1        | 0     | 0       |
/// | nothing          | `A`                     | 0        | 1     | 0       |
/// | `A` in line 1, `B` in line 2 | `A` in line 1, `B` in line 2 | 2 | 2 | 2 |
/// | `A` in line 1, `B` in line 2 | `B` in line 1, `A` in line 2 | 2 | 2 | 0 |
/// | `A`, `A`         | `A`                     | 2        | 1     | 1       |
/// | `A`              | `A`, `A`                | 1        | 2     | 1       |
/// | `A` of BT-1      | `A` under a number no term has | BT-1: 1 | 0 | 0 |
/// |                  |                         | the number: 0 | 1 | 0 |
///
/// The places at which not every value matches are returned as well.
fn tally(
    draft: &Draft,
    expected: &BTreeMap<Place, Vec<String>>,
) -> (BTreeMap<u16, Count>, Vec<Difference>) {
    let mut found: BTreeMap<Place, Vec<&str>> = BTreeMap::new();
    for entry in &draft.entries {
        let place = Place {
            term: entry.term,
            occurrence: entry.within.first().copied(),
        };
        found.entry(place).or_default().push(&entry.text);
    }
    let places: BTreeSet<Place> = expected.keys().chain(found.keys()).copied().collect();
    let mut counts: BTreeMap<u16, Count> = BTreeMap::new();
    let mut differences = Vec::new();
    for place in places {
        let wanted = expected.get(&place).map_or(&[][..], Vec::as_slice);
        let given = found.get(&place).map_or(&[][..], Vec::as_slice);
        let count = count(place.term, given, wanted);
        if count.wrong() > 0 || count.missing() > 0 {
            differences.push(Difference {
                place,
                expected: wanted.to_vec(),
                found: given.iter().map(|text| (*text).to_owned()).collect(),
            });
        }
        counts.entry(place.term).or_default().add(count);
    }
    (counts, differences)
}

/// How many of `expected` are among `found`, each value found used once.
fn count(term: u16, found: &[&str], expected: &[String]) -> Count {
    let mut used = vec![false; found.len()];
    let mut correct = 0;
    if let Some(term) = term_by_number(term) {
        for value in expected {
            let wanted = compare::canonical(term.data_type, value);
            let matched = (0..found.len()).find(|index| {
                !used[*index] && compare::same(term.data_type, found[*index], &wanted)
            });
            if let Some(index) = matched {
                used[index] = true;
                correct += 1;
            }
        }
    }
    Count {
        expected: expected.len(),
        found: found.len(),
        correct,
    }
}

/// The identifier and name of a term, or its bare number if the model does
/// not know it.
fn label(term: u16) -> (String, &'static str) {
    term_by_number(term).map_or_else(
        || (term.to_string(), ""),
        |known| (Term::id(known), known.name),
    )
}

/// What separates several values in one cell of the listing.
const VALUE_SEPARATOR: &str = " / ";

/// `text` as one cell of a Markdown table: its bars escaped, its line breaks
/// turned into spaces.
///
/// | Text      | Cell       |
/// |-----------|------------|
/// | `A`       | `A`        |
/// | `A \| B`  | `A \\| B`  |
/// | `A`, line break, `B` | `A B` |
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace(['\n', '\r'], " ")
}

/// The differences at terms the extraction found a value for anywhere.
fn listing(differences: &[(PathBuf, Difference)], counts: &BTreeMap<u16, Count>) -> String {
    let mut out =
        String::from("\n## Differences\n\n| File | Term | Expected | Found |\n|---|---|---|---|\n");
    for (path, difference) in differences {
        let term = difference.place.term;
        if counts.get(&term).is_none_or(|count| count.found == 0) {
            continue;
        }
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            cell(&path.display().to_string()),
            label(term).0,
            cell(&difference.expected.join(VALUE_SEPARATOR)),
            cell(&difference.found.join(VALUE_SEPARATOR))
        );
    }
    out
}

fn report(files: &Files, counts: &BTreeMap<u16, Count>) -> String {
    let mut total = Count::default();
    for count in counts.values() {
        total.add(*count);
    }
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
        total.wrong(),
        total.missing()
    );
    out.push_str("| Term | Name | Expected | Found | Correct | Wrong | Missing |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for (term, count) in counts {
        let (id, name) = label(*term);
        let _ = writeln!(
            out,
            "| {id} | {name} | {} | {} | {} | {} | {} |",
            count.expected,
            count.found,
            count.correct,
            count.wrong(),
            count.missing()
        );
    }
    for (term, times) in &files.unevaluated {
        let _ = write!(out, "\nNot evaluated: {} in {times} files", label(*term).0);
    }
    for path in &files.unreadable {
        let _ = write!(out, "\nUnreadable: {}", path.display());
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use doc2einvoice_model::{Draft, Entry, Occurrence, Origin};

    use super::{Count, Place, cell, pdf_files, tally};

    /// The business group of an invoice line, and two terms: the invoice
    /// number and the item name.
    const LINE: u16 = 25;
    const NUMBER: u16 = 1;
    const ITEM: u16 = 153;
    /// A number the model assigns no term.
    const UNKNOWN: u16 = 4;

    fn line(index: usize) -> Occurrence {
        Occurrence { group: LINE, index }
    }

    fn entry(term: u16, occurrence: Option<Occurrence>, text: &str) -> Entry {
        Entry {
            term,
            within: occurrence.into_iter().collect(),
            text: text.to_owned(),
            origin: Origin::Rule,
            location: None,
        }
    }

    fn xml(values: &[(u16, Option<Occurrence>, &str)]) -> BTreeMap<Place, Vec<String>> {
        let mut map: BTreeMap<Place, Vec<String>> = BTreeMap::new();
        for (term, occurrence, text) in values {
            map.entry(Place {
                term: *term,
                occurrence: *occurrence,
            })
            .or_default()
            .push((*text).to_owned());
        }
        map
    }

    fn counted(draft: &[Entry], expected: &[(u16, Option<Occurrence>, &str)], term: u16) -> Count {
        let draft = Draft {
            entries: draft.to_vec(),
        };
        tally(&draft, &xml(expected))
            .0
            .get(&term)
            .copied()
            .unwrap_or_default()
    }

    fn count(expected: usize, found: usize, correct: usize) -> Count {
        Count {
            expected,
            found,
            correct,
        }
    }

    #[test]
    fn tallies_follow_their_table() {
        let a = [(NUMBER, None, "A")];
        assert_eq!(
            counted(&[entry(NUMBER, None, "A")], &a, NUMBER),
            count(1, 1, 1)
        );
        assert_eq!(counted(&[], &a, NUMBER), count(1, 0, 0));
        assert_eq!(
            counted(&[entry(NUMBER, None, "A")], &[], NUMBER),
            count(0, 1, 0)
        );
        let lines = [(ITEM, Some(line(0)), "A"), (ITEM, Some(line(1)), "B")];
        let placed = [
            entry(ITEM, Some(line(0)), "A"),
            entry(ITEM, Some(line(1)), "B"),
        ];
        assert_eq!(counted(&placed, &lines, ITEM), count(2, 2, 2));
        let swapped = [
            entry(ITEM, Some(line(0)), "B"),
            entry(ITEM, Some(line(1)), "A"),
        ];
        assert_eq!(counted(&swapped, &lines, ITEM), count(2, 2, 0));
        let twice = [(NUMBER, None, "A"), (NUMBER, None, "A")];
        assert_eq!(
            counted(&[entry(NUMBER, None, "A")], &twice, NUMBER),
            count(2, 1, 1)
        );
        let doubled = [entry(NUMBER, None, "A"), entry(NUMBER, None, "A")];
        assert_eq!(counted(&doubled, &a, NUMBER), count(1, 2, 1));
        let unknown = [entry(UNKNOWN, None, "A")];
        assert_eq!(counted(&unknown, &a, NUMBER), count(1, 0, 0));
        assert_eq!(counted(&unknown, &a, UNKNOWN), count(0, 1, 0));
    }

    /// A folder of its own below the temporary folder, removed afterwards.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("measure-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("sub"))
                .unwrap_or_else(|error| unreachable!("{error}"));
            Self(path)
        }

        fn file(&self, name: &str) {
            std::fs::write(self.0.join(name), b"").unwrap_or_else(|error| unreachable!("{error}"));
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn files_are_found_after_their_table() {
        let scratch = Scratch::new("files");
        for name in ["a.pdf", "B.PDF", "c.txt", "sub/d.pdf"] {
            scratch.file(name);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&scratch.0, scratch.0.join("sub/loop"))
                .unwrap_or_else(|error| unreachable!("{error}"));
            symlink(scratch.0.join("a.pdf"), scratch.0.join("e.pdf"))
                .unwrap_or_else(|error| unreachable!("{error}"));
        }
        let found = pdf_files(&scratch.0).unwrap_or_else(|error| unreachable!("{error}"));
        let names: Vec<String> = found
            .iter()
            .filter_map(|path| path.strip_prefix(&scratch.0).ok())
            .map(|path| {
                // Joined with one separator, whatever the system writes between folders.
                let parts: Vec<String> = path
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect();
                parts.join("/")
            })
            .collect();
        let mut expected = vec!["B.PDF", "a.pdf", "sub/d.pdf"];
        if cfg!(unix) {
            expected.push("e.pdf");
            expected.sort_unstable();
        }
        assert_eq!(names, expected);
    }

    #[test]
    fn cells_follow_their_table() {
        assert_eq!(cell("A"), "A");
        assert_eq!(cell("A | B"), "A \\| B");
        assert_eq!(cell("A\nB"), "A B");
        assert_eq!(cell("A\r\nB"), "A  B");
    }

    #[test]
    fn a_missing_folder_is_an_error() {
        let scratch = Scratch::new("missing");
        assert!(pdf_files(&scratch.0.join("nowhere")).is_err());
    }
}
