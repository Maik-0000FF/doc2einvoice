// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What Cargo is asked: where the workspace lies, which packages are its
//! own, and to fetch the sources of the dependencies.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::Error;

/// The variable Cargo names itself in for the programs it runs.
const CARGO_VARIABLE: &str = "CARGO";
/// How Cargo is called where that variable is not set.
const CARGO_PROGRAM: &str = "cargo";
/// The package whose dependencies a package of the application contains.
const APPLICATION_PACKAGE: &str = "doc2einvoice";

/// What Cargo says about the workspace, as far as it is read here.
#[derive(Debug, Deserialize)]
struct Metadata {
    workspace_root: PathBuf,
    target_directory: PathBuf,
    packages: Vec<Member>,
}

#[derive(Debug, Deserialize)]
struct Member {
    name: String,
    manifest_path: PathBuf,
}

/// The workspace the tools run in.
#[derive(Debug)]
pub struct Workspace {
    /// Its root directory.
    pub root: PathBuf,
    /// Where Cargo keeps what it builds.
    pub target_directory: PathBuf,
    /// The manifest of the application.
    pub application: PathBuf,
    /// The manifests of its own packages.
    members: BTreeSet<PathBuf>,
}

impl Workspace {
    /// Asks Cargo for the workspace of the current directory.
    pub fn read() -> Result<Self, Error> {
        let output =
            run(cargo().args(["metadata", "--no-deps", "--format-version", "1", "--locked"]))?;
        let metadata: Metadata =
            serde_json::from_slice(&output).map_err(|error| Error::Command {
                command: "cargo metadata".to_owned(),
                detail: error.to_string(),
            })?;
        let application = metadata
            .packages
            .iter()
            .find(|member| member.name == APPLICATION_PACKAGE)
            .map(|member| member.manifest_path.clone())
            .ok_or_else(|| Error::Command {
                command: "cargo metadata".to_owned(),
                detail: format!("the workspace has no package {APPLICATION_PACKAGE}"),
            })?;
        Ok(Self {
            root: metadata.workspace_root,
            target_directory: metadata.target_directory,
            application,
            members: metadata
                .packages
                .into_iter()
                .map(|member| member.manifest_path)
                .collect(),
        })
    }

    /// Whether a manifest is that of a package of the workspace itself.
    pub fn owns(&self, manifest: &PathBuf) -> bool {
        self.members.contains(manifest)
    }

    /// Downloads the sources of every dependency the lock file names, for
    /// every target, as a build would for its own. What reads the licences
    /// afterwards needs no network.
    pub fn fetch_sources(&self) -> Result<(), Error> {
        run(cargo().current_dir(&self.root).args(["fetch", "--locked"])).map(drop)
    }
}

/// Cargo, as the program that runs the tools.
pub fn cargo() -> Command {
    Command::new(std::env::var_os(CARGO_VARIABLE).unwrap_or_else(|| OsString::from(CARGO_PROGRAM)))
}

/// Runs a command to its end and gives what it wrote to its output.
///
/// | The command                    | Outcome                                   |
/// |--------------------------------|-------------------------------------------|
/// | ends with success              | its output                                |
/// | ends with failure              | refused, with what it wrote as its error  |
/// | cannot be started              | refused, with the reason                  |
pub fn run(command: &mut Command) -> Result<Vec<u8>, Error> {
    let shown = format!("{command:?}");
    let output = command.output().map_err(|error| Error::Command {
        command: shown.clone(),
        detail: error.to_string(),
    })?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(Error::Command {
            command: shown,
            detail: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::{cargo, run};
    use crate::notices::Error;

    #[test]
    fn a_command_that_succeeds_gives_its_output() -> Result<(), Error> {
        let output = run(cargo().arg("--version"))?;

        assert!(!output.is_empty());
        Ok(())
    }

    #[test]
    fn a_command_that_fails_is_refused_with_its_error() {
        let result = run(cargo().arg("--no-such-option"));

        assert!(matches!(result, Err(Error::Command { detail, .. }) if !detail.is_empty()));
    }

    #[test]
    fn a_command_that_cannot_be_started_is_refused() {
        let result = run(&mut Command::new("no-such-program-of-the-build-tools"));

        assert!(matches!(result, Err(Error::Command { .. })));
    }
}
