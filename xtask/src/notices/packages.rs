// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libraries of the interface: the packages whose code the build of the
//! interface put into its bundle.
//!
//! That build writes the directories of those packages into a list. Each is
//! read here: what its manifest states, and the licence file it holds.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::format::LicenseText;
use super::{Candidate, Error, file_names, read_text};

/// The list the build of the interface writes, in the directory of the
/// generated lists. `tools/bundled-packages.ts` writes it and names it the
/// same.
pub const BUNDLED_PACKAGES_FILE: &str = "bundled-packages.json";
/// The manifest of a package of the interface.
const MANIFEST_FILE: &str = "package.json";
/// What the name of a licence file starts with, in lower case.
const LICENSE_FILE_STARTS: [&str; 3] = ["licence", "license", "copying"];

/// The manifest of a package, as far as it is read here.
#[derive(Debug, Deserialize)]
struct Manifest {
    name: String,
    version: String,
    license: String,
}

/// Whether a file of a package is a licence file, by its name.
///
/// | Name                  | Licence file |
/// |-----------------------|--------------|
/// | `LICENSE`             | yes          |
/// | `LICENSE.md`          | yes          |
/// | `licence.txt`         | yes          |
/// | `LICENSE-MIT`         | yes          |
/// | `COPYING`             | yes          |
/// | `README.md`           | no           |
/// | `package.json`        | no           |
/// | `relicense.md`        | no           |
/// | empty                 | no           |
fn is_license_file(name: &str) -> bool {
    let name = name.to_lowercase();
    LICENSE_FILE_STARTS
        .iter()
        .any(|start| name.starts_with(start))
}

/// Reads the packages the list of the interface build names.
///
/// A package holds its text when it holds exactly one licence file. With
/// none or with several, the settings have to name the one that applies.
pub fn read(root: &Path, list: &Path) -> Result<Vec<Candidate>, Error> {
    let listed = read_text(list).map_err(|_| Error::InterfaceNotBuilt(list.to_path_buf()))?;
    let directories: Vec<PathBuf> =
        serde_json::from_str(&listed).map_err(|error| Error::Unreadable {
            path: list.to_path_buf(),
            message: error.to_string(),
        })?;

    directories
        .into_iter()
        .map(|directory| package(&root.join(directory)))
        .collect()
}

fn package(directory: &Path) -> Result<Candidate, Error> {
    let path = directory.join(MANIFEST_FILE);
    let manifest: Manifest =
        serde_json::from_str(&read_text(&path)?).map_err(|error| Error::Unreadable {
            path,
            message: error.to_string(),
        })?;
    let files: Vec<String> = file_names(directory)?
        .into_iter()
        .filter(|name| is_license_file(name))
        .collect();
    let own = match files.as_slice() {
        [file] => vec![LicenseText {
            license: manifest.license.clone(),
            text: read_text(&directory.join(file))?,
        }],
        _ => Vec::new(),
    };
    Ok(Candidate {
        name: manifest.name,
        version: manifest.version,
        license: manifest.license,
        directory: directory.to_path_buf(),
        lacks_own_text: own.is_empty(),
        own,
        // No standard text is at hand for a package of the interface.
        standard: Vec::new(),
        changed_copy: false,
    })
}

#[cfg(test)]
mod tests {
    use super::is_license_file;

    #[test]
    fn a_licence_file_follows_its_table() {
        let table = [
            ("LICENSE", true),
            ("LICENSE.md", true),
            ("licence.txt", true),
            ("LICENSE-MIT", true),
            ("COPYING", true),
            ("README.md", false),
            ("package.json", false),
            ("relicense.md", false),
            ("", false),
        ];
        for (name, expected) in table {
            assert_eq!(is_license_file(name), expected, "{name}");
        }
    }
}
