// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where the lists of the contained libraries lie.
//!
//! The place is stated once, in the manifest of the interface, because the
//! build of the interface has to read it as well and can read nothing else
//! of this workspace. This file is part of two programs: the generator, and
//! the build script of the application, which includes it by its path. It
//! uses nothing but the standard library and the JSON reader both have.

use std::path::{Component, Path, PathBuf};

/// The manifest of the interface, in the root of the repository.
const MANIFEST_FILE: &str = "package.json";
/// The part of the manifest that holds the settings of the repository.
const SETTINGS_KEY: &str = "config";
/// The directory the lists are generated in, relative to the root.
const DIRECTORY_KEY: &str = "noticesDirectory";
/// The list of the packages in the bundle of the interface, in that
/// directory.
const BUNDLED_PACKAGES_KEY: &str = "bundledPackagesList";
/// What the name of the list of a target ends with.
const LIST_EXTENSION: &str = "json";

/// Whether a path stays below the directory it is relative to.
///
/// | Path                     | Stays below |
/// |--------------------------|-------------|
/// | `LICENSE`                | yes         |
/// | `docs/LICENSE`           | yes         |
/// | empty                    | no          |
/// | `../LICENSE`             | no          |
/// | `docs/../../LICENSE`     | no          |
/// | `/etc/passwd`            | no          |
/// | `./LICENSE`              | no          |
pub fn stays_below(path: &str) -> bool {
    let mut components = Path::new(path).components().peekable();
    components.peek().is_some()
        && components.all(|component| matches!(component, Component::Normal(_)))
}

/// Where the lists lie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    manifest: PathBuf,
    directory: PathBuf,
    bundled_packages: PathBuf,
}

impl Location {
    /// Reads the place from the manifest of the interface, which is looked
    /// for in the given directory and then in each one above it.
    ///
    /// | Manifest                                             | Outcome        |
    /// |------------------------------------------------------|----------------|
    /// | found, states both values as paths that stay below   | the place      |
    /// | found in a directory above                           | the place      |
    /// | not found up to the root of the file system          | refused        |
    /// | not JSON                                             | refused        |
    /// | lacks a value, or holds one that is no text          | refused        |
    /// | holds a value that leaves the repository             | refused        |
    pub fn find(start: &Path) -> Result<Self, String> {
        let manifest = start
            .ancestors()
            .map(|directory| directory.join(MANIFEST_FILE))
            .find(|manifest| manifest.is_file())
            .ok_or_else(|| format!("no {MANIFEST_FILE} in {} or above", start.display()))?;
        let text = std::fs::read_to_string(&manifest)
            .map_err(|error| format!("{}: {error}", manifest.display()))?;
        let root = manifest.parent().unwrap_or(start).to_path_buf();
        Self::from_manifest(&text, &root, manifest)
    }

    fn from_manifest(text: &str, root: &Path, manifest: PathBuf) -> Result<Self, String> {
        let read: serde_json::Value = serde_json::from_str(text)
            .map_err(|error| format!("{}: {error}", manifest.display()))?;
        let value = |key: &str| {
            read.get(SETTINGS_KEY)
                .and_then(|settings| settings.get(key))
                .and_then(serde_json::Value::as_str)
                .filter(|path| stays_below(path))
                .ok_or_else(|| {
                    format!(
                        "{}: `{SETTINGS_KEY}.{key}` has to be a path below the repository",
                        manifest.display()
                    )
                })
        };
        let directory = root.join(value(DIRECTORY_KEY)?);
        let bundled_packages = directory.join(value(BUNDLED_PACKAGES_KEY)?);
        Ok(Self {
            manifest,
            directory,
            bundled_packages,
        })
    }

    /// The manifest the place was read from.
    pub fn manifest(&self) -> &Path {
        &self.manifest
    }

    /// The list of the libraries a package for the given target contains.
    pub fn list(&self, target: &str) -> PathBuf {
        self.directory.join(format!("{target}.{LIST_EXTENSION}"))
    }

    /// The list of the packages in the bundle of the interface, which its
    /// build writes.
    pub fn bundled_packages(&self) -> &Path {
        &self.bundled_packages
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Location, stays_below};

    #[test]
    fn a_path_that_stays_below_follows_its_table() {
        let table = [
            ("LICENSE", true),
            ("docs/LICENSE", true),
            ("", false),
            ("../LICENSE", false),
            ("docs/../../LICENSE", false),
            ("/etc/passwd", false),
            ("./LICENSE", false),
        ];
        for (path, expected) in table {
            assert_eq!(stays_below(path), expected, "{path}");
        }
    }

    fn read(text: &str) -> Result<Location, String> {
        Location::from_manifest(text, Path::new("root"), PathBuf::from("root/package.json"))
    }

    #[test]
    fn a_manifest_that_states_both_values_gives_the_place() -> Result<(), String> {
        let location = read(
            r#"{"config": {"noticesDirectory": "gen/lists", "bundledPackagesList": "p.json"}}"#,
        )?;

        assert_eq!(location.manifest(), Path::new("root/package.json"));
        assert_eq!(
            location.list("target"),
            Path::new("root/gen/lists/target.json")
        );
        assert_eq!(
            location.bundled_packages(),
            Path::new("root/gen/lists/p.json")
        );
        Ok(())
    }

    #[test]
    fn a_manifest_that_does_not_state_the_place_is_refused() {
        let table = [
            "not JSON",
            "{}",
            r#"{"config": {"noticesDirectory": "gen"}}"#,
            r#"{"config": {"bundledPackagesList": "p.json"}}"#,
            r#"{"config": {"noticesDirectory": 1, "bundledPackagesList": "p.json"}}"#,
            r#"{"config": {"noticesDirectory": "../gen", "bundledPackagesList": "p.json"}}"#,
            r#"{"config": {"noticesDirectory": "gen", "bundledPackagesList": "/p.json"}}"#,
            r#"{"config": {"noticesDirectory": "", "bundledPackagesList": "p.json"}}"#,
        ];
        for text in table {
            assert!(read(text).is_err(), "{text}");
        }
    }

    #[test]
    fn the_manifest_is_found_in_a_directory_above() -> Result<(), String> {
        // The tests run in the directory of this crate, one below the root.
        let location = Location::find(Path::new(env!("CARGO_MANIFEST_DIR")))?;

        assert!(location.manifest().is_file());
        assert!(
            location.list("target").starts_with(
                location
                    .manifest()
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
            )
        );
        Ok(())
    }

    #[test]
    fn a_directory_without_a_manifest_above_is_refused() {
        // A relative path has nothing above it but the empty path.
        assert!(Location::find(Path::new("no-such-directory-of-the-build-tools")).is_err());
    }
}
