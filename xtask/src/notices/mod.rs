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
mod files;
mod format;
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
use self::files::{Document, document};
use self::format::{Library, LibraryNotices, LicenseText};
use self::location::Location;
use self::rule::{Settled, Unsettled, settle};
use self::settings::{Clarification, Invalid, POLICY_FILE, SETTINGS_FILE, Selector, Settings};

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
    /// The names of the regular files that directory holds itself.
    pub files: Vec<String>,
    /// The licence texts its package holds.
    pub own: Vec<LicenseText>,
    /// The texts that stand in for texts its package does not hold.
    pub substituted: Vec<LicenseText>,
    /// Whether a text that applies to it is not held by its package.
    pub lacks_own_text: bool,
    /// Whether it is a copy kept in this repository, not the published one.
    pub changed_copy: bool,
    /// What was found wrong while it was read.
    pub problems: Vec<Reason>,
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
    /// The settings name a licence the library does not offer.
    #[error(
        "{SETTINGS_FILE} states the licence {license} for it, which is not one it offers ({stated})"
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
    /// Its package holds a notice the list has no place for yet.
    #[error("its package holds the notice file `{0}`, which the list does not carry yet")]
    NoticeFile(String),
    /// Its package cannot be read.
    #[error("its package cannot be read: {0}")]
    PackageUnreadable(String),
    /// Its licence is not one a library may have.
    #[error("its licence {0} is not one {POLICY_FILE} allows")]
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

/// The licences a stated licence consists of, each once, in the order they
/// are stated, or nothing where it is not an SPDX expression.
///
/// | Stated                              | Licences                          |
/// |-------------------------------------|-----------------------------------|
/// | `MIT`                               | `MIT`                             |
/// | `MIT OR Apache-2.0`                 | `MIT`, `Apache-2.0`               |
/// | `Apache-2.0 AND MIT`                | `Apache-2.0`, `MIT`               |
/// | `(MIT OR Apache-2.0) AND MIT`       | `MIT`, `Apache-2.0`               |
/// | `MIT/Apache-2.0`, as older crates do | `MIT`, `Apache-2.0`              |
/// | `Apache-2.0 WITH LLVM-exception`    | `Apache-2.0 WITH LLVM-exception`  |
/// | `GPL-3.0-or-later`                  | `GPL-3.0-or-later`                |
/// | `SEE LICENSE IN LICENSE`            | nothing                           |
/// | `MIT OR`                            | nothing                           |
/// | empty                               | nothing                           |
fn licenses_offered(stated: &str) -> Option<Vec<String>> {
    let expression = spdx::Expression::parse_mode(stated, spdx::ParseMode::LAX).ok()?;
    let mut licenses = Vec::new();
    for requirement in expression.requirements() {
        let license = requirement.req.to_string();
        if !licenses.contains(&license) {
            licenses.push(license);
        }
    }
    Some(licenses)
}

/// The notice file among the regular files of a package, if it holds one.
fn notice_file(files: &[String]) -> Option<&String> {
    files
        .iter()
        .find(|name| document(name) == Some(Document::Notice))
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

/// The text the settings name for a library, read from its package.
///
/// | The named file                        | The named licence             | Outcome             |
/// |---------------------------------------|-------------------------------|---------------------|
/// | a regular file with a text            | one the library offers        | the text            |
/// | a regular file with a text            | the library states no SPDX expression | the text    |
/// | a regular file with a text            | not one the library offers    | refused: licence    |
/// | a directory, or not there             | any                           | refused: file       |
/// | empty, or not a text                  | any                           | refused: file       |
fn clarified_text(
    candidate: &Candidate,
    offered: Option<&[String]>,
    clarification: &Clarification,
) -> Result<LicenseText, Reason> {
    if offered.is_some_and(|offered| !offered.contains(&clarification.license)) {
        return Err(Reason::ClarifiedLicense {
            license: clarification.license.clone(),
            stated: candidate.license.clone(),
        });
    }
    let refused = |message: String| Reason::ClarifiedFile {
        file: clarification.file.clone(),
        message,
    };
    let path = candidate.directory.join(&clarification.file);
    if !std::fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
        return Err(refused("it is not a file of the package".to_owned()));
    }
    let text = std::fs::read_to_string(&path).map_err(|error| refused(error.to_string()))?;
    if text.trim().is_empty() {
        return Err(refused("it is empty".to_owned()));
    }
    Ok(LicenseText {
        license: clarification.license.clone(),
        text,
    })
}

/// The licence the list shows for a library: the one it states, or, where
/// that is no SPDX expression, the licences of its texts.
fn shown_license(stated: &str, offered: Option<&[String]>, texts: &[LicenseText]) -> String {
    if offered.is_some() {
        return stated.to_owned();
    }
    let mut licenses: Vec<&str> = Vec::new();
    for text in texts {
        if !licenses.contains(&text.license.as_str()) {
            licenses.push(&text.license);
        }
    }
    licenses.join(ALL_APPLY)
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

/// A library as the list names it, before its texts have their places.
struct SettledLibrary {
    license: String,
    texts: Vec<LicenseText>,
    changes: Option<String>,
}

/// One library as the list names it, or every reason it is not settled.
/// Names in `used` every setting that applies to it.
fn settled(
    candidate: &Candidate,
    settings: &Settings,
    used: &mut BTreeSet<Selector>,
) -> Result<SettledLibrary, Vec<Reason>> {
    let mut reasons = candidate.problems.clone();
    let mut texts = candidate.own.clone();
    if let Some(file) = notice_file(&candidate.files) {
        reasons.push(Reason::NoticeFile(file.clone()));
    }

    let offered = licenses_offered(&candidate.license);
    let clarification = settings.clarification(&candidate.name, &candidate.version);
    let standard_text = settings.standard_text(&candidate.name, &candidate.version);
    used.extend(clarification.map(|(selector, _)| selector.clone()));
    used.extend(standard_text.cloned());
    match settle(
        candidate.lacks_own_text,
        clarification.map(|(_, clarification)| clarification),
        standard_text.is_some(),
    ) {
        Ok(Settled::OwnText) => {}
        Ok(Settled::Clarified(clarification)) => {
            match clarified_text(candidate, offered.as_deref(), clarification) {
                Ok(text) => texts.push(text),
                Err(reason) => reasons.push(reason),
            }
        }
        Ok(Settled::StandardText) if candidate.substituted.is_empty() => {
            reasons.push(Reason::NoStandardText);
        }
        Ok(Settled::StandardText) => texts.extend(candidate.substituted.iter().cloned()),
        Err(unsettled) => reasons.push(unsettled.into()),
    }

    let stated = settings.changes(&candidate.name, &candidate.version);
    used.extend(stated.map(|(selector, _)| selector.clone()));
    let changes = match stated_changes(candidate.changed_copy, stated.map(|(_, changes)| changes)) {
        Ok(changes) => changes.map(str::to_owned),
        Err(reason) => {
            reasons.push(reason);
            None
        }
    };

    if reasons.is_empty() {
        Ok(SettledLibrary {
            license: shown_license(&candidate.license, offered.as_deref(), &texts),
            texts,
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
    used: &mut BTreeSet<Selector>,
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
pub fn run(selection: &Selection) -> Result<(), Error> {
    let workspace = Workspace::read()?;
    let location = Location::find(&workspace.root).map_err(Error::Location)?;
    let settings = Settings::from_texts(
        &read_text(&workspace.root.join(POLICY_FILE))?,
        &read_text(&workspace.root.join(SETTINGS_FILE))?,
    )?;
    let interface = packages::read(
        &workspace.root,
        location.bundled_packages(),
        settings.accepted(),
    )?;
    workspace.fetch_sources()?;

    let targets = match selection {
        Selection::One(target) => settings.built_for(target),
        Selection::All => settings.targets(),
    };
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
            "{target}: {} libraries, {} licence texts",
            notices.libraries.len(),
            notices.texts.len()
        );
    }

    if *selection == Selection::All {
        let unused: Vec<String> = settings
            .selectors()
            .into_iter()
            .filter(|selector| !used.contains(*selector))
            .map(Selector::to_string)
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
    use super::settings::{Clarification, Invalid, Selector, Settings};
    use super::{
        Candidate, Reason, Texts, assemble, clarified_text, licenses_offered, notice_file,
        shown_license, stated_changes,
    };
    use crate::notices::rule::Unsettled;

    const POLICY: &str = "[licenses]\nallow = [\"Apache-2.0\", \"MIT\"]\n";
    const SETTINGS: &str = "\
preference = [\"MIT\"]
targets = [\"one\"]
standard-text = [\"bare\", { name = \"split\", version = \"2.0.0\" }]

[[clarify]]
libraries = [\"named\"]
license = \"MIT\"
file = \"Cargo.toml\"

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

    /// A library whose package is the directory of this crate.
    fn candidate(name: &str, own: Vec<LicenseText>, substituted: Vec<LicenseText>) -> Candidate {
        Candidate {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            license: "MIT".to_owned(),
            directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            files: vec!["Cargo.toml".to_owned(), "build.rs".to_owned()],
            lacks_own_text: own.is_empty() || !substituted.is_empty(),
            own,
            substituted,
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

    #[test]
    fn the_licences_offered_follow_their_table() {
        let table: [(&str, Option<&[&str]>); 10] = [
            ("MIT", Some(&["MIT"])),
            ("MIT OR Apache-2.0", Some(&["MIT", "Apache-2.0"])),
            ("Apache-2.0 AND MIT", Some(&["Apache-2.0", "MIT"])),
            ("(MIT OR Apache-2.0) AND MIT", Some(&["MIT", "Apache-2.0"])),
            ("MIT/Apache-2.0", Some(&["MIT", "Apache-2.0"])),
            (
                "Apache-2.0 WITH LLVM-exception",
                Some(&["Apache-2.0 WITH LLVM-exception"]),
            ),
            ("GPL-3.0-or-later", Some(&["GPL-3.0-or-later"])),
            ("SEE LICENSE IN LICENSE", None),
            ("MIT OR", None),
            ("", None),
        ];
        for (stated, expected) in table {
            let offered = licenses_offered(stated);

            assert_eq!(
                offered
                    .as_ref()
                    .map(|licenses| licenses.iter().map(String::as_str).collect::<Vec<_>>())
                    .as_deref(),
                expected,
                "{stated}"
            );
        }
    }

    #[test]
    fn a_notice_file_is_found_among_the_files() {
        let files =
            |names: &[&str]| -> Vec<String> { names.iter().map(|&name| name.to_owned()).collect() };

        assert_eq!(notice_file(&files(&["LICENSE", "README.md"])), None);
        assert_eq!(
            notice_file(&files(&["LICENSE", "NOTICE"])).map(String::as_str),
            Some("NOTICE")
        );
        assert_eq!(notice_file(&files(&["notice_board.rs"])), None);
        assert_eq!(notice_file(&[]), None);
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
        let library = candidate("named", Vec::new(), Vec::new());
        let offered = ["MIT".to_owned(), "Apache-2.0".to_owned()];
        let clarification = |license: &str, file: &str| Clarification {
            license: license.to_owned(),
            file: file.to_owned(),
        };
        let read = |offered: Option<&[String]>, license, file| {
            clarified_text(&library, offered, &clarification(license, file))
        };

        assert!(
            read(Some(&offered), "MIT", "Cargo.toml")
                .is_ok_and(|text| text.license == "MIT" && text.text.contains("xtask"))
        );
        assert!(read(None, "MIT", "Cargo.toml").is_ok());
        assert_eq!(
            read(Some(&offered), "Zlib", "Cargo.toml"),
            Err(Reason::ClarifiedLicense {
                license: "Zlib".to_owned(),
                stated: "MIT".to_owned(),
            })
        );
        for file in ["src", "no-such-file", "src/no-such-file"] {
            assert!(
                matches!(
                    read(Some(&offered), "MIT", file),
                    Err(Reason::ClarifiedFile { .. })
                ),
                "{file}"
            );
        }
    }

    #[test]
    fn the_shown_licence_is_the_stated_one_where_it_can_be_read() {
        let texts = [
            text("MIT", "one"),
            text("Zlib", "two"),
            text("MIT", "three"),
        ];
        let offered = ["MIT".to_owned()];

        assert_eq!(
            shown_license("MIT OR Zlib", Some(&offered), &texts),
            "MIT OR Zlib"
        );
        assert_eq!(
            shown_license("SEE LICENSE IN LICENSE", None, &texts),
            "MIT AND Zlib"
        );
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
        let mut split = candidate("split", Vec::new(), vec![text("MIT", "standard")]);
        split.version = "2.0.0".to_owned();
        let candidates = vec![
            candidate("Zeta", vec![text("MIT", "shared")], Vec::new()),
            candidate("bare", Vec::new(), vec![text("MIT", "standard")]),
            copy,
            split,
            candidate("split", vec![text("MIT", "own")], Vec::new()),
            candidate("named", Vec::new(), vec![text("MIT", "standard")]),
            candidate("alpha", vec![text("MIT", "own")], Vec::new()),
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
                ("copy", "1.0.0", [2].as_slice(), Some("One line.")),
                // The file the settings name, not the text that stood in.
                ("named", "1.0.0", [3].as_slice(), None),
                // The version that holds its text needs no setting, the
                // other one has its own.
                ("split", "1.0.0", [0].as_slice(), None),
                ("split", "2.0.0", [1].as_slice(), None),
                ("Zeta", "1.0.0", [2].as_slice(), None),
            ]
        );
        assert_eq!(notices.texts.len(), 4);
        assert_eq!(
            notices.texts[..3],
            [
                text("MIT", "own"),
                text("MIT", "standard"),
                text("MIT", "shared")
            ]
        );
        assert_eq!(
            used.iter().map(Selector::to_string).collect::<Vec<_>>(),
            ["bare", "copy", "named", "split 2.0.0"]
        );
        Ok(())
    }

    #[test]
    fn every_library_that_is_not_settled_is_named_with_its_reason() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut unstated = candidate("unstated", vec![text("MIT", "own")], Vec::new());
        unstated.changed_copy = true;
        let mut noticed = candidate("noticed", vec![text("MIT", "own")], Vec::new());
        noticed.files.push("NOTICE".to_owned());
        let mut unreadable = candidate("unreadable", vec![text("MIT", "own")], Vec::new());
        unreadable
            .problems
            .push(Reason::PackageUnreadable("no access".to_owned()));
        let candidates = vec![
            candidate("missing", Vec::new(), vec![text("MIT", "standard")]),
            candidate("nothing", Vec::new(), Vec::new()),
            candidate("bare", vec![text("MIT", "own")], Vec::new()),
            candidate("copy", vec![text("MIT", "own")], Vec::new()),
            unstated,
            noticed,
            unreadable,
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
                (
                    "noticed 1.0.0".to_owned(),
                    Reason::NoticeFile("NOTICE".to_owned())
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
            refusals(vec![candidate("bare", Vec::new(), Vec::new())], &settings),
            Err(vec![("bare 1.0.0".to_owned(), Reason::NoStandardText)])
        );
        Ok(())
    }

    #[test]
    fn a_clarified_licence_has_to_be_one_the_library_offers() -> Result<(), Invalid> {
        let settings = Settings::from_texts(POLICY, SETTINGS)?;
        let mut named = candidate("named", Vec::new(), Vec::new());
        named.license = "Apache-2.0".to_owned();

        assert_eq!(
            refusals(vec![named], &settings),
            Err(vec![(
                "named 1.0.0".to_owned(),
                Reason::ClarifiedLicense {
                    license: "MIT".to_owned(),
                    stated: "Apache-2.0".to_owned(),
                }
            )])
        );
        Ok(())
    }
}
