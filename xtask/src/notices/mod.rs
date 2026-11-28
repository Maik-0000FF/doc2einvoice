// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generates the list of the libraries a package of the application contains,
//! with the texts they ask to be passed on.
//!
//! The Rust libraries are read by `cargo about`, the libraries of the
//! interface from the list its build writes. Two things are asked of every
//! library, and nothing is left out silently:
//!
//! - Every file in the root of its package whose name says licence, notice
//!   or copyright is passed on as it is, unless the settings name it as none
//!   of these.
//! - Its package has to hold the texts of the licences it states. Where it
//!   does not, the generator stops, unless the settings name the file that
//!   holds a text or let the standard text of the licence stand for it.
//!
//! A setting that is not needed any more stops the generator as well.

mod cargo;
mod crates;
mod files;
mod format;
mod license;
// The build script of the application includes this module by its path.
#[allow(
    dead_code,
    reason = "the build script of the application uses the rest of what this module offers"
)]
mod location;
mod packages;
mod rule;
mod settings;

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};

use thiserror::Error;

use self::cargo::Workspace;
use self::files::names_document;
use self::format::{Library, LibraryNotices};
use self::license::Stated;
use self::location::{Location, is_one_name};
use self::rule::{Settled, Unsettled, settle};
use self::settings::{
    Clarification, Invalid, POLICY_FILE, SETTINGS_FILE, Setting, Settings, Settling,
};

/// What joins the licences of a library that all apply to it.
const ALL_APPLY: &str = " AND ";

/// Which targets the list is generated for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The targets of one package.
    One(String),
    /// Every target packages are built for. Only then is it known whether a
    /// setting applies to no library at all.
    All,
}

/// The standard text of a licence, which stands in for a text the package of
/// a library does not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardText {
    /// SPDX identifier of the licence.
    pub license: String,
    /// The text.
    pub text: String,
}

/// A library a package contains, with what was found in its package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// The name its author publishes it under.
    pub name: String,
    /// The version contained.
    pub version: String,
    /// The licence its author states.
    pub license: String,
    /// The directory of its package.
    pub directory: PathBuf,
    /// The names of the regular files that directory holds itself.
    pub files: Vec<String>,
    /// The licences whose texts its package holds.
    pub licenses_held: Vec<String>,
    /// Those texts, as far as a tool found them; they may lie below the root
    /// of the package.
    pub texts_held: Vec<String>,
    /// The standard texts of the licences whose texts its package does not
    /// hold.
    pub standard_texts: Vec<StandardText>,
    /// Whether it is a copy kept in this repository, not the published one.
    pub changed_copy: bool,
    /// What was found wrong while it was read.
    pub problems: Vec<Reason>,
}

/// Why a library keeps the list from being generated.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Reason {
    /// Its licence texts are not settled.
    #[error(transparent)]
    Unsettled(#[from] Unsettled),
    /// What the settings say does not give all the texts it states.
    #[error(
        "what {SETTINGS_FILE} says for it does not give the texts of the licences it states ({0})"
    )]
    StillLacking(String),
    /// The file the settings name does not give its text.
    #[error("the file `{file}` that {SETTINGS_FILE} names cannot be read: {message}")]
    ClarifiedFile {
        /// The file.
        file: String,
        /// Why it cannot be read.
        message: String,
    },
    /// The settings name a licence the library does not state.
    #[error(
        "{SETTINGS_FILE} states the licence {license} for it, which is not one it states ({stated})"
    )]
    ClarifiedLicense {
        /// The licence the settings state.
        license: String,
        /// The licence the library states.
        stated: String,
    },
    /// The standard text is allowed, and there is none.
    #[error("{SETTINGS_FILE} lets the standard text stand for it, and none is at hand")]
    NoStandardText,
    /// A file that is passed on with it cannot be read as a text.
    #[error(
        "the file `{file}` of its package says licence, notice or copyright and cannot be read as a text ({message}); if it is none of these, name it under `no-notice` in {SETTINGS_FILE}"
    )]
    Document {
        /// The file.
        file: String,
        /// Why it cannot be read.
        message: String,
    },
    /// The settings name a file as no notice that its package does not hold.
    #[error(
        "{SETTINGS_FILE} names `{0}` as no notice, and its package holds no such file that would be passed on"
    )]
    NoNoticeNotNeeded(String),
    /// Its package cannot be read.
    #[error("its package cannot be read: {0}")]
    PackageUnreadable(String),
    /// Its licence cannot be fulfilled by the licences a library may have.
    #[error("the licence it states ({0}) is not one {POLICY_FILE} allows")]
    LicenseNotAllowed(String),
    /// It is a changed copy and the change is not stated.
    #[error("it is a changed copy, and {SETTINGS_FILE} does not state what was changed")]
    UnstatedChange,
    /// A change is stated for the published library.
    #[error(
        "it is the published library, so its statement of changes in {SETTINGS_FILE} is not needed"
    )]
    ChangeNotNeeded,
}

/// The libraries that keep the list from being generated, each with its
/// reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusals(Vec<(String, Reason)>);

impl fmt::Display for Refusals {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "The list of libraries is not generated:")?;
        for (library, reason) in &self.0 {
            write!(formatter, "\n  {library}: {reason}")?;
        }
        Ok(())
    }
}

/// Why the generator ends without a list.
#[derive(Debug, Error)]
pub enum Error {
    /// A file or a directory cannot be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file or the directory.
        path: PathBuf,
        /// What the operating system says.
        source: std::io::Error,
    },
    /// A file is not what its form asks for.
    #[error("{}: {message}", path.display())]
    Unreadable {
        /// The file.
        path: PathBuf,
        /// What its reader says.
        message: String,
    },
    /// A command the generator runs did not succeed.
    #[error("{command} failed: {detail}")]
    Command {
        /// The command.
        command: String,
        /// What it wrote as its error.
        detail: String,
    },
    /// Where the lists lie is not stated.
    #[error("{0}")]
    Location(String),
    /// What is asked for as a target is not the name of one.
    #[error("`{0}` is not the name of a target")]
    Target(String),
    /// The list of the packages of the interface is missing.
    #[error(
        "{} is missing; build the interface first (`npm run build`)",
        .0.display()
    )]
    InterfaceNotBuilt(PathBuf),
    /// The policy or the settings are not accepted.
    #[error(transparent)]
    Settings(#[from] Invalid),
    /// Libraries are not settled.
    #[error("{0}")]
    Refused(Refusals),
    /// Settings apply to no library of any target.
    #[error("{SETTINGS_FILE} holds settings no package needs: {}", .0.join(", "))]
    Unused(Vec<String>),
}

/// The text of a file.
fn read_text(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Writes a file, and the directories above it where they are missing.
fn write_text(path: &Path, text: &str) -> Result<(), Error> {
    let io = |source| Error::Io {
        path: path.to_path_buf(),
        source,
    };
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory).map_err(io)?;
    }
    std::fs::write(path, text).map_err(io)
}

/// The text of a file that is passed on, which has to say something.
///
/// | The file                               | Outcome          |
/// |----------------------------------------|------------------|
/// | a regular file that holds a text       | the text         |
/// | a directory, or not there              | refused          |
/// | not a text, as an image is             | refused          |
/// | empty, or nothing but blank lines      | refused          |
fn passed_on_text(path: &Path) -> Result<String, String> {
    if !std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
        return Err("it is not a file of the package".to_owned());
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    if text.trim().is_empty() {
        return Err("it is empty".to_owned());
    }
    Ok(text)
}

/// What the list states as changed in a library.
///
/// | A changed copy | Changes stated | Outcome                 |
/// |----------------|----------------|-------------------------|
/// | yes            | yes            | the statement           |
/// | yes            | no             | refused: unstated       |
/// | no             | yes            | refused: not needed     |
/// | no             | no             | nothing                 |
fn stated_changes(changed_copy: bool, stated: Option<&str>) -> Result<Option<&str>, Reason> {
    match (changed_copy, stated) {
        (true, Some(changes)) => Ok(Some(changes)),
        (true, None) => Err(Reason::UnstatedChange),
        (false, Some(_)) => Err(Reason::ChangeNotNeeded),
        (false, None) => Ok(None),
    }
}

/// The text the settings name for a licence of a library, read from its
/// package.
///
/// | The named licence                         | The named file                | Outcome          |
/// |-------------------------------------------|-------------------------------|------------------|
/// | one the library states                    | a file that holds a text      | the text         |
/// | any, and the library states no expression | a file that holds a text      | the text         |
/// | not one the library states                | any                           | refused: licence |
/// | one the library states                    | not a file, no text, or empty | refused: file    |
fn clarified_text(
    candidate: &Candidate,
    stated: &Stated,
    clarification: &Clarification,
) -> Result<String, Reason> {
    if stated.is_expression() && !stated.licenses().contains(&clarification.license) {
        return Err(Reason::ClarifiedLicense {
            license: clarification.license.clone(),
            stated: candidate.license.clone(),
        });
    }
    passed_on_text(&candidate.directory.join(&clarification.file)).map_err(|message| {
        Reason::ClarifiedFile {
            file: clarification.file.clone(),
            message,
        }
    })
}

/// The texts of one library, each once, in the order they were added.
#[derive(Debug, Default)]
struct OwnTexts(Vec<String>);

impl OwnTexts {
    fn add(&mut self, text: String) {
        if !self.0.contains(&text) {
            self.0.push(text);
        }
    }
}

/// The texts of the list, each held once.
#[derive(Debug, Default)]
struct Texts {
    texts: Vec<String>,
    places: HashMap<String, usize>,
}

impl Texts {
    /// The place of a text, which is added if it is not held yet.
    fn place(&mut self, text: String) -> usize {
        if let Some(place) = self.places.get(&text) {
            return *place;
        }
        let place = self.texts.len();
        self.texts.push(text.clone());
        self.places.insert(text, place);
        place
    }
}

/// A library as the list names it, before its texts have their places.
struct SettledLibrary {
    license: String,
    texts: Vec<String>,
    changes: Option<String>,
}

/// The files in the root of the package of a library that are passed on with
/// it, read into `texts`. A file the settings name as no notice is left out;
/// a setting that names a file the package would not pass on is refused.
fn pass_on_documents(
    candidate: &Candidate,
    settings: &Settings,
    used: &mut BTreeSet<Setting>,
    texts: &mut OwnTexts,
    reasons: &mut Vec<Reason>,
) {
    let no_notices = settings.no_notices(&candidate.name, &candidate.version);
    let documents: Vec<&String> = candidate
        .files
        .iter()
        .filter(|file| names_document(file))
        .collect();
    for (setting, file) in &no_notices {
        if documents.iter().any(|document| document.as_str() == *file) {
            used.insert(setting.clone());
        } else {
            reasons.push(Reason::NoNoticeNotNeeded((*file).to_owned()));
        }
    }
    for file in documents {
        if no_notices.iter().any(|(_, named)| *named == file.as_str()) {
            continue;
        }
        match passed_on_text(&candidate.directory.join(file)) {
            Ok(text) => texts.add(text),
            Err(message) => reasons.push(Reason::Document {
                file: file.clone(),
                message,
            }),
        }
    }
}

/// Settles the licence texts of a library: adds to `texts` what the settings
/// give, and says which licences have their text at hand then.
fn settle_license_texts(
    candidate: &Candidate,
    stated: &Stated,
    settings: &Settings,
    used: &mut BTreeSet<Setting>,
    texts: &mut OwnTexts,
    reasons: &mut Vec<Reason>,
) -> Vec<String> {
    let accepted = settings.accepted();
    let mut at_hand = candidate.licenses_held.clone();
    let settling = settings.settling(&candidate.name, &candidate.version);
    if let Some((setting, _)) = &settling {
        used.insert(setting.clone());
    }
    let before = reasons.len();
    match settle(
        stated.fulfilled_by(&at_hand, accepted),
        settling.map(|(_, settling)| settling),
    ) {
        Ok(Settled::OwnTexts) => return at_hand,
        Ok(Settled::Settings(Settling::Clarified(clarifications))) => {
            for clarification in clarifications {
                match clarified_text(candidate, stated, clarification) {
                    Ok(text) => {
                        texts.add(text);
                        at_hand.push(clarification.license.clone());
                    }
                    Err(reason) => reasons.push(reason),
                }
            }
        }
        Ok(Settled::Settings(Settling::StandardText)) => {
            if candidate.standard_texts.is_empty() {
                reasons.push(Reason::NoStandardText);
            }
            for standard in &candidate.standard_texts {
                texts.add(standard.text.clone());
                at_hand.push(standard.license.clone());
            }
        }
        Err(unsettled) => reasons.push(unsettled.into()),
    }
    // What the settings give has to be enough: a library that asks for two
    // licences is not settled by the text of one.
    if reasons.len() == before && !stated.fulfilled_by(&at_hand, accepted) {
        reasons.push(Reason::StillLacking(candidate.license.clone()));
    }
    at_hand
}

/// The licence the list shows for a library: the one it states, or, where
/// that is no SPDX expression, the licences whose texts are at hand.
fn shown_license(candidate: &Candidate, stated: &Stated, at_hand: &[String]) -> String {
    if stated.is_expression() {
        return candidate.license.clone();
    }
    let mut licenses: Vec<&str> = Vec::new();
    for license in at_hand {
        if !licenses.contains(&license.as_str()) {
            licenses.push(license);
        }
    }
    licenses.join(ALL_APPLY)
}

/// One library as the list names it, or every reason it is not settled.
/// Names in `used` every setting that was needed for it.
fn settled(
    candidate: &Candidate,
    settings: &Settings,
    used: &mut BTreeSet<Setting>,
) -> Result<SettledLibrary, Vec<Reason>> {
    let mut reasons = candidate.problems.clone();
    let mut texts = OwnTexts::default();
    let stated = Stated::read(&candidate.license);
    if !stated.can_be_allowed(settings.accepted()) {
        reasons.push(Reason::LicenseNotAllowed(candidate.license.clone()));
    }

    pass_on_documents(candidate, settings, used, &mut texts, &mut reasons);
    for text in &candidate.texts_held {
        texts.add(text.clone());
    }
    let at_hand =
        settle_license_texts(candidate, &stated, settings, used, &mut texts, &mut reasons);

    let stated_change = settings.changes(&candidate.name, &candidate.version);
    if let Some((setting, _)) = &stated_change {
        used.insert(setting.clone());
    }
    let changes = match stated_changes(
        candidate.changed_copy,
        stated_change.map(|(_, changes)| changes),
    ) {
        Ok(changes) => changes.map(str::to_owned),
        Err(reason) => {
            reasons.push(reason);
            None
        }
    };

    if reasons.is_empty() {
        Ok(SettledLibrary {
            license: shown_license(candidate, &stated, &at_hand),
            texts: texts.0,
            changes,
        })
    } else {
        Err(reasons)
    }
}

/// The list of the given libraries, by name, or every library that is not
/// settled with every reason.
fn assemble(
    mut candidates: Vec<Candidate>,
    settings: &Settings,
    used: &mut BTreeSet<Setting>,
) -> Result<LibraryNotices, Refusals> {
    candidates.sort_by_cached_key(|candidate| {
        (
            candidate.name.to_lowercase(),
            candidate.name.clone(),
            candidate.version.clone(),
        )
    });
    let mut texts = Texts::default();
    let mut libraries = Vec::new();
    let mut refusals = Vec::new();
    for candidate in candidates {
        match settled(&candidate, settings, used) {
            Ok(library) => libraries.push(Library {
                texts: library
                    .texts
                    .into_iter()
                    .map(|text| texts.place(text))
                    .collect(),
                name: candidate.name,
                version: candidate.version,
                license: library.license,
                changes: library.changes,
            }),
            Err(reasons) => refusals.extend(
                reasons
                    .into_iter()
                    .map(|reason| (format!("{} {}", candidate.name, candidate.version), reason)),
            ),
        }
    }
    if refusals.is_empty() {
        Ok(LibraryNotices {
            texts: texts.texts,
            libraries,
        })
    } else {
        Err(Refusals(refusals))
    }
}

/// Generates the list for the selected targets and writes one file per
/// target.
///
/// | What is asked for                          | Lists generated for            |
/// |--------------------------------------------|--------------------------------|
/// | every target                               | each target of the settings    |
/// | one target                                 | that target                    |
/// | a target the settings combine from others  | each of those                  |
/// | a target that is not one name, as `../x`   | none: refused                  |
pub fn run(selection: &Selection) -> Result<(), Error> {
    let workspace = Workspace::read()?;
    let location = Location::find(&workspace.root).map_err(Error::Location)?;
    let settings = Settings::from_texts(
        &read_text(&workspace.root.join(POLICY_FILE))?,
        &read_text(&workspace.root.join(SETTINGS_FILE))?,
    )?;
    let targets = match selection {
        Selection::One(target) => settings.built_for(target),
        Selection::All => settings.targets(),
    };
    // A target names the files written for it.
    if let Some(target) = targets.iter().find(|target| !is_one_name(target)) {
        return Err(Error::Target(target.clone()));
    }
    let interface = packages::read(&workspace.root, location.bundled_packages())?;
    workspace.fetch_sources()?;

    let mut used = BTreeSet::new();
    for target in targets {
        let mut candidates = crates::read(&workspace, &settings, target)?;
        candidates.extend(interface.iter().cloned());
        let notices = assemble(candidates, &settings, &mut used).map_err(Error::Refused)?;
        let file = location.list(target);
        let written = serde_json::to_string(&notices).map_err(|error| Error::Unreadable {
            path: file.clone(),
            message: error.to_string(),
        })?;
        write_text(&file, &written)?;
        println!(
            "{target}: {} libraries, {} texts",
            notices.libraries.len(),
            notices.texts.len()
        );
    }

    if *selection == Selection::All {
        let unused: Vec<String> = settings
            .settings()
            .iter()
            .filter(|setting| !used.contains(*setting))
            .map(Setting::to_string)
            .collect();
        if !unused.is_empty() {
            return Err(Error::Unused(unused));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use super::license::Stated;
    use super::settings::{Clarification, Invalid, Setting, Settings};
    use super::{
        Candidate, Reason, StandardText, Texts, assemble, clarified_text, passed_on_text,
        stated_changes,
    };
    use crate::notices::rule::Unsettled;

    const POLICY: &str = "[licenses]\nallow = [\"Apache-2.0\", \"MIT\", \"Zlib\"]\n";
    const SETTINGS: &str = "\
preference = [\"MIT\"]
targets = [\"one\"]
standard-text = [\"bare\", { name = \"split\", version = \"2.0.0\" }]

[[clarify]]
libraries = [\"named\", \"two\", { name = \"bare\", version = \"3.0.0\" }]
license = \"MIT\"
file = \"Cargo.toml\"

[[clarify]]
libraries = [\"two\"]
license = \"Zlib\"
file = \"src/main.rs\"

[[no-notice]]
libraries = [\"plain\"]
file = \"copyright.rs\"

[[changed]]
library = \"copy\"
changes = \"One line.\"
";

    /// The directory of this crate, which stands for the package of every
    /// library of these tests. Its manifest and its code are the texts the
    /// settings name.
    const PACKAGE: &str = env!("CARGO_MANIFEST_DIR");

    fn file(name: &str) -> String {
        std::fs::read_to_string(Path::new(PACKAGE).join(name)).unwrap_or_default()
    }

    /// A library that states MIT, in whose package no file is passed on.
    fn candidate(name: &str, held: &[&str], standard: &[&str]) -> Candidate {
        Candidate {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            license: "MIT".to_owned(),
            directory: PathBuf::from(PACKAGE),
            files: vec!["Cargo.toml".to_owned()],
            licenses_held: held.iter().map(|&license| license.to_owned()).collect(),
            texts_held: held
                .iter()
                .map(|license| format!("the text of {license}"))
                .collect(),
            standard_texts: standard
                .iter()
                .map(|&license| StandardText {
                    license: license.to_owned(),
                    text: format!("the standard text of {license}"),
                })
                .collect(),
            changed_copy: false,
            problems: Vec::new(),
        }
    }

    fn refusals(
        candidates: Vec<Candidate>,
        settings: &Settings,
    ) -> Result<(), Vec<(String, Reason)>> {
        assemble(candidates, settings, &mut BTreeSet::new())
            .map(drop)
            .map_err(|refusals| refusals.0)
    }

    fn refused(name: &str, reason: Reason) -> Result<(), Vec<(String, Reason)>> {
        Err(vec![(format!("{name} 1.0.0"), reason)])
    }

    #[test]
    fn a_passed_on_text_follows_its_table() {
        let package = Path::new(PACKAGE);

        assert!(
            passed_on_text(&package.join("Cargo.toml")).is_ok_and(|text| text.contains("xtask"))
        );
        for path in ["src", "no-such-file", "src/no-such-file"] {
            assert!(passed_on_text(&package.join(path)).is_err(), "{path}");
        }
    }

    #[test]
    fn stated_changes_follow_their_table() {
        let table = [
            (true, Some("changed"), Ok(Some("changed"))),
            (true, None, Err(Reason::UnstatedChange)),
            (false, Some("changed"), Err(Reason::ChangeNotNeeded)),
            (false, None, Ok(None)),
        ];
        for (changed_copy, stated, expected) in table {
            assert_eq!(stated_changes(changed_copy, stated), expected);
        }
    }

    #[test]
    fn the_clarified_text_follows_its_table() {
        let mut library = candidate("named", &[], &[]);
        library.license = "MIT OR Apache-2.0".to_owned();
        let clarification = |license: &str, file: &str| Clarification {
            license: license.to_owned(),
            file: file.to_owned(),
        };
        let read = |stated: &str, license, file| {
            clarified_text(
                &library,
                &Stated::read(stated),
                &clarification(license, file),
            )
        };

        assert_eq!(
            read("MIT OR Apache-2.0", "MIT", "Cargo.toml"),
            Ok(file("Cargo.toml"))
        );
        assert_eq!(
            read("SEE LICENSE IN LICENSE", "Zlib", "Cargo.toml"),
            Ok(file("Cargo.toml"))
        );
        assert_eq!(
            read("MIT OR Apache-2.0", "Zlib", "Cargo.toml"),
            Err(Reason::ClarifiedLicense {
                license: "Zlib".to_owned(),
                stated: "MIT OR Apache-2.0".to_owned(),
            })
        );
        for missing in ["src", "no-such-file"] {
            assert!(
                matches!(
                    read("MIT OR Apache-2.0", "MIT", missing),
                    Err(Reason::ClarifiedFile { .. })
                ),
                "{missing}"
            );
        }
    }

    #[test]
    fn a_text_is_held_once() {
        let mut texts = Texts::default();

        let first = texts.place("one".to_owned());
        let second = texts.place("two".to_owned());
        let again = texts.place("one".to_owned());

        assert_eq!((first, second, again), (0, 1, 0));
        assert_eq!(texts.texts, ["one", "two"]);
    }

    #[test]
    fn the_list_names_libraries_in_order_and_shares_their_texts() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut used = BTreeSet::new();
        let mut copy = candidate("copy", &["MIT"], &[]);
        copy.changed_copy = true;
        let mut split = candidate("split", &[], &["MIT"]);
        split.version = "2.0.0".to_owned();
        // The setting for this version clarifies the library, the one for
        // every version gives it the standard text: the closer one decides.
        let mut bare = candidate("bare", &[], &["MIT"]);
        bare.version = "3.0.0".to_owned();
        let candidates = vec![
            candidate("Zeta", &["MIT"], &[]),
            candidate("bare", &[], &["MIT"]),
            bare,
            copy,
            split,
            candidate("split", &["MIT"], &[]),
            candidate("named", &[], &["MIT"]),
            candidate("alpha", &["MIT"], &[]),
        ];

        let notices = assemble(candidates, &settings, &mut used);

        let Ok(notices) = notices else {
            unreachable!("refused: {notices:?}")
        };
        let named: Vec<(&str, &str, &[usize], Option<&str>)> = notices
            .libraries
            .iter()
            .map(|library| {
                (
                    library.name.as_str(),
                    library.version.as_str(),
                    library.texts.as_slice(),
                    library.changes.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            named,
            [
                ("alpha", "1.0.0", [0].as_slice(), None),
                ("bare", "1.0.0", [1].as_slice(), None),
                ("bare", "3.0.0", [2].as_slice(), None),
                ("copy", "1.0.0", [0].as_slice(), Some("One line.")),
                ("named", "1.0.0", [2].as_slice(), None),
                ("split", "1.0.0", [0].as_slice(), None),
                ("split", "2.0.0", [1].as_slice(), None),
                ("Zeta", "1.0.0", [0].as_slice(), None),
            ]
        );
        assert_eq!(
            notices.texts,
            [
                "the text of MIT".to_owned(),
                "the standard text of MIT".to_owned(),
                file("Cargo.toml"),
            ]
        );
        assert_eq!(
            used.iter().map(Setting::to_string).collect::<Vec<_>>(),
            [
                "the licence text of bare",
                "the licence text of bare 3.0.0",
                "the licence text of named",
                "the licence text of split 2.0.0",
                "the changes of copy",
            ]
        );
        Ok(())
    }

    #[test]
    fn a_file_whose_name_says_so_is_passed_on_unless_it_is_named_as_none() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut used = BTreeSet::new();
        let mut silent = candidate("silent", &["MIT"], &[]);
        silent.files = vec!["Cargo.toml".to_owned(), "README.md".to_owned()];
        let mut plain = candidate("plain", &["MIT"], &[]);
        plain.files = vec!["Cargo.toml".to_owned(), "copyright.rs".to_owned()];

        let notices = assemble(vec![silent, plain], &settings, &mut used);

        let Ok(notices) = notices else {
            unreachable!("refused: {notices:?}")
        };
        // Neither holds a file that is passed on: the names of the one say
        // nothing of the kind, and `copyright.rs` is named as no notice.
        assert_eq!(notices.texts, ["the text of MIT"]);
        assert_eq!(
            used.iter().map(Setting::to_string).collect::<Vec<_>>(),
            ["`copyright.rs` of plain"]
        );
        Ok(())
    }

    #[test]
    fn a_file_that_is_passed_on_is_read_from_the_package() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let package = Path::new(PACKAGE).join("src").join("notices");
        // `license.rs` is a file of this crate whose name says licence.
        let mut library = candidate("library", &["MIT"], &[]);
        library.directory.clone_from(&package);
        library.files = vec!["license.rs".to_owned(), "mod.rs".to_owned()];
        let mut unreadable = candidate("unreadable", &["MIT"], &[]);
        unreadable.directory = package;
        unreadable.files = vec!["no-such-copyright".to_owned()];

        let notices = assemble(vec![library], &settings, &mut BTreeSet::new());

        let Ok(notices) = notices else {
            unreachable!("refused: {notices:?}")
        };
        assert_eq!(
            notices.texts,
            [file("src/notices/license.rs"), "the text of MIT".to_owned()]
        );
        let refused = refusals(vec![unreadable], &settings);
        assert!(
            matches!(
                refused.as_ref().map_err(Vec::as_slice),
                Err([(library, Reason::Document { file, .. })])
                    if library == "unreadable 1.0.0" && file == "no-such-copyright"
            ),
            "{refused:?}"
        );
        Ok(())
    }

    #[test]
    fn every_library_that_is_not_settled_is_named_with_its_reason() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut unstated = candidate("unstated", &["MIT"], &[]);
        unstated.changed_copy = true;
        let mut unreadable = candidate("unreadable", &["MIT"], &[]);
        unreadable
            .problems
            .push(Reason::PackageUnreadable("no access".to_owned()));
        let mut forbidden = candidate("forbidden", &["0BSD"], &[]);
        forbidden.license = "0BSD".to_owned();
        let candidates = vec![
            candidate("missing", &[], &["MIT"]),
            candidate("nothing", &[], &[]),
            candidate("bare", &["MIT"], &[]),
            candidate("copy", &["MIT"], &[]),
            candidate("plain", &["MIT"], &[]),
            unstated,
            unreadable,
            forbidden,
            candidate("fine", &["MIT"], &[]),
        ];

        let refused = assemble(candidates, &settings, &mut BTreeSet::new());

        let Err(refusals) = refused else {
            unreachable!("generated")
        };
        assert_eq!(
            refusals.0,
            [
                (
                    "bare 1.0.0".to_owned(),
                    Reason::Unsettled(Unsettled::NotNeeded)
                ),
                ("copy 1.0.0".to_owned(), Reason::ChangeNotNeeded),
                (
                    "forbidden 1.0.0".to_owned(),
                    Reason::LicenseNotAllowed("0BSD".to_owned())
                ),
                (
                    "forbidden 1.0.0".to_owned(),
                    Reason::Unsettled(Unsettled::NoText)
                ),
                (
                    "missing 1.0.0".to_owned(),
                    Reason::Unsettled(Unsettled::NoText)
                ),
                (
                    "nothing 1.0.0".to_owned(),
                    Reason::Unsettled(Unsettled::NoText)
                ),
                (
                    "plain 1.0.0".to_owned(),
                    Reason::NoNoticeNotNeeded("copyright.rs".to_owned())
                ),
                (
                    "unreadable 1.0.0".to_owned(),
                    Reason::PackageUnreadable("no access".to_owned())
                ),
                ("unstated 1.0.0".to_owned(), Reason::UnstatedChange),
            ]
        );
        assert!(refusals.to_string().contains("missing 1.0.0: "));
        Ok(())
    }

    #[test]
    fn the_standard_text_cannot_stand_where_there_is_none() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;

        assert_eq!(
            refusals(vec![candidate("bare", &[], &[])], &settings),
            refused("bare", Reason::NoStandardText)
        );
        Ok(())
    }

    #[test]
    fn a_clarified_licence_has_to_be_one_the_library_states() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut named = candidate("named", &[], &[]);
        named.license = "Apache-2.0".to_owned();

        assert_eq!(
            refusals(vec![named], &settings),
            refused(
                "named",
                Reason::ClarifiedLicense {
                    license: "MIT".to_owned(),
                    stated: "Apache-2.0".to_owned(),
                }
            )
        );
        Ok(())
    }

    #[test]
    fn a_library_that_asks_for_two_licences_needs_the_texts_of_both() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let both = "MIT AND Zlib";
        // One clarification, for one of the two licences it asks for.
        let mut one = candidate("named", &[], &[]);
        one.license = both.to_owned();
        // One clarification for each.
        let mut two = candidate("two", &[], &[]);
        two.license = both.to_owned();
        // The package holds the text of one, the settings name the other.
        let mut half = candidate("named", &["Zlib"], &[]);
        half.license = both.to_owned();
        // The standard text of one of the two only.
        let mut bare = candidate("bare", &[], &["MIT"]);
        bare.license = both.to_owned();
        // A choice is settled by one.
        let mut choice = candidate("named", &[], &[]);
        choice.license = "MIT OR Zlib".to_owned();

        assert_eq!(
            refusals(vec![one], &settings),
            refused("named", Reason::StillLacking(both.to_owned()))
        );
        assert_eq!(refusals(vec![two], &settings), Ok(()));
        assert_eq!(refusals(vec![half], &settings), Ok(()));
        assert_eq!(
            refusals(vec![bare], &settings),
            refused("bare", Reason::StillLacking(both.to_owned()))
        );
        assert_eq!(refusals(vec![choice], &settings), Ok(()));
        Ok(())
    }

    #[test]
    fn a_licence_that_is_no_expression_is_shown_as_what_was_settled() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut named = candidate("named", &[], &[]);
        named.license = "SEE LICENSE IN LICENSE".to_owned();

        let notices = assemble(vec![named], &settings, &mut BTreeSet::new());

        assert_eq!(
            notices.map(|notices| notices.libraries[0].license.clone()),
            Ok("MIT".to_owned())
        );
        Ok(())
    }
}
