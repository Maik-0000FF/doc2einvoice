// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Rust libraries of a package: what `cargo about` finds for one target.
//!
//! The tool chooses a licence for every crate and looks for its text in the
//! package of the crate. Where it finds none it puts the standard text of the
//! licence in its place. It then holds every text once, for all crates the
//! text applies to, and names one file the text was found in: the file of one
//! of those crates, not of each. Whether a text comes from the package of a
//! crate is therefore decided here, crate by crate.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::cargo::{Workspace, cargo, run};
use super::files::{holds_text, regular_files};
use super::format::LicenseText;
use super::settings::Settings;
use super::{Candidate, Error, Reason, read_text, write_text};

/// The directory below the one Cargo builds in that holds what the tool is
/// given and what it answers.
const WORK_DIRECTORY: &str = "notices";

/// The configuration the tool is given for one target.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
struct ToolConfiguration<'a> {
    /// The licences a crate may have; of a choice, the first of these.
    accepted: &'a [String],
    targets: [&'a str; 1],
    /// A crate that only takes part in the build is listed as well: whether
    /// it leaves code in the program is not something to guess.
    ignore_build_dependencies: bool,
    /// What only the tests use is in no package.
    ignore_dev_dependencies: bool,
}

/// What the tool answers, as far as it is read here.
#[derive(Debug, Deserialize)]
struct Report {
    licenses: Vec<Entry>,
    crates: Vec<Listed>,
}

/// A licence text and the crates it applies to.
#[derive(Debug, Deserialize)]
struct Entry {
    id: String,
    text: String,
    /// A file the text was found in, of one of the crates; none where the
    /// standard text stands for it.
    source_path: Option<PathBuf>,
    used_by: Vec<Use>,
}

#[derive(Debug, Deserialize)]
struct Use {
    #[serde(rename = "crate")]
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Listed {
    package: Package,
    /// The licence the tool reads for the crate, as an SPDX expression.
    license: String,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
    /// Where the crate was published; none for one that lies in a directory.
    source: Option<String>,
    manifest_path: PathBuf,
}

/// A crate by what tells it from every other.
type Identity = (String, String);

impl Package {
    fn identity(&self) -> Identity {
        (self.name.clone(), self.version.clone())
    }
}

/// Whether a text the tool reports for a crate comes from the package of
/// that crate.
///
/// | The file the tool names   | The package of the crate                | From its own package |
/// |---------------------------|-----------------------------------------|----------------------|
/// | inside the package        | not asked                               | yes                  |
/// | inside another package    | a file of it holds the text as it is    | yes                  |
/// | inside another package    | no file of it holds the text            | no                   |
/// | none: the standard text   | not asked                               | no                   |
fn from_own_package(
    named: Option<&Path>,
    directory: &Path,
    package_holds_text: impl FnOnce() -> bool,
) -> bool {
    match named {
        None => false,
        Some(file) if file.starts_with(directory) => true,
        Some(_) => package_holds_text(),
    }
}

/// Asks the tool for the crates the application contains for one target.
/// The sources have to be fetched before; the tool is kept from the network.
pub fn read(
    workspace: &Workspace,
    settings: &Settings,
    target: &str,
) -> Result<Vec<Candidate>, Error> {
    let directory = workspace.target_directory.join(WORK_DIRECTORY);
    let configuration = directory.join(format!("{target}.toml"));
    let answer = directory.join(format!("{target}.json"));
    let written = toml::to_string(&ToolConfiguration {
        accepted: settings.accepted(),
        targets: [target],
        ignore_build_dependencies: false,
        ignore_dev_dependencies: true,
    })
    .map_err(|error| Error::Unreadable {
        path: configuration.clone(),
        message: error.to_string(),
    })?;
    write_text(&configuration, &written)?;

    run(cargo()
        .current_dir(&workspace.root)
        .args(["about", "generate", "--offline", "--locked"])
        .args(["--format", "json"])
        .arg("--config")
        .arg(&configuration)
        .arg("--manifest-path")
        .arg(&workspace.application)
        .arg("--output-file")
        .arg(&answer))?;

    let report: Report =
        serde_json::from_str(&read_text(&answer)?).map_err(|error| Error::Unreadable {
            path: answer,
            message: error.to_string(),
        })?;
    Ok(candidates(&report, workspace))
}

/// The crates of a report that are not the workspace's own, each with the
/// texts its own package holds and the texts that stand in for those it does
/// not hold.
fn candidates(report: &Report, workspace: &Workspace) -> Vec<Candidate> {
    let mut entries: BTreeMap<Identity, Vec<&Entry>> = BTreeMap::new();
    for entry in &report.licenses {
        for used in &entry.used_by {
            entries
                .entry(used.package.identity())
                .or_default()
                .push(entry);
        }
    }

    report
        .crates
        .iter()
        .filter(|listed| !workspace.owns(&listed.package.manifest_path))
        .map(|listed| {
            let package = &listed.package;
            let directory = package
                .manifest_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            let mut problems = Vec::new();
            let files = regular_files(&directory).unwrap_or_else(|error| {
                problems.push(Reason::PackageUnreadable(error.to_string()));
                Vec::new()
            });

            let mut own = Vec::new();
            let mut substituted = Vec::new();
            for entry in entries.get(&package.identity()).into_iter().flatten() {
                let text = LicenseText {
                    license: entry.id.clone(),
                    text: entry.text.clone(),
                };
                let holds = || holds_text(&directory, &files, &entry.text);
                if from_own_package(entry.source_path.as_deref(), &directory, holds) {
                    own.push(text);
                } else {
                    substituted.push(text);
                }
            }
            Candidate {
                name: package.name.clone(),
                version: package.version.clone(),
                license: listed.license.clone(),
                lacks_own_text: own.is_empty() || !substituted.is_empty(),
                changed_copy: package.source.is_none(),
                directory,
                files,
                own,
                substituted,
                problems,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::from_own_package;

    #[test]
    fn whether_a_text_comes_from_the_own_package_follows_its_table() {
        let directory = Path::new("registry/library-1.0.0");
        let inside = Path::new("registry/library-1.0.0/LICENSE");
        // The name of another package starts like that of this one: it is
        // another directory all the same.
        let another = Path::new("registry/library-1.0.0-extra/LICENSE");
        let table = [
            (Some(inside), true, true),
            (Some(inside), false, true),
            (Some(another), true, true),
            (Some(another), false, false),
            (None, true, false),
            (None, false, false),
        ];
        for (named, holds, expected) in table {
            assert_eq!(
                from_own_package(named, directory, || holds),
                expected,
                "{named:?} {holds}"
            );
        }
    }

    #[test]
    fn the_package_is_not_asked_where_the_named_file_decides() {
        let directory = Path::new("registry/library-1.0.0");
        let inside = Path::new("registry/library-1.0.0/LICENSE");
        let asked = || unreachable!("the package was asked");

        assert!(from_own_package(Some(inside), directory, asked));
        assert!(!from_own_package(None, directory, asked));
    }
}
