// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the generator is told: the licences the policy of the dependencies
//! allows, and the settings of the list itself.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use serde::Deserialize;
use thiserror::Error;

/// The settings of the list, in the root of the repository.
pub const SETTINGS_FILE: &str = "notices.toml";
/// The policy of the dependencies, in the root of the repository. It alone
/// says which licences a library may have.
pub const POLICY_FILE: &str = "deny.toml";

/// The part of the policy read here.
#[derive(Debug, Deserialize)]
struct Policy {
    licenses: PolicyLicenses,
}

#[derive(Debug, Deserialize)]
struct PolicyLicenses {
    allow: Vec<String>,
}

/// The settings file as it is written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SettingsFile {
    preference: Vec<String>,
    targets: Vec<String>,
    standard_text: Vec<String>,
    clarify: Vec<ClarifyEntry>,
    changed: Vec<ChangedEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClarifyEntry {
    libraries: Vec<String>,
    license: String,
    file: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangedEntry {
    library: String,
    changes: String,
}

/// The file of a package that holds its licence text, and the licence it
/// states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clarification {
    /// SPDX identifier of the licence.
    pub license: String,
    /// The file, relative to the directory of the package.
    pub file: String,
}

/// Why the two files are not accepted.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Invalid {
    /// One of the files is not what its form asks for.
    #[error("{file}: {message}")]
    Form {
        /// The file.
        file: &'static str,
        /// What its reader says.
        message: String,
    },
    /// A licence is named that the policy does not allow.
    #[error("{SETTINGS_FILE} names the licence {0}, which {POLICY_FILE} does not allow")]
    LicenseNotAllowed(String),
    /// No target is named.
    #[error("{SETTINGS_FILE} names no target")]
    NoTarget,
    /// A library is settled more than once.
    #[error("{SETTINGS_FILE} settles the library {0} more than once")]
    SettledTwice(String),
    /// A change is stated more than once for a library.
    #[error("{SETTINGS_FILE} states the changes of the library {0} more than once")]
    ChangedTwice(String),
    /// A clarification names no file inside the package.
    #[error("{SETTINGS_FILE} names `{0}`, which is not a file inside a package")]
    FileOutsidePackage(String),
    /// A value that has to say something is empty.
    #[error("{SETTINGS_FILE} holds an empty {0}")]
    Empty(&'static str),
}

/// What the generator is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    accepted: Vec<String>,
    targets: Vec<String>,
    standard_text: BTreeSet<String>,
    clarifications: BTreeMap<String, Clarification>,
    changes: BTreeMap<String, String>,
}

/// Whether a clarification names a file inside the package.
///
/// | File                     | Inside |
/// |--------------------------|--------|
/// | `LICENSE`                | yes    |
/// | `docs/LICENSE`           | yes    |
/// | empty                    | no     |
/// | `../LICENSE`             | no     |
/// | `docs/../../LICENSE`     | no     |
/// | `/etc/passwd`            | no     |
/// | `./LICENSE`              | no     |
fn inside_package(file: &str) -> bool {
    let mut components = Path::new(file).components().peekable();
    components.peek().is_some()
        && components.all(|component| matches!(component, Component::Normal(_)))
}

impl Settings {
    /// Reads the policy and the settings from their texts.
    ///
    /// | Input                                                     | Outcome                |
    /// |-----------------------------------------------------------|------------------------|
    /// | both well formed, every name once                         | accepted               |
    /// | a file that is not TOML, or lacks or misnames a key       | refused: form          |
    /// | a preferred or clarified licence the policy does not allow | refused: not allowed  |
    /// | no target                                                 | refused: no target     |
    /// | a library clarified twice, or clarified and excepted      | refused: settled twice |
    /// | a library excepted twice                                  | refused: settled twice |
    /// | the changes of a library stated twice                     | refused: changed twice |
    /// | a clarified file outside the package                      | refused: outside       |
    /// | an empty name, target or statement of changes             | refused: empty         |
    pub fn from_texts(policy: &str, settings: &str) -> Result<Self, Invalid> {
        let policy: Policy = toml::from_str(policy).map_err(|error| Invalid::Form {
            file: POLICY_FILE,
            message: error.to_string(),
        })?;
        let file: SettingsFile = toml::from_str(settings).map_err(|error| Invalid::Form {
            file: SETTINGS_FILE,
            message: error.to_string(),
        })?;
        let allowed = policy.licenses.allow;

        let named = file
            .preference
            .iter()
            .chain(file.clarify.iter().map(|entry| &entry.license));
        if let Some(license) = named.into_iter().find(|license| !allowed.contains(license)) {
            return Err(Invalid::LicenseNotAllowed(license.clone()));
        }
        let mut accepted = Vec::new();
        for license in file.preference.iter().chain(&allowed) {
            if !accepted.contains(license) {
                accepted.push(license.clone());
            }
        }

        if file.targets.is_empty() {
            return Err(Invalid::NoTarget);
        }
        if file.targets.iter().any(|target| target.trim().is_empty()) {
            return Err(Invalid::Empty("target"));
        }

        let mut settled = BTreeSet::new();
        let mut standard_text = BTreeSet::new();
        let mut clarifications = BTreeMap::new();
        for library in file.standard_text {
            Self::settle_once(&mut settled, &library)?;
            standard_text.insert(library);
        }
        for entry in file.clarify {
            if !inside_package(&entry.file) {
                return Err(Invalid::FileOutsidePackage(entry.file));
            }
            for library in entry.libraries {
                Self::settle_once(&mut settled, &library)?;
                clarifications.insert(
                    library,
                    Clarification {
                        license: entry.license.clone(),
                        file: entry.file.clone(),
                    },
                );
            }
        }

        let mut changes = BTreeMap::new();
        for entry in file.changed {
            if entry.library.trim().is_empty() {
                return Err(Invalid::Empty("name of a library"));
            }
            if entry.changes.trim().is_empty() {
                return Err(Invalid::Empty("statement of changes"));
            }
            if changes.contains_key(&entry.library) {
                return Err(Invalid::ChangedTwice(entry.library));
            }
            changes.insert(entry.library, entry.changes);
        }

        Ok(Self {
            accepted,
            targets: file.targets,
            standard_text,
            clarifications,
            changes,
        })
    }

    fn settle_once(settled: &mut BTreeSet<String>, library: &str) -> Result<(), Invalid> {
        if library.trim().is_empty() {
            return Err(Invalid::Empty("name of a library"));
        }
        if settled.insert(library.to_owned()) {
            Ok(())
        } else {
            Err(Invalid::SettledTwice(library.to_owned()))
        }
    }

    /// The licences a library may have: the preferred ones first, in their
    /// order, then the others the policy allows.
    pub fn accepted(&self) -> &[String] {
        &self.accepted
    }

    /// The targets packages are built for.
    pub fn targets(&self) -> &[String] {
        &self.targets
    }

    /// The file that holds the licence text of a library, if one is named.
    pub fn clarification(&self, library: &str) -> Option<&Clarification> {
        self.clarifications.get(library)
    }

    /// Whether the standard text may stand for the text of a library.
    pub fn standard_text_allowed(&self, library: &str) -> bool {
        self.standard_text.contains(library)
    }

    /// What was changed in the copy of a library, if it is stated.
    pub fn changes(&self, library: &str) -> Option<&str> {
        self.changes.get(library).map(String::as_str)
    }

    /// Every library the settings say something about.
    pub fn libraries(&self) -> BTreeSet<&str> {
        self.standard_text
            .iter()
            .chain(self.clarifications.keys())
            .chain(self.changes.keys())
            .map(String::as_str)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Clarification, Invalid, Settings, inside_package};

    const POLICY: &str = "[licenses]\nallow = [\"Apache-2.0\", \"MIT\", \"Zlib\"]\n";

    /// Settings that are accepted, with one line each test may replace.
    fn settings(replace: &str, with: &str) -> String {
        let accepted = "\
preference = [\"MIT\"]
targets = [\"one\", \"two\"]
standard-text = [\"bare\"]

[[clarify]]
libraries = [\"odd\", \"odder\"]
license = \"Zlib\"
file = \"docs/LICENSE\"

[[changed]]
library = \"copy\"
changes = \"One line.\"
";
        assert!(
            replace.is_empty() || accepted.contains(replace),
            "{replace}"
        );
        if replace.is_empty() {
            accepted.to_owned()
        } else {
            accepted.replace(replace, with)
        }
    }

    #[test]
    fn a_file_inside_a_package_follows_its_table() {
        let table = [
            ("LICENSE", true),
            ("docs/LICENSE", true),
            ("", false),
            ("../LICENSE", false),
            ("docs/../../LICENSE", false),
            ("/etc/passwd", false),
            ("./LICENSE", false),
        ];
        for (file, expected) in table {
            assert_eq!(inside_package(file), expected, "{file}");
        }
    }

    #[test]
    fn well_formed_settings_are_accepted() -> Result<(), Invalid> {
        let read = Settings::from_texts(POLICY, &settings("", ""))?;

        assert_eq!(read.accepted(), ["MIT", "Apache-2.0", "Zlib"]);
        assert_eq!(read.targets(), ["one", "two"]);
        assert!(read.standard_text_allowed("bare"));
        assert!(!read.standard_text_allowed("odd"));
        let clarification = Clarification {
            license: "Zlib".to_owned(),
            file: "docs/LICENSE".to_owned(),
        };
        assert_eq!(read.clarification("odd"), Some(&clarification));
        assert_eq!(read.clarification("odder"), Some(&clarification));
        assert_eq!(read.clarification("bare"), None);
        assert_eq!(read.changes("copy"), Some("One line."));
        assert_eq!(read.changes("odd"), None);
        assert_eq!(
            read.libraries().into_iter().collect::<Vec<_>>(),
            ["bare", "copy", "odd", "odder"]
        );
        Ok(())
    }

    #[test]
    fn what_is_refused_follows_its_table() {
        let not_allowed = || Some(Invalid::LicenseNotAllowed("0BSD".to_owned()));
        let twice = |library: &str| Some(Invalid::SettledTwice(library.to_owned()));
        // Where the form is refused, the message is the one of the reader of
        // the file and is not compared: nothing is expected but the refusal.
        let table = [
            ("preference = [\"MIT\"]", "preference = \"MIT\"", None),
            ("preference = [\"MIT\"]", "", None),
            ("standard-text", "standard_text", None),
            (
                "preference = [\"MIT\"]",
                "preference = [\"0BSD\"]",
                not_allowed(),
            ),
            ("license = \"Zlib\"", "license = \"0BSD\"", not_allowed()),
            (
                "targets = [\"one\", \"two\"]",
                "targets = []",
                Some(Invalid::NoTarget),
            ),
            (
                "targets = [\"one\", \"two\"]",
                "targets = [\" \"]",
                Some(Invalid::Empty("target")),
            ),
            ("[\"odd\", \"odder\"]", "[\"odd\", \"odd\"]", twice("odd")),
            ("[\"odd\", \"odder\"]", "[\"bare\"]", twice("bare")),
            ("[\"bare\"]", "[\"bare\", \"bare\"]", twice("bare")),
            (
                "file = \"docs/LICENSE\"",
                "file = \"../LICENSE\"",
                Some(Invalid::FileOutsidePackage("../LICENSE".to_owned())),
            ),
            (
                "changes = \"One line.\"",
                "changes = \" \"",
                Some(Invalid::Empty("statement of changes")),
            ),
        ];
        for (replace, with, expected) in table {
            let result = Settings::from_texts(POLICY, &settings(replace, with));

            match expected {
                Some(refusal) => assert_eq!(result, Err(refusal), "{replace} -> {with}"),
                None => assert!(
                    matches!(result, Err(Invalid::Form { .. })),
                    "{replace} -> {with}: {result:?}"
                ),
            }
        }
    }

    #[test]
    fn the_changes_of_a_library_are_stated_once() {
        let again =
            "changes = \"One line.\"\n\n[[changed]]\nlibrary = \"copy\"\nchanges = \"Again.\"\n";
        let result = Settings::from_texts(POLICY, &settings("changes = \"One line.\"\n", again));

        assert_eq!(result, Err(Invalid::ChangedTwice("copy".to_owned())));
    }

    #[test]
    fn a_policy_without_allowed_licences_is_refused() {
        let result = Settings::from_texts("[licenses]\n", &settings("", ""));

        assert!(matches!(result, Err(Invalid::Form { .. })));
    }
}
