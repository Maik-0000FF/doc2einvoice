// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The libraries of the interface: the packages whose code the build of the
//! interface put into its bundle.
//!
//! That build writes the directories of those packages into a list. Each is
//! read here: what its manifest states, and the files in its root.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::files::{names_license, regular_files};
use super::license::Stated;
use super::location::MANIFEST_FILE;
use super::{Candidate, Error, Reason, read_text};

/// The manifest of a package, as far as it is read here.
#[derive(Debug, Deserialize)]
struct Manifest {
    name: String,
    version: String,
    license: String,
}

/// The licence whose text a package of the interface holds, as far as that
/// can be told without help.
///
/// No tool reads the licence files of such a package. Which licence a file
/// states is known only where the package states one licence and holds a
/// licence file: then the file is the text of that licence. Everything else
/// is settled in the settings, which name the file and the licence.
///
/// | Stated licence              | Licence files in its root | Licence held |
/// |-----------------------------|---------------------------|--------------|
/// | one licence                 | at least one              | that licence |
/// | one licence                 | none                      | none         |
/// | a choice or a combination   | any                       | none         |
/// | not an SPDX expression      | any                       | none         |
fn license_held(stated: &Stated, holds_license_file: bool) -> Option<String> {
    stated.only().filter(|_| holds_license_file)
}

/// Reads the packages the list of the interface build names.
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
    let mut problems = Vec::new();
    let files = regular_files(directory).unwrap_or_else(|error| {
        problems.push(Reason::PackageUnreadable(error.to_string()));
        Vec::new()
    });
    let holds_license_file = files.iter().any(|name| names_license(name));
    let held = license_held(&Stated::read(&manifest.license), holds_license_file);
    Ok(Candidate {
        name: manifest.name,
        version: manifest.version,
        license: manifest.license,
        directory: directory.to_path_buf(),
        files,
        licenses_held: held.into_iter().collect(),
        // The files in its root are passed on as they are; no tool finds
        // further texts, and no standard text is at hand.
        texts_held: Vec::new(),
        standard_texts: Vec::new(),
        changed_copy: false,
        problems,
    })
}

#[cfg(test)]
mod tests {
    use super::license_held;
    use crate::notices::license::Stated;

    #[test]
    fn the_licence_a_package_holds_follows_its_table() {
        let table = [
            ("MIT", true, Some("MIT")),
            ("MIT", false, None),
            ("MIT OR Apache-2.0", true, None),
            ("MIT AND Apache-2.0", true, None),
            ("SEE LICENSE IN LICENSE", true, None),
            ("", true, None),
        ];
        for (stated, holds_license_file, expected) in table {
            assert_eq!(
                license_held(&Stated::read(stated), holds_license_file).as_deref(),
                expected,
                "{stated} {holds_license_file}"
            );
        }
    }
}
