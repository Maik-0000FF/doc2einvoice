// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the generator is told: the licences the policy of the dependencies
//! allows, and the settings of the list itself.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;
use thiserror::Error;

use super::location::stays_below;

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

/// The settings file as it is written. The lists of libraries may be left
/// out: a list whose last entry was removed is no error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SettingsFile {
    preference: Vec<String>,
    targets: Vec<String>,
    #[serde(default)]
    combined: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    standard_text: Vec<Named>,
    #[serde(default)]
    clarify: Vec<ClarifyEntry>,
    #[serde(default)]
    changed: Vec<ChangedEntry>,
}

/// A library as the settings name it: by its name alone, or by its name and
/// one version.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Named {
    Name(String),
    Version(NamedVersion),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedVersion {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClarifyEntry {
    libraries: Vec<Named>,
    license: String,
    file: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangedEntry {
    library: String,
    version: Option<String>,
    changes: String,
}

/// The libraries a setting applies to: every version of a name, or one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Selector {
    name: String,
    version: Option<String>,
}

impl fmt::Display for Selector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(version) => write!(formatter, "{} {version}", self.name),
            None => write!(formatter, "{}", self.name),
        }
    }
}

impl Selector {
    fn new(name: String, version: Option<String>) -> Result<Self, Invalid> {
        if name.trim().is_empty() {
            return Err(Invalid::Empty("name of a library"));
        }
        if version
            .as_ref()
            .is_some_and(|version| version.trim().is_empty())
        {
            return Err(Invalid::Empty("version of a library"));
        }
        Ok(Self { name, version })
    }

    /// The selectors that apply to a version of a library, the closer one
    /// first: the one for that version, then the one for every version.
    fn applying(name: &str, version: &str) -> [Self; 2] {
        [
            Self {
                name: name.to_owned(),
                version: Some(version.to_owned()),
            },
            Self {
                name: name.to_owned(),
                version: None,
            },
        ]
    }
}

impl TryFrom<Named> for Selector {
    type Error = Invalid;

    fn try_from(named: Named) -> Result<Self, Invalid> {
        match named {
            Named::Name(name) => Self::new(name, None),
            Named::Version(NamedVersion { name, version }) => Self::new(name, Some(version)),
        }
    }
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
    SettledTwice(Selector),
    /// A change is stated more than once for a library.
    #[error("{SETTINGS_FILE} states the changes of the library {0} more than once")]
    ChangedTwice(Selector),
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
    combined: BTreeMap<String, Vec<String>>,
    standard_text: BTreeSet<Selector>,
    clarifications: BTreeMap<Selector, Clarification>,
    changes: BTreeMap<Selector, String>,
}

/// The entry of a map that applies to a version of a library.
///
/// | Entries for the name              | Asked version | Applies               |
/// |-----------------------------------|---------------|-----------------------|
/// | one for every version             | any           | that one              |
/// | one for version 1                 | 1             | that one              |
/// | one for version 1                 | 2             | none                  |
/// | one for every version, one for 1  | 1             | the one for version 1 |
/// | one for every version, one for 1  | 2             | the one for every     |
/// | none                              | any           | none                  |
fn applying<'a, T>(
    entries: &'a BTreeMap<Selector, T>,
    name: &str,
    version: &str,
) -> Option<(&'a Selector, &'a T)> {
    Selector::applying(name, version)
        .iter()
        .find_map(|selector| entries.get_key_value(selector))
}

impl Settings {
    /// Reads the policy and the settings from their texts.
    ///
    /// | Input                                                      | Outcome                |
    /// |------------------------------------------------------------|------------------------|
    /// | both well formed, every library settled once               | accepted               |
    /// | the lists of libraries left out                            | accepted, as empty     |
    /// | a library settled for every version and for one version    | accepted               |
    /// | a file that is not TOML, or misnames a key                 | refused: form          |
    /// | the preference or the targets left out                     | refused: form          |
    /// | a preferred or clarified licence the policy does not allow | refused: not allowed   |
    /// | no target, or a combined target that stands for none       | refused: no target     |
    /// | a library clarified twice, or clarified and excepted       | refused: settled twice |
    /// | a library excepted twice                                   | refused: settled twice |
    /// | the changes of a library stated twice                      | refused: changed twice |
    /// | a clarified file outside the package                       | refused: outside       |
    /// | an empty name, version, target or statement of changes     | refused: empty         |
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

        if file.targets.is_empty() || file.combined.values().any(Vec::is_empty) {
            return Err(Invalid::NoTarget);
        }
        let targets = file
            .targets
            .iter()
            .chain(file.combined.keys())
            .chain(file.combined.values().flatten());
        if targets.into_iter().any(|target| target.trim().is_empty()) {
            return Err(Invalid::Empty("target"));
        }

        let mut settled = BTreeSet::new();
        let mut standard_text = BTreeSet::new();
        let mut clarifications = BTreeMap::new();
        for library in file.standard_text {
            let selector = Selector::try_from(library)?;
            Self::settle_once(&mut settled, &selector)?;
            standard_text.insert(selector);
        }
        for entry in file.clarify {
            if !stays_below(&entry.file) {
                return Err(Invalid::FileOutsidePackage(entry.file));
            }
            for library in entry.libraries {
                let selector = Selector::try_from(library)?;
                Self::settle_once(&mut settled, &selector)?;
                clarifications.insert(
                    selector,
                    Clarification {
                        license: entry.license.clone(),
                        file: entry.file.clone(),
                    },
                );
            }
        }

        let mut changes = BTreeMap::new();
        for entry in file.changed {
            let selector = Selector::new(entry.library, entry.version)?;
            if entry.changes.trim().is_empty() {
                return Err(Invalid::Empty("statement of changes"));
            }
            if changes.contains_key(&selector) {
                return Err(Invalid::ChangedTwice(selector));
            }
            changes.insert(selector, entry.changes);
        }

        Ok(Self {
            accepted,
            targets: file.targets,
            combined: file.combined,
            standard_text,
            clarifications,
            changes,
        })
    }

    fn settle_once(settled: &mut BTreeSet<Selector>, selector: &Selector) -> Result<(), Invalid> {
        if settled.insert(selector.clone()) {
            Ok(())
        } else {
            Err(Invalid::SettledTwice(selector.clone()))
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

    /// The targets a package for the given one is built for: those a
    /// combined target stands for, or the target itself.
    pub fn built_for<'a>(&'a self, target: &'a String) -> &'a [String] {
        self.combined
            .get(target)
            .map_or(std::slice::from_ref(target), Vec::as_slice)
    }

    /// The file that holds the licence text of a library, if one is named,
    /// with the setting that names it.
    pub fn clarification(&self, name: &str, version: &str) -> Option<(&Selector, &Clarification)> {
        applying(&self.clarifications, name, version)
    }

    /// The setting that lets the standard text stand for the text of a
    /// library, if there is one.
    pub fn standard_text(&self, name: &str, version: &str) -> Option<&Selector> {
        Selector::applying(name, version)
            .iter()
            .find_map(|selector| self.standard_text.get(selector))
    }

    /// What was changed in the copy of a library, if it is stated, with the
    /// setting that states it.
    pub fn changes(&self, name: &str, version: &str) -> Option<(&Selector, &str)> {
        applying(&self.changes, name, version)
            .map(|(selector, changes)| (selector, changes.as_str()))
    }

    /// Every setting that names a library.
    pub fn selectors(&self) -> BTreeSet<&Selector> {
        self.standard_text
            .iter()
            .chain(self.clarifications.keys())
            .chain(self.changes.keys())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Clarification, Invalid, Selector, Settings, applying};

    const POLICY: &str = "[licenses]\nallow = [\"Apache-2.0\", \"MIT\", \"Zlib\"]\n";

    /// Settings that are accepted, of which each test may replace one part.
    fn settings(replace: &str, with: &str) -> String {
        let accepted = "\
preference = [\"MIT\"]
targets = [\"one\", \"two\"]
standard-text = [\"bare\", { name = \"split\", version = \"2.0.0\" }]

[combined]
both = [\"one\", \"two\"]

[[clarify]]
libraries = [\"odd\", \"odder\", { name = \"bare\", version = \"1.0.0\" }]
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

    fn selector(name: &str, version: Option<&str>) -> Selector {
        Selector {
            name: name.to_owned(),
            version: version.map(str::to_owned),
        }
    }

    #[test]
    fn the_entry_that_applies_follows_its_table() {
        let entries = BTreeMap::from([
            (selector("both", None), "every"),
            (selector("both", Some("1")), "one"),
            (selector("every", None), "every"),
            (selector("one", Some("1")), "one"),
        ]);
        let table = [
            ("every", "1", Some("every")),
            ("every", "2", Some("every")),
            ("one", "1", Some("one")),
            ("one", "2", None),
            ("both", "1", Some("one")),
            ("both", "2", Some("every")),
            ("none", "1", None),
        ];
        for (name, version, expected) in table {
            assert_eq!(
                applying(&entries, name, version).map(|(_, entry)| *entry),
                expected,
                "{name} {version}"
            );
        }
    }

    #[test]
    fn well_formed_settings_are_accepted() -> Result<(), Invalid> {
        let read = Settings::from_texts(POLICY, &settings("", ""))?;
        let clarification = Clarification {
            license: "Zlib".to_owned(),
            file: "docs/LICENSE".to_owned(),
        };
        let clarified = |name, version| {
            read.clarification(name, version)
                .map(|(selector, clarification)| (selector.to_string(), clarification))
        };

        assert_eq!(read.accepted(), ["MIT", "Apache-2.0", "Zlib"]);
        assert_eq!(read.targets(), ["one", "two"]);
        assert_eq!(read.built_for(&"both".to_owned()), ["one", "two"]);
        assert_eq!(read.built_for(&"other".to_owned()), ["other"]);
        // A setting for one version comes before the one for every version.
        assert_eq!(
            clarified("bare", "1.0.0"),
            Some(("bare 1.0.0".to_owned(), &clarification))
        );
        assert_eq!(clarified("bare", "2.0.0"), None);
        assert_eq!(
            clarified("odd", "3.1.0"),
            Some(("odd".to_owned(), &clarification))
        );
        assert_eq!(
            read.standard_text("bare", "2.0.0").map(Selector::to_string),
            Some("bare".to_owned())
        );
        assert_eq!(
            read.standard_text("split", "2.0.0")
                .map(Selector::to_string),
            Some("split 2.0.0".to_owned())
        );
        assert_eq!(read.standard_text("split", "1.0.0"), None);
        assert_eq!(read.standard_text("odd", "1.0.0"), None);
        assert_eq!(
            read.changes("copy", "0.1.0")
                .map(|(selector, changes)| (selector.to_string(), changes)),
            Some(("copy".to_owned(), "One line."))
        );
        assert_eq!(read.changes("odd", "1.0.0"), None);
        assert_eq!(
            read.selectors()
                .into_iter()
                .map(Selector::to_string)
                .collect::<Vec<_>>(),
            ["bare", "bare 1.0.0", "copy", "odd", "odder", "split 2.0.0"]
        );
        Ok(())
    }

    #[test]
    fn the_lists_of_libraries_may_be_left_out() -> Result<(), Invalid> {
        let read = Settings::from_texts(POLICY, "preference = []\ntargets = [\"one\"]\n")?;

        assert_eq!(read.accepted(), ["Apache-2.0", "MIT", "Zlib"]);
        assert!(read.selectors().is_empty());
        assert_eq!(read.built_for(&"one".to_owned()), ["one"]);
        Ok(())
    }

    #[test]
    fn what_is_refused_follows_its_table() {
        let not_allowed = || Some(Invalid::LicenseNotAllowed("0BSD".to_owned()));
        let twice = |name, version| Some(Invalid::SettledTwice(selector(name, version)));
        let libraries = "[\"odd\", \"odder\", { name = \"bare\", version = \"1.0.0\" }]";
        let excepted = "[\"bare\", { name = \"split\", version = \"2.0.0\" }]";
        let changed = "library = \"copy\"\nchanges = \"One line.\"\n";
        let changed_twice = "library = \"copy\"\nchanges = \"One line.\"\n\n[[changed]]\nlibrary = \"copy\"\nchanges = \"Again.\"\n";
        // Where the form is refused, the message is the one of the reader of
        // the file and is not compared: nothing is expected but the refusal.
        let table = [
            ("preference = [\"MIT\"]", "preference = \"MIT\"", None),
            ("preference = [\"MIT\"]\n", "", None),
            ("targets = [\"one\", \"two\"]\n", "", None),
            ("standard-text", "standard_text", None),
            ("version = \"2.0.0\"", "edition = \"2.0.0\"", None),
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
                "both = [\"one\", \"two\"]",
                "both = []",
                Some(Invalid::NoTarget),
            ),
            (
                "targets = [\"one\", \"two\"]",
                "targets = [\" \"]",
                Some(Invalid::Empty("target")),
            ),
            (
                "both = [\"one\", \"two\"]",
                "both = [\"one\", \"\"]",
                Some(Invalid::Empty("target")),
            ),
            (libraries, "[\"odd\", \"odd\"]", twice("odd", None)),
            (libraries, "[\"bare\"]", twice("bare", None)),
            (
                libraries,
                "[{ name = \"split\", version = \"2.0.0\" }]",
                twice("split", Some("2.0.0")),
            ),
            (excepted, "[\"bare\", \"bare\"]", twice("bare", None)),
            (
                excepted,
                "[\"\"]",
                Some(Invalid::Empty("name of a library")),
            ),
            (
                "version = \"2.0.0\"",
                "version = \" \"",
                Some(Invalid::Empty("version of a library")),
            ),
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
            (
                changed,
                changed_twice,
                Some(Invalid::ChangedTwice(selector("copy", None))),
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
    fn the_changes_of_one_version_may_be_stated_beside_those_of_every_version()
    -> Result<(), Invalid> {
        let changed = "library = \"copy\"\nchanges = \"One line.\"\n";
        let one_more = "library = \"copy\"\nchanges = \"One line.\"\n\n[[changed]]\nlibrary = \"copy\"\nversion = \"2.0.0\"\nchanges = \"Another.\"\n";
        let read = Settings::from_texts(POLICY, &settings(changed, one_more))?;

        assert_eq!(
            read.changes("copy", "2.0.0").map(|(_, changes)| changes),
            Some("Another.")
        );
        assert_eq!(
            read.changes("copy", "1.0.0").map(|(_, changes)| changes),
            Some("One line.")
        );
        Ok(())
    }

    #[test]
    fn a_policy_without_allowed_licences_is_refused() {
        let result = Settings::from_texts("[licenses]\n", &settings("", ""));

        assert!(matches!(result, Err(Invalid::Form { .. })));
    }
}
