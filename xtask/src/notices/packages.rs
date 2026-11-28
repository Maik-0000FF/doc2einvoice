// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libraries of the interface: the packages whose code the build of the
//! interface put into its bundle.
//!
//! That build writes the directories of those packages into a list. Each is
//! read here: what its manifest states, and the licence file it holds.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::files::{Document, document, regular_files};
use super::format::LicenseText;
use super::{Candidate, Error, Reason, licenses_offered, read_text};

/// The manifest of a package of the interface.
const MANIFEST_FILE: &str = "package.json";

/// The manifest of a package, as far as it is read here.
#[derive(Debug, Deserialize)]
struct Manifest {
    name: String,
    version: String,
    license: String,
}

/// The licence whose text the one licence file of a package is, if that can
/// be told without help.
///
/// A package states its licence itself, and no tool chooses among several
/// for it. Its text is known only where it states one licence and holds one
/// licence file. Everything else has to be settled in the settings, which
/// name the file and the licence.
///
/// | Stated licence              | Licence files  | Outcome                         |
/// |-----------------------------|----------------|---------------------------------|
/// | one, allowed                | one            | that licence: the file is its text |
/// | one, allowed                | none, several  | none: to be settled             |
/// | one, not allowed            | any            | refused: not allowed            |
/// | a choice or a combination   | any            | none: to be settled             |
/// | not an SPDX expression      | any            | none: to be settled             |
fn told_license(
    offered: Option<&[String]>,
    license_files: usize,
    accepted: &[String],
) -> Result<Option<String>, Reason> {
    match offered {
        Some([license]) if !accepted.contains(license) => {
            Err(Reason::LicenseNotAllowed(license.clone()))
        }
        Some([license]) if license_files == 1 => Ok(Some(license.clone())),
        _ => Ok(None),
    }
}

/// Reads the packages the list of the interface build names.
pub fn read(root: &Path, list: &Path, accepted: &[String]) -> Result<Vec<Candidate>, Error> {
    let listed = read_text(list).map_err(|_| Error::InterfaceNotBuilt(list.to_path_buf()))?;
    let directories: Vec<PathBuf> =
        serde_json::from_str(&listed).map_err(|error| Error::Unreadable {
            path: list.to_path_buf(),
            message: error.to_string(),
        })?;

    directories
        .into_iter()
        .map(|directory| package(&root.join(directory), accepted))
        .collect()
}

fn package(directory: &Path, accepted: &[String]) -> Result<Candidate, Error> {
    let path = directory.join(MANIFEST_FILE);
    let manifest: Manifest =
        serde_json::from_str(&read_text(&path)?).map_err(|error| Error::Unreadable {
            path,
            message: error.to_string(),
        })?;
    let mut problems = Vec::new();
    let files = regular_files(directory).unwrap_or_else(|error| {
        problems.push(Reason::PackageUnreadable(error.to_string()));
        Vec::new()
    });
    let license_files: Vec<&String> = files
        .iter()
        .filter(|name| document(name) == Some(Document::License))
        .collect();

    let offered = licenses_offered(&manifest.license);
    let told =
        told_license(offered.as_deref(), license_files.len(), accepted).unwrap_or_else(|reason| {
            problems.push(reason);
            None
        });
    let own = match (told, license_files.as_slice()) {
        (Some(license), [file]) => match std::fs::read_to_string(directory.join(file)) {
            Ok(text) => vec![LicenseText { license, text }],
            Err(error) => {
                problems.push(Reason::PackageUnreadable(format!("{file}: {error}")));
                Vec::new()
            }
        },
        _ => Vec::new(),
    };
    Ok(Candidate {
        name: manifest.name,
        version: manifest.version,
        license: manifest.license,
        directory: directory.to_path_buf(),
        lacks_own_text: own.is_empty(),
        files,
        own,
        // No standard text is at hand for a package of the interface.
        substituted: Vec::new(),
        changed_copy: false,
        problems,
    })
}

#[cfg(test)]
mod tests {
    use super::told_license;
    use crate::notices::Reason;

    #[test]
    fn the_licence_of_the_one_file_follows_its_table() {
        let accepted = ["MIT".to_owned(), "Apache-2.0".to_owned()];
        let one = ["MIT".to_owned()];
        let not_allowed = ["GPL-2.0-only".to_owned()];
        let choice = ["MIT".to_owned(), "Apache-2.0".to_owned()];
        let refused = || Err(Reason::LicenseNotAllowed("GPL-2.0-only".to_owned()));
        let table = [
            (Some(one.as_slice()), 1, Ok(Some("MIT".to_owned()))),
            (Some(one.as_slice()), 0, Ok(None)),
            (Some(one.as_slice()), 2, Ok(None)),
            (Some(not_allowed.as_slice()), 1, refused()),
            (Some(not_allowed.as_slice()), 0, refused()),
            (Some(not_allowed.as_slice()), 2, refused()),
            (Some(choice.as_slice()), 1, Ok(None)),
            (Some(choice.as_slice()), 2, Ok(None)),
            (None, 1, Ok(None)),
            (None, 0, Ok(None)),
            (Some([].as_slice()), 1, Ok(None)),
        ];
        for (offered, license_files, expected) in table {
            assert_eq!(
                told_license(offered, license_files, &accepted),
                expected,
                "{offered:?} {license_files}"
            );
        }
    }
}
