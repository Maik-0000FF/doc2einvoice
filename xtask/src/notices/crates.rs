// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The Rust libraries of a package: what `cargo about` finds for one target.
//!
//! The tool chooses a licence for every crate and looks for its text in the
//! package of the crate. Where it finds none it puts the standard text of the
//! licence in its place and says so only by naming no file the text came
//! from. That is read here, and nothing else is decided here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::cargo::{Workspace, cargo, run};
use super::settings::Settings;
use super::{Candidate, Error, read_text, write_text};
use crate::notices::format::LicenseText;

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
    /// The file the text was found in; none where the standard text stands
    /// for it.
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
    Ok(candidates(report, workspace))
}

/// The crates of a report that are not the workspace's own, each with the
/// texts found in its package and the standard texts put in their place.
fn candidates(report: Report, workspace: &Workspace) -> Vec<Candidate> {
    let mut texts: BTreeMap<Identity, (Vec<LicenseText>, Vec<LicenseText>)> = BTreeMap::new();
    for entry in report.licenses {
        for used in &entry.used_by {
            let (own, standard) = texts.entry(used.package.identity()).or_default();
            let text = LicenseText {
                license: entry.id.clone(),
                text: entry.text.clone(),
            };
            if entry.source_path.is_some() {
                own.push(text);
            } else {
                standard.push(text);
            }
        }
    }

    report
        .crates
        .into_iter()
        .filter(|listed| !workspace.owns(&listed.package.manifest_path))
        .map(|listed| {
            let (own, standard) = texts.remove(&listed.package.identity()).unwrap_or_default();
            Candidate {
                lacks_own_text: own.is_empty() || !standard.is_empty(),
                directory: listed
                    .package
                    .manifest_path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_default(),
                changed_copy: listed.package.source.is_none(),
                name: listed.package.name,
                version: listed.package.version,
                license: listed.license,
                own,
                standard,
            }
        })
        .collect()
}
