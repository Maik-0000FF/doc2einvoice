// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the generator is told: the licences the policy of the dependencies
//! allows, and the settings of the list itself.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;
use thiserror::Error;

use super::location::{is_one_name, stays_below};

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
    no_notice: Vec<NoNoticeEntry>,
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
struct NoNoticeEntry {
    libraries: Vec<Named>,
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

/// The file of a package that holds the text of a licence, and the licence
/// it states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clarification {
    /// SPDX identifier of the licence.
    pub license: String,
    /// The file, relative to the directory of the package.
    pub file: String,
}

/// How the settings settle the licence texts a library does not bring
/// without help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settling {
    /// The files of its package that hold them, one for each licence named.
    Clarified(Vec<Clarification>),
    /// The standard texts of its licences stand for them.
    StandardText,
}

/// One setting about a library, as the generator keeps track of which were
/// needed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Setting {
    /// How its licence texts are settled.
    Settling(Selector),
    /// That a file of its package is passed on with no library.
    NoNotice(Selector, String),
    /// What was changed in it.
    Changes(Selector),
}

impl fmt::Display for Setting {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Settling(selector) => write!(formatter, "the licence text of {selector}"),
            Self::NoNotice(selector, file) => write!(formatter, "`{file}` of {selector}"),
            Self::Changes(selector) => write!(formatter, "the changes of {selector}"),
        }
    }
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
    /// A target is not one name.
    #[error("{SETTINGS_FILE} names the target `{0}`, which is not the name of a target")]
    Target(String),
    /// A library is settled more than once.
    #[error("{SETTINGS_FILE} settles the library {0} more than once")]
    SettledTwice(Selector),
    /// A change is stated more than once for a library.
    #[error("{SETTINGS_FILE} states the changes of the library {0} more than once")]
    ChangedTwice(Selector),
    /// A file is named twice as no notice of a library.
    #[error("{SETTINGS_FILE} names `{1}` of the library {0} more than once")]
    NamedTwice(Selector, String),
    /// A named file does not lie where the setting asks.
    #[error("{SETTINGS_FILE} names `{0}`, which is not a file {1}")]
    File(String, &'static str),
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
    settlings: BTreeMap<Selector, Settling>,
    no_notices: BTreeMap<Selector, BTreeSet<String>>,
    changes: BTreeMap<Selector, String>,
}

/// The entry of a map that applies to a version of a library. Each kind of
/// setting is held in one map, so that the entry for one version comes
/// before the one for every version whatever the two say.
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
    /// | Input                                                       | Outcome                |
    /// |-------------------------------------------------------------|------------------------|
    /// | both well formed, every library settled once                | accepted               |
    /// | the lists of libraries left out                             | accepted, as empty     |
    /// | a library settled for every version and, in any way, for one | accepted              |
    /// | a library clarified for two different licences              | accepted               |
    /// | a file that is not TOML, or misnames a key                  | refused: form          |
    /// | the preference or the targets left out                      | refused: form          |
    /// | a preferred or clarified licence the policy does not allow  | refused: not allowed   |
    /// | no target, or a combined target that stands for none        | refused: no target     |
    /// | a target that is not one name                               | refused: target        |
    /// | a library clarified twice for one licence                   | refused: settled twice |
    /// | a library clarified and given the standard text, or given it twice | refused: settled twice |
    /// | the changes of a library stated twice                       | refused: changed twice |
    /// | a file named twice as no notice of a library                | refused: named twice   |
    /// | a clarified file outside the package                        | refused: file          |
    /// | a file named as no notice that is not in the root           | refused: file          |
    /// | an empty name, version or statement of changes              | refused: empty         |
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
        if let Some(target) = targets.into_iter().find(|target| !is_one_name(target)) {
            return Err(Invalid::Target(target.clone()));
        }

        Ok(Self {
            accepted,
            targets: file.targets,
            combined: file.combined,
            settlings: Self::read_settlings(file.standard_text, file.clarify)?,
            no_notices: Self::read_no_notices(file.no_notice)?,
            changes: Self::read_changes(file.changed)?,
        })
    }

    fn read_settlings(
        standard_text: Vec<Named>,
        clarify: Vec<ClarifyEntry>,
    ) -> Result<BTreeMap<Selector, Settling>, Invalid> {
        let mut settlings = BTreeMap::new();
        for library in standard_text {
            let selector = Selector::try_from(library)?;
            if settlings.contains_key(&selector) {
                return Err(Invalid::SettledTwice(selector));
            }
            settlings.insert(selector, Settling::StandardText);
        }
        for entry in clarify {
            if !stays_below(&entry.file) {
                return Err(Invalid::File(entry.file, "inside a package"));
            }
            for library in entry.libraries {
                let selector = Selector::try_from(library)?;
                let clarification = Clarification {
                    license: entry.license.clone(),
                    file: entry.file.clone(),
                };
                match settlings
                    .entry(selector.clone())
                    .or_insert_with(|| Settling::Clarified(Vec::new()))
                {
                    Settling::Clarified(clarifications)
                        if clarifications
                            .iter()
                            .all(|other| other.license != clarification.license) =>
                    {
                        clarifications.push(clarification);
                    }
                    _ => return Err(Invalid::SettledTwice(selector)),
                }
            }
        }
        Ok(settlings)
    }

    fn read_no_notices(
        entries: Vec<NoNoticeEntry>,
    ) -> Result<BTreeMap<Selector, BTreeSet<String>>, Invalid> {
        let mut no_notices: BTreeMap<Selector, BTreeSet<String>> = BTreeMap::new();
        for entry in entries {
            if !is_one_name(&entry.file) {
                return Err(Invalid::File(entry.file, "in the root of a package"));
            }
            for library in entry.libraries {
                let selector = Selector::try_from(library)?;
                if !no_notices
                    .entry(selector.clone())
                    .or_default()
                    .insert(entry.file.clone())
                {
                    return Err(Invalid::NamedTwice(selector, entry.file));
                }
            }
        }
        Ok(no_notices)
    }

    fn read_changes(entries: Vec<ChangedEntry>) -> Result<BTreeMap<Selector, String>, Invalid> {
        let mut changes = BTreeMap::new();
        for entry in entries {
            let selector = Selector::new(entry.library, entry.version)?;
            if entry.changes.trim().is_empty() {
                return Err(Invalid::Empty("statement of changes"));
            }
            if changes.contains_key(&selector) {
                return Err(Invalid::ChangedTwice(selector));
            }
            changes.insert(selector, entry.changes);
        }
        Ok(changes)
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

    /// How the licence texts of a library are settled, if the settings say,
    /// with the setting that says it.
    pub fn settling(&self, name: &str, version: &str) -> Option<(Setting, &Settling)> {
        applying(&self.settlings, name, version)
            .map(|(selector, settling)| (Setting::Settling(selector.clone()), settling))
    }

    /// The files in the root of the package of a library that are passed on
    /// with no library, each with the setting that names it.
    pub fn no_notices(&self, name: &str, version: &str) -> Vec<(Setting, &str)> {
        applying(&self.no_notices, name, version)
            .into_iter()
            .flat_map(|(selector, files)| {
                files.iter().map(|file| {
                    (
                        Setting::NoNotice(selector.clone(), file.clone()),
                        file.as_str(),
                    )
                })
            })
            .collect()
    }

    /// What was changed in the copy of a library, if it is stated, with the
    /// setting that states it.
    pub fn changes(&self, name: &str, version: &str) -> Option<(Setting, &str)> {
        applying(&self.changes, name, version)
            .map(|(selector, changes)| (Setting::Changes(selector.clone()), changes.as_str()))
    }

    /// Every setting about a library.
    pub fn settings(&self) -> BTreeSet<Setting> {
        let settlings = self.settlings.keys().cloned().map(Setting::Settling);
        let no_notices = self.no_notices.iter().flat_map(|(selector, files)| {
            files
                .iter()
                .map(|file| Setting::NoNotice(selector.clone(), file.clone()))
        });
        let changes = self.changes.keys().cloned().map(Setting::Changes);
        settlings.chain(no_notices).chain(changes).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Clarification, Invalid, Selector, Setting, Settings, Settling, applying};

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

[[clarify]]
libraries = [\"odd\"]
license = \"MIT\"
file = \"LICENSE-MIT\"

[[no-notice]]
libraries = [\"odd\"]
file = \"Notice.svelte\"

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

    fn clarification(license: &str, file: &str) -> Clarification {
        Clarification {
            license: license.to_owned(),
            file: file.to_owned(),
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
        let zlib = clarification("Zlib", "docs/LICENSE");
        let settling = |name, version| {
            read.settling(name, version)
                .map(|(setting, settling)| (setting.to_string(), settling.clone()))
        };

        assert_eq!(read.accepted(), ["MIT", "Apache-2.0", "Zlib"]);
        assert_eq!(read.targets(), ["one", "two"]);
        assert_eq!(read.built_for(&"both".to_owned()), ["one", "two"]);
        assert_eq!(read.built_for(&"other".to_owned()), ["other"]);
        // The setting for one version comes before the one for every
        // version, though the two settle the library in different ways.
        assert_eq!(
            settling("bare", "1.0.0"),
            Some((
                "the licence text of bare 1.0.0".to_owned(),
                Settling::Clarified(vec![zlib.clone()])
            ))
        );
        assert_eq!(
            settling("bare", "2.0.0"),
            Some((
                "the licence text of bare".to_owned(),
                Settling::StandardText
            ))
        );
        assert_eq!(
            settling("split", "2.0.0"),
            Some((
                "the licence text of split 2.0.0".to_owned(),
                Settling::StandardText
            ))
        );
        assert_eq!(settling("split", "1.0.0"), None);
        // A library may be clarified once for each of its licences.
        assert_eq!(
            settling("odd", "3.1.0"),
            Some((
                "the licence text of odd".to_owned(),
                Settling::Clarified(vec![zlib, clarification("MIT", "LICENSE-MIT")])
            ))
        );
        assert_eq!(
            read.no_notices("odd", "3.1.0")
                .into_iter()
                .map(|(setting, file)| (setting.to_string(), file))
                .collect::<Vec<_>>(),
            [("`Notice.svelte` of odd".to_owned(), "Notice.svelte")]
        );
        assert!(read.no_notices("bare", "1.0.0").is_empty());
        assert_eq!(
            read.changes("copy", "0.1.0")
                .map(|(setting, changes)| (setting.to_string(), changes)),
            Some(("the changes of copy".to_owned(), "One line."))
        );
        assert_eq!(read.changes("odd", "1.0.0"), None);
        assert_eq!(
            read.settings()
                .iter()
                .map(Setting::to_string)
                .collect::<Vec<_>>(),
            [
                "the licence text of bare",
                "the licence text of bare 1.0.0",
                "the licence text of odd",
                "the licence text of odder",
                "the licence text of split 2.0.0",
                "`Notice.svelte` of odd",
                "the changes of copy",
            ]
        );
        Ok(())
    }

    #[test]
    fn the_lists_of_libraries_may_be_left_out() -> Result<(), Invalid> {
        let read = Settings::from_texts(POLICY, "preference = []\ntargets = [\"one\"]\n")?;

        assert_eq!(read.accepted(), ["Apache-2.0", "MIT", "Zlib"]);
        assert!(read.settings().is_empty());
        assert_eq!(read.built_for(&"one".to_owned()), ["one"]);
        Ok(())
    }

    /// Replaces one part of the accepted settings by another and expects the
    /// refusal named. Where the form is refused, the message is the one of
    /// the reader of the file and is not compared: nothing is expected but
    /// the refusal.
    fn assert_refused(table: &[(&str, &str, Option<Invalid>)]) {
        for (replace, with, expected) in table {
            let result = Settings::from_texts(POLICY, &settings(replace, with));

            match expected {
                Some(refusal) => assert_eq!(result.as_ref(), Err(refusal), "{replace} -> {with}"),
                None => assert!(
                    matches!(result, Err(Invalid::Form { .. })),
                    "{replace} -> {with}: {result:?}"
                ),
            }
        }
    }

    #[test]
    fn a_form_a_licence_or_a_target_that_is_wrong_is_refused() {
        let not_allowed = || Some(Invalid::LicenseNotAllowed("0BSD".to_owned()));
        let target = |name: &str| Some(Invalid::Target(name.to_owned()));
        assert_refused(&[
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
                target(" "),
            ),
            (
                "targets = [\"one\", \"two\"]",
                "targets = [\"../one\"]",
                target("../one"),
            ),
            (
                "both = [\"one\", \"two\"]",
                "both = [\"one\", \"\"]",
                target(""),
            ),
            (
                "both = [\"one\", \"two\"]",
                "\"a/b\" = [\"one\"]",
                target("a/b"),
            ),
        ]);
    }

    #[test]
    fn a_library_that_is_settled_wrongly_is_refused() {
        let twice = |name, version| Some(Invalid::SettledTwice(selector(name, version)));
        let libraries = "[\"odd\", \"odder\", { name = \"bare\", version = \"1.0.0\" }]";
        let excepted = "[\"bare\", { name = \"split\", version = \"2.0.0\" }]";
        let changed = "library = \"copy\"\nchanges = \"One line.\"\n";
        let changed_twice = "library = \"copy\"\nchanges = \"One line.\"\n\n[[changed]]\nlibrary = \"copy\"\nchanges = \"Again.\"\n";
        let no_notice = "libraries = [\"odd\"]\nfile = \"Notice.svelte\"";
        let no_notice_twice = "libraries = [\"odd\", \"odd\"]\nfile = \"Notice.svelte\"";
        assert_refused(&[
            (libraries, "[\"bare\"]", twice("bare", None)),
            (
                libraries,
                "[{ name = \"split\", version = \"2.0.0\" }]",
                twice("split", Some("2.0.0")),
            ),
            // The same licence a second time, for a library the next entry
            // clarifies for another.
            (
                "license = \"MIT\"",
                "license = \"Zlib\"",
                twice("odd", None),
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
                Some(Invalid::File("../LICENSE".to_owned(), "inside a package")),
            ),
            (
                "file = \"Notice.svelte\"",
                "file = \"src/Notice.svelte\"",
                Some(Invalid::File(
                    "src/Notice.svelte".to_owned(),
                    "in the root of a package",
                )),
            ),
            (
                no_notice,
                no_notice_twice,
                Some(Invalid::NamedTwice(
                    selector("odd", None),
                    "Notice.svelte".to_owned(),
                )),
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
        ]);
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
