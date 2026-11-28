// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The build tools of this repository, run as `cargo xtask <task>`.
//!
//! A tool lives here when it needs more than a few lines and has to read the
//! files of the workspace. It is not part of the application.

mod notices;

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use crate::notices::Selection;

/// The task that generates the list of the contained libraries.
const NOTICES_TASK: &str = "notices";
/// Its option that names one target.
const TARGET_OPTION: &str = "target";
/// Its option that asks for every target packages are built for.
const ALL_TARGETS_OPTION: &str = "all-targets";

/// The variable in which the packaging tool names the target it builds for,
/// set for the command it runs before a build.
const PACKAGING_TARGET_VARIABLE: &str = "TAURI_ENV_TARGET_TRIPLE";

fn command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new(NOTICES_TASK)
                .about(
                    "Generates the list of the libraries a package contains, with the texts \
                     they ask to be passed on",
                )
                .arg(
                    Arg::new(TARGET_OPTION)
                        .long(TARGET_OPTION)
                        .value_name("TRIPLE")
                        .help(
                            "The target the package is built for [default: the one being \
                             packaged]",
                        ),
                )
                .arg(
                    Arg::new(ALL_TARGETS_OPTION)
                        .long(ALL_TARGETS_OPTION)
                        .action(ArgAction::SetTrue)
                        .conflicts_with(TARGET_OPTION)
                        .help(
                            "Every target packages are built for, and a check that no setting \
                             is left unused",
                        ),
                ),
        )
}

/// Which targets the list is asked for.
///
/// | Command line      | Packaging tool names a target | Selection          |
/// |-------------------|-------------------------------|--------------------|
/// | `--all-targets`   | any                           | every target       |
/// | `--target T`      | any                           | `T`                |
/// | neither           | yes, `P`                      | `P`                |
/// | neither           | no                            | none: to be named  |
/// | both              | any                           | refused by clap    |
fn selection(all: bool, named: Option<&String>, packaging: Option<String>) -> Option<Selection> {
    if all {
        Some(Selection::All)
    } else {
        named.cloned().or(packaging).map(Selection::One)
    }
}

fn main() -> ExitCode {
    let matches = command().get_matches();
    let result = match matches.subcommand() {
        Some((NOTICES_TASK, task)) => match selection(
            task.get_flag(ALL_TARGETS_OPTION),
            task.get_one::<String>(TARGET_OPTION),
            std::env::var(PACKAGING_TARGET_VARIABLE).ok(),
        ) {
            Some(selection) => notices::run(&selection).map_err(|error| error.to_string()),
            None => Err(format!(
                "Name the target with --{TARGET_OPTION}, or ask for every target with \
                 --{ALL_TARGETS_OPTION}."
            )),
        },
        // Not reached: a task is required, and no other is defined.
        _ => Ok(()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Selection, command, selection};

    #[test]
    fn the_command_line_is_consistent() {
        command().debug_assert();
    }

    #[test]
    fn the_selection_follows_its_table() {
        let named = "named".to_owned();
        let packaging = || Some("packaged".to_owned());
        let one = |target: &str| Some(Selection::One(target.to_owned()));
        let table = [
            (true, None, packaging(), Some(Selection::All)),
            (true, None, None, Some(Selection::All)),
            (false, Some(&named), packaging(), one("named")),
            (false, Some(&named), None, one("named")),
            (false, None, packaging(), one("packaged")),
            (false, None, None, None),
        ];
        for (all, named, packaging, expected) in table {
            assert_eq!(selection(all, named, packaging), expected);
        }
    }

    #[test]
    fn one_target_and_all_targets_exclude_each_other() {
        let result =
            command().try_get_matches_from(["xtask", "notices", "--target", "t", "--all-targets"]);

        assert!(result.is_err());
    }
}
