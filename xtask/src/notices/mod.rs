// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generates the list of the libraries a package of the application contains,
//! with the licence texts they ask to be passed on.
//!
//! The Rust libraries are read by `cargo about`, the libraries of the
//! interface from the list its build writes. Every library has to bring its
//! licence text in its own package. One that does not stops the generator,
//! unless the settings name the file that holds the text or let the standard
//! text of the licence stand for it. Nothing is left out silently, and no
//! setting stays that is not needed any more.

mod cargo;
mod crates;
mod format;
mod packages;
mod rule;
mod settings;

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};

use thiserror::Error;

use self::cargo::Workspace;
use self::format::{Library, LibraryNotices, LicenseText};
use self::packages::BUNDLED_PACKAGES_FILE;
use self::rule::{Settled, Unsettled, settle};
use self::settings::{Invalid, POLICY_FILE, SETTINGS_FILE, Settings};

/// The variable that names the directory the lists are written to. Cargo sets
/// it from its configuration in this repository.
const NOTICES_DIRECTORY_VARIABLE: &str = "NOTICES_DIR";
/// What the name of a notice file starts with, in lower case.
const NOTICE_FILE_START: &str = "notice";

/// Which targets the list is generated for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// One target.
    One(String),
    /// Every target packages are built for. Only then is it known whether a
    /// setting applies to no library at all.
    All,
}

/// A library a package contains, with what was found about its licence text.
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
    /// The licence texts found in its package.
    pub own: Vec<LicenseText>,
    /// The standard texts put in the place of texts not found there.
    pub standard: Vec<LicenseText>,
    /// Whether a text that applies to it was not found in its package.
    pub lacks_own_text: bool,
    /// Whether it is a copy kept in this repository, not the published one.
    pub changed_copy: bool,
}

/// Why a library keeps the list from being generated.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Reason {
    /// Its licence text is not settled.
    #[error(transparent)]
    Unsettled(#[from] Unsettled),
    /// The file the settings name does not give its text.
    #[error("the file `{file}` that {SETTINGS_FILE} names cannot be read: {message}")]
    ClarifiedFile {
        /// The file.
        file: String,
        /// Why it cannot be read.
        message: String,
    },
    /// The standard text is allowed, and there is none.
    #[error("{SETTINGS_FILE} lets the standard text stand for it, and none is at hand")]
    NoStandardText,
    /// Its package holds a notice the list has no place for yet.
    #[error("its package holds the notice file `{0}`, which the list does not carry yet")]
    NoticeFile(String),
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
    /// The generator was not started by Cargo in this repository.
    #[error("the variable {0} is not set; run the tool as `cargo xtask`")]
    NotRunByCargo(&'static str),
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
    #[error("{SETTINGS_FILE} names libraries no package contains: {}", .0.join(", "))]
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

/// The names of the files and directories a directory holds, in order.
fn file_names(directory: &Path) -> Result<Vec<String>, Error> {
    let io = |source| Error::Io {
        path: directory.to_path_buf(),
        source,
    };
    let mut names = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(io)? {
        names.push(
            entry
                .map_err(io)?
                .file_name()
                .to_string_lossy()
                .into_owned(),
        );
    }
    names.sort();
    Ok(names)
}

/// The notice file among the files of a package, if it holds one.
///
/// | Files                         | Notice file |
/// |-------------------------------|-------------|
/// | `LICENSE`, `README.md`        | none        |
/// | `NOTICE`                      | `NOTICE`    |
/// | `notice.txt`, `LICENSE`       | `notice.txt`|
/// | `NOTICE`, `NOTICE.md`         | `NOTICE`    |
/// | none                          | none        |
fn notice_file(names: &[String]) -> Option<&String> {
    names
        .iter()
        .find(|name| name.to_lowercase().starts_with(NOTICE_FILE_START))
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

/// The licence texts of the list, each held once.
#[derive(Debug, Default)]
struct Texts {
    texts: Vec<LicenseText>,
    places: HashMap<LicenseText, usize>,
}

impl Texts {
    /// The place of a text, which is added if it is not held yet.
    fn place(&mut self, text: LicenseText) -> usize {
        if let Some(place) = self.places.get(&text) {
            return *place;
        }
        let place = self.texts.len();
        self.texts.push(text.clone());
        self.places.insert(text, place);
        place
    }
}

/// The licence texts of one library and what was changed in it, or every
/// reason it is not settled. Names the library in `used` when a setting
/// applies to it.
fn settled(
    candidate: &Candidate,
    settings: &Settings,
    used: &mut BTreeSet<String>,
) -> Result<(Vec<LicenseText>, Option<String>), Vec<Reason>> {
    let mut reasons = Vec::new();
    let mut texts = candidate.own.clone();

    match file_names(&candidate.directory) {
        Ok(names) => {
            if let Some(file) = notice_file(&names) {
                reasons.push(Reason::NoticeFile(file.clone()));
            }
        }
        Err(error) => reasons.push(Reason::ClarifiedFile {
            file: candidate.directory.display().to_string(),
            message: error.to_string(),
        }),
    }

    let clarification = settings.clarification(&candidate.name);
    let standard_text_allowed = settings.standard_text_allowed(&candidate.name);
    if clarification.is_some() || standard_text_allowed {
        used.insert(candidate.name.clone());
    }
    match settle(
        candidate.lacks_own_text,
        clarification,
        standard_text_allowed,
    ) {
        Ok(Settled::OwnText) => {}
        Ok(Settled::Clarified(clarification)) => {
            match std::fs::read_to_string(candidate.directory.join(&clarification.file)) {
                Ok(text) if !text.trim().is_empty() => texts.push(LicenseText {
                    license: clarification.license.clone(),
                    text,
                }),
                Ok(_) => reasons.push(Reason::ClarifiedFile {
                    file: clarification.file.clone(),
                    message: "it is empty".to_owned(),
                }),
                Err(error) => reasons.push(Reason::ClarifiedFile {
                    file: clarification.file.clone(),
                    message: error.to_string(),
                }),
            }
        }
        Ok(Settled::StandardText) if candidate.standard.is_empty() => {
            reasons.push(Reason::NoStandardText);
        }
        Ok(Settled::StandardText) => texts.extend(candidate.standard.iter().cloned()),
        Err(unsettled) => reasons.push(unsettled.into()),
    }

    let stated = settings.changes(&candidate.name);
    if stated.is_some() {
        used.insert(candidate.name.clone());
    }
    let changes = match stated_changes(candidate.changed_copy, stated) {
        Ok(changes) => changes.map(str::to_owned),
        Err(reason) => {
            reasons.push(reason);
            None
        }
    };

    if reasons.is_empty() {
        Ok((texts, changes))
    } else {
        Err(reasons)
    }
}

/// The list of the given libraries, by name, or every library that is not
/// settled with every reason.
fn assemble(
    mut candidates: Vec<Candidate>,
    settings: &Settings,
    used: &mut BTreeSet<String>,
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
            Ok((own, changes)) => libraries.push(Library {
                texts: own.into_iter().map(|text| texts.place(text)).collect(),
                name: candidate.name,
                version: candidate.version,
                license: candidate.license,
                changes,
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
pub fn run(selection: &Selection) -> Result<(), Error> {
    let directory = std::env::var_os(NOTICES_DIRECTORY_VARIABLE)
        .map(PathBuf::from)
        .ok_or(Error::NotRunByCargo(NOTICES_DIRECTORY_VARIABLE))?;
    let workspace = Workspace::read()?;
    let settings = Settings::from_texts(
        &read_text(&workspace.root.join(POLICY_FILE))?,
        &read_text(&workspace.root.join(SETTINGS_FILE))?,
    )?;
    let interface = packages::read(&workspace.root, &directory.join(BUNDLED_PACKAGES_FILE))?;
    workspace.fetch_sources()?;

    let targets = match selection {
        Selection::One(target) => std::slice::from_ref(target),
        Selection::All => settings.targets(),
    };
    let mut used = BTreeSet::new();
    for target in targets {
        let mut candidates = crates::read(&workspace, &settings, target)?;
        candidates.extend(interface.iter().cloned());
        let notices = assemble(candidates, &settings, &mut used).map_err(Error::Refused)?;
        let file = directory.join(format!("{target}.json"));
        let written = serde_json::to_string(&notices).map_err(|error| Error::Unreadable {
            path: file.clone(),
            message: error.to_string(),
        })?;
        write_text(&file, &written)?;
        println!(
            "{target}: {} libraries, {} licence texts",
            notices.libraries.len(),
            notices.texts.len()
        );
    }

    if *selection == Selection::All {
        let unused: Vec<String> = settings
            .libraries()
            .into_iter()
            .filter(|library| !used.contains(*library))
            .map(str::to_owned)
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
    use std::path::PathBuf;

    use super::format::LicenseText;
    use super::settings::{Invalid, Settings};
    use super::{Candidate, Reason, Texts, assemble, notice_file, stated_changes};
    use crate::notices::rule::Unsettled;

    const POLICY: &str = "[licenses]\nallow = [\"Apache-2.0\", \"MIT\"]\n";
    const SETTINGS: &str = "\
preference = [\"MIT\"]
targets = [\"one\"]
standard-text = [\"bare\"]
clarify = []

[[changed]]
library = \"copy\"
changes = \"One line.\"
";

    fn text(license: &str, text: &str) -> LicenseText {
        LicenseText {
            license: license.to_owned(),
            text: text.to_owned(),
        }
    }

    /// A library whose package is the directory of this crate, which holds
    /// no notice file.
    fn candidate(name: &str, own: Vec<LicenseText>, standard: Vec<LicenseText>) -> Candidate {
        Candidate {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            license: "MIT".to_owned(),
            directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            lacks_own_text: own.is_empty() || !standard.is_empty(),
            own,
            standard,
            changed_copy: false,
        }
    }

    #[test]
    fn a_notice_file_follows_its_table() {
        let table: [(&[&str], Option<&str>); 5] = [
            (&["LICENSE", "README.md"], None),
            (&["NOTICE"], Some("NOTICE")),
            (&["notice.txt", "LICENSE"], Some("notice.txt")),
            (&["NOTICE", "NOTICE.md"], Some("NOTICE")),
            (&[], None),
        ];
        for (names, expected) in table {
            let names: Vec<String> = names.iter().map(|&name| name.to_owned()).collect();

            assert_eq!(
                notice_file(&names).map(String::as_str),
                expected,
                "{names:?}"
            );
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
    fn a_text_is_held_once() {
        let mut texts = Texts::default();

        let first = texts.place(text("MIT", "one"));
        let second = texts.place(text("MIT", "two"));
        let again = texts.place(text("MIT", "one"));
        let other_licence = texts.place(text("Apache-2.0", "one"));

        assert_eq!((first, second, again, other_licence), (0, 1, 0, 2));
        assert_eq!(texts.texts.len(), 3);
    }

    #[test]
    fn the_list_names_libraries_in_order_and_shares_their_texts() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut used = BTreeSet::new();
        let mut copy = candidate("copy", vec![text("MIT", "shared")], Vec::new());
        copy.changed_copy = true;
        let candidates = vec![
            candidate("Zeta", vec![text("MIT", "shared")], Vec::new()),
            candidate("bare", Vec::new(), vec![text("MIT", "standard")]),
            copy,
            candidate("alpha", vec![text("MIT", "own")], Vec::new()),
        ];

        let notices = assemble(candidates, &settings, &mut used);

        let Ok(notices) = notices else {
            unreachable!("refused: {notices:?}")
        };
        let named: Vec<(&str, &[usize], Option<&str>)> = notices
            .libraries
            .iter()
            .map(|library| {
                (
                    library.name.as_str(),
                    library.texts.as_slice(),
                    library.changes.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            named,
            [
                ("alpha", [0].as_slice(), None),
                ("bare", [1].as_slice(), None),
                ("copy", [2].as_slice(), Some("One line.")),
                ("Zeta", [2].as_slice(), None),
            ]
        );
        assert_eq!(
            notices.texts,
            [
                text("MIT", "own"),
                text("MIT", "standard"),
                text("MIT", "shared")
            ]
        );
        assert_eq!(used.into_iter().collect::<Vec<_>>(), ["bare", "copy"]);
        Ok(())
    }

    #[test]
    fn every_library_that_is_not_settled_is_named_with_its_reason() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut unstated = candidate("unstated", vec![text("MIT", "own")], Vec::new());
        unstated.changed_copy = true;
        let candidates = vec![
            candidate("missing", Vec::new(), vec![text("MIT", "standard")]),
            candidate("nothing", Vec::new(), Vec::new()),
            candidate("bare", vec![text("MIT", "own")], Vec::new()),
            candidate("copy", vec![text("MIT", "own")], Vec::new()),
            unstated,
            candidate("fine", vec![text("MIT", "own")], Vec::new()),
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
                    "missing 1.0.0".to_owned(),
                    Reason::Unsettled(Unsettled::NoText)
                ),
                (
                    "nothing 1.0.0".to_owned(),
                    Reason::Unsettled(Unsettled::NoText)
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
        let candidates = vec![candidate("bare", Vec::new(), Vec::new())];

        let refused = assemble(candidates, &settings, &mut BTreeSet::new());

        assert_eq!(
            refused.map_err(|refusals| refusals.0),
            Err(vec![("bare 1.0.0".to_owned(), Reason::NoStandardText)])
        );
        Ok(())
    }
}
