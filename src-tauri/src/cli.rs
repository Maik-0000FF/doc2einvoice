// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of the program.
//!
//! The arguments are interpreted before anything of the application starts, so
//! that asking for the version opens no window and needs no display.
//!
//! What each command line leads to:
//!
//! | Arguments                 | Outcome                                   |
//! |---------------------------|-------------------------------------------|
//! | none                      | the application starts                    |
//! | `--`                      | the application starts, nothing follows   |
//! | `invoice.xml`             | the application starts and checks that    |
//! |                           | file in its window                        |
//! | `-- invoice.xml`          | the same                                  |
//! | `-- --version`, `-- check`| the application starts with a file of     |
//! |                           | that name: after `--` nothing is an option|
//! | `help`, `chec`            | the application starts with a file of     |
//! |                           | that name: there is no such command       |
//! | `a.xml b.xml`             | usage error on stderr: the window opens   |
//! |                           | one file                                  |
//! | `help check`, `chec a.xml`| the same: two words, two files            |
//! | `a.xml check b.xml`       | the same: a command stands first          |
//! | `""`                      | usage error on stderr: a file has a name  |
//! | an argument not in UTF-8  | the application starts with a file of     |
//! |                           | that name: a file name is no text         |
//! | `--version`, `-V`         | name and version on stdout, success       |
//! | `--help`, `-h`            | the help text on stdout, success          |
//! | `--version --help`        | the version: the first of the two decides |
//! | `--help --version`        | the help: the first of the two decides    |
//! | `--version invoice.xml`   | the version: what follows is not read     |
//! | `invoice.xml --version`   | the version: the file is not opened       |
//! | `invoice.xml --unknown`   | usage error on stderr                     |
//! | `--version=1`             | usage error on stderr                     |
//! | `--unknown`, `-x`         | usage error on stderr                     |
//! | `--versio`                | usage error on stderr, no abbreviation    |
//! | `check a.xml`             | the file is checked, no window opens      |
//! | `check a.xml b.xml dir`   | every path is checked, in that order      |
//! | `check`                   | usage error on stderr: nothing to check   |
//! | `check --profile xrechnung a.xml` | every file is checked as that     |
//! |                           | profile                                   |
//! | `check --profile other a.xml` | usage error on stderr               |
//! | `check --findings a.xml`  | the findings are listed as well           |
//! | `check --help`            | the help of the check on stdout, success  |
//! | `check -- --odd-name.xml` | the file of that name is checked          |
//! | `check` with a path not in UTF-8 | the path is checked: a file name   |
//! |                           | is no text                                |
//!
//! Arguments are read from left to right. `--version` and `--help` answer at
//! once, as is customary; whatever follows them is not read. A word that is
//! neither an option nor the command `check` is the name of a file: that is
//! how a file manager hands a file over ("Open with"). Whether the file exists
//! and is an invoice is found out in the window, which says so, exactly as for
//! a file chosen in its dialog. An option that is not known never starts the
//! application.
//!
//! On Windows the released program has no console of its own. Its answers
//! reach a pipe or a file, but a terminal it was called from shows nothing.
//! Attaching to that terminal takes a call of the operating system that Rust
//! cannot check, and this project contains no such code. That is a decision,
//! not an open point.

use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::PossibleValuesParser;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};

use crate::checking::batch::BatchRequest;
use crate::checking::outcome::ProfileKind;

/// Exit status of a command line that was not understood, as is customary for
/// usage errors.
pub const USAGE_ERROR_STATUS: u8 = 2;

/// What stderr says when more than one file is named for the window.
const ONE_FILE_ONLY: &str =
    "error: the window opens one file; to check several files, use `check`\n";

/// Names of the subcommand that checks invoices and of its arguments.
const FILE_ARGUMENT: &str = "file";
/// Takes whatever follows the one file, so that the program can say itself
/// why that is too much. It is not part of the help: there is nothing to
/// offer.
const FURTHER_FILES_ARGUMENT: &str = "further-files";
const CHECK_COMMAND: &str = "check";
const PATHS_ARGUMENT: &str = "paths";
const PROFILE_ARGUMENT: &str = "profile";
const FINDINGS_ARGUMENT: &str = "findings";

/// What a command line asks the program to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// Start the application, and check this file at once if one is named.
    Start(Option<PathBuf>),
    /// Print a text and end without starting the application.
    Print(Output),
    /// Check invoice files and end without starting the application.
    Check(BatchRequest),
}

/// A text for the terminal and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The complete text, including its final line break.
    pub text: String,
    /// The stream the text is written to.
    pub stream: Stream,
}

/// The two streams of a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// The answer to what was asked for: the version or the help text.
    Stdout,
    /// The explanation of a command line that was not understood.
    Stderr,
}

/// Interprets a command line. The first argument is the name the program was
/// called by, as the operating system passes it.
pub fn interpret<Arguments, Argument>(arguments: Arguments) -> Invocation
where
    Arguments: IntoIterator<Item = Argument>,
    Argument: Into<OsString> + Clone,
{
    match command().try_get_matches_from(arguments) {
        Ok(matches) => match matches.subcommand() {
            Some((CHECK_COMMAND, check)) => Invocation::Check(batch_request(check)),
            _ => start(&matches),
        },
        Err(error) => Invocation::Print(Output {
            text: error.render().to_string(),
            stream: if error.use_stderr() {
                Stream::Stderr
            } else {
                Stream::Stdout
            },
        }),
    }
}

/// Starts the application, with the file the command line names.
fn start(matches: &ArgMatches) -> Invocation {
    if matches.contains_id(FURTHER_FILES_ARGUMENT) {
        return Invocation::Print(Output {
            text: ONE_FILE_ONLY.to_owned(),
            stream: Stream::Stderr,
        });
    }
    Invocation::Start(matches.get_one::<PathBuf>(FILE_ARGUMENT).cloned())
}

/// The command line as the argument parser knows it. Name, version and
/// description come from the crate manifest, the one place they are stated.
fn command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .infer_long_args(false)
        .infer_subcommands(false)
        .disable_help_subcommand(true)
        .args_conflicts_with_subcommands(true)
        .arg(
            Arg::new(FILE_ARGUMENT)
                .value_name("FILE")
                .help("An e-invoice file (XML in UBL or CII) to open and check in the window")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new(FURTHER_FILES_ARGUMENT)
                .hide(true)
                .num_args(1..)
                .value_parser(value_parser!(PathBuf)),
        )
        .subcommand(check_command())
}

/// The subcommand that checks invoice files without opening a window.
fn check_command() -> Command {
    Command::new(CHECK_COMMAND)
        .about(
            "Check e-invoice files (XML in UBL or CII) and print one line per file. \
             Ends successfully only if every file is an invoice that conforms.",
        )
        .arg(
            Arg::new(PATHS_ARGUMENT)
                .value_name("PATH")
                .help("A file, or a directory whose XML files are checked")
                .required(true)
                .num_args(1..)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new(PROFILE_ARGUMENT)
                .long(PROFILE_ARGUMENT)
                .value_name("PROFILE")
                .help("Check every file as this profile instead of the one it names")
                .value_parser(PossibleValuesParser::new(
                    ProfileKind::ALL.map(ProfileKind::name),
                )),
        )
        .arg(
            Arg::new(FINDINGS_ARGUMENT)
                .long(FINDINGS_ARGUMENT)
                .help("List the findings of each file below its line")
                .action(ArgAction::SetTrue),
        )
}

/// What the arguments of the check ask for.
fn batch_request(check: &ArgMatches) -> BatchRequest {
    let chosen = check.get_one::<String>(PROFILE_ARGUMENT);
    BatchRequest {
        paths: check
            .get_many::<PathBuf>(PATHS_ARGUMENT)
            .map(|paths| paths.cloned().collect())
            .unwrap_or_default(),
        profile: ProfileKind::ALL
            .into_iter()
            .find(|profile| chosen.is_some_and(|name| name == profile.name()))
            .map(Into::into),
        list_findings: check.get_flag(FINDINGS_ARGUMENT),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Invocation, Output, Stream, interpret};
    use crate::checking::batch::BatchRequest;
    use crate::checking::outcome::ProfileKind;

    /// The name the program is called by in these tests.
    const PROGRAM: &str = env!("CARGO_PKG_NAME");

    fn invocation(arguments: &[&str]) -> Invocation {
        interpret(std::iter::once(PROGRAM).chain(arguments.iter().copied()))
    }

    fn version_line() -> String {
        format!("{PROGRAM} {}\n", env!("CARGO_PKG_VERSION"))
    }

    fn is_usage_error(invocation: &Invocation) -> bool {
        matches!(
            invocation,
            Invocation::Print(Output { text, stream: Stream::Stderr }) if !text.is_empty()
        )
    }

    fn is_help(invocation: &Invocation) -> bool {
        matches!(
            invocation,
            Invocation::Print(Output { text, stream: Stream::Stdout })
                if text.contains("--version") && text.contains("--help")
        )
    }

    fn is_version(invocation: &Invocation) -> bool {
        *invocation
            == Invocation::Print(Output {
                text: version_line(),
                stream: Stream::Stdout,
            })
    }

    #[test]
    fn no_argument_starts_the_application() {
        assert_eq!(invocation(&[]), Invocation::Start(None));
    }

    #[test]
    fn a_bare_separator_starts_the_application() {
        assert_eq!(invocation(&["--"]), Invocation::Start(None));
    }

    #[test]
    fn the_version_is_name_and_version_on_stdout() {
        for arguments in [["--version"], ["-V"]] {
            assert!(is_version(&invocation(&arguments)), "{arguments:?}");
        }
    }

    #[test]
    fn the_help_goes_to_stdout() {
        for arguments in [["--help"], ["-h"]] {
            assert!(is_help(&invocation(&arguments)), "{arguments:?}");
        }
    }

    #[test]
    fn the_help_offers_one_file() {
        /// How the help marks an argument that may be given several times.
        const REPEATABLE: &str = "...";

        let Invocation::Print(Output { text, .. }) = invocation(&["--help"]) else {
            unreachable!("the help is printed");
        };
        let lines_about_the_file: Vec<&str> = text
            .lines()
            .filter(|line| line.contains("[FILE]"))
            .collect();

        assert!(!lines_about_the_file.is_empty());
        assert!(
            lines_about_the_file
                .iter()
                .all(|line| !line.contains(REPEATABLE)),
            "{text}"
        );
    }

    #[test]
    fn the_first_of_version_and_help_answers_and_the_rest_is_not_read() {
        assert!(is_version(&invocation(&["--version", "--help"])));
        assert!(is_help(&invocation(&["--help", "--version"])));
        assert!(is_version(&invocation(&["--version", "invoice.xml"])));
    }

    #[test]
    fn what_is_not_understood_is_a_usage_error() {
        for arguments in [
            vec!["invoice.xml", "--unknown"],
            vec!["--version=1"],
            vec!["--unknown"],
            vec!["-x"],
            vec!["--versio"],
            vec![""],
        ] {
            assert!(is_usage_error(&invocation(&arguments)), "{arguments:?}");
        }
    }

    #[test]
    fn a_file_name_starts_the_application_with_that_file() {
        for (arguments, file) in [
            (vec!["invoice.xml"], "invoice.xml"),
            (vec!["--", "invoice.xml"], "invoice.xml"),
            (vec!["--", "--version"], "--version"),
            (vec!["--", "check"], "check"),
            (vec!["help"], "help"),
            (vec!["chec"], "chec"),
        ] {
            assert_eq!(
                invocation(&arguments),
                Invocation::Start(Some(PathBuf::from(file))),
                "{arguments:?}"
            );
        }
    }

    #[test]
    fn more_than_one_file_is_a_usage_error() {
        for arguments in [
            vec!["a.xml", "b.xml"],
            vec!["help", "check"],
            vec!["chec", "a.xml"],
            vec!["a.xml", "check", "b.xml"],
            vec!["--", "a.xml", "b.xml"],
        ] {
            assert!(is_usage_error(&invocation(&arguments)), "{arguments:?}");
        }
    }

    #[test]
    fn the_version_answers_also_after_a_file_name() {
        assert!(is_version(&invocation(&["invoice.xml", "--version"])));
    }

    fn check_request(arguments: &[&str]) -> Option<BatchRequest> {
        match invocation(arguments) {
            Invocation::Check(request) => Some(request),
            _ => None,
        }
    }

    #[test]
    fn check_takes_every_path_in_the_order_given() {
        let request = check_request(&["check", "a.xml", "b.xml", "dir"]);

        assert_eq!(
            request,
            Some(BatchRequest {
                paths: ["a.xml", "b.xml", "dir"].map(PathBuf::from).to_vec(),
                profile: None,
                list_findings: false,
            })
        );
    }

    #[test]
    fn check_takes_a_profile_and_the_wish_for_findings() {
        for profile in ProfileKind::ALL {
            let request =
                check_request(&["check", "--profile", profile.name(), "--findings", "a.xml"]);

            assert_eq!(
                request,
                Some(BatchRequest {
                    paths: vec![PathBuf::from("a.xml")],
                    profile: Some(profile.into()),
                    list_findings: true,
                })
            );
        }
    }

    #[test]
    fn check_takes_a_file_whose_name_looks_like_an_option() {
        let request = check_request(&["check", "--", "--odd-name.xml"]);

        assert_eq!(
            request.map(|request| request.paths),
            Some(vec![PathBuf::from("--odd-name.xml")])
        );
    }

    #[test]
    fn check_without_a_sensible_request_is_a_usage_error() {
        for arguments in [
            vec!["check"],
            vec!["check", "--profile", "other", "a.xml"],
            vec!["check", "--profile"],
            vec!["check", "--unknown", "a.xml"],
        ] {
            assert!(is_usage_error(&invocation(&arguments)), "{arguments:?}");
        }
    }

    #[test]
    fn the_help_of_check_goes_to_stdout() {
        assert!(matches!(
            invocation(&["check", "--help"]),
            Invocation::Print(Output { text, stream: Stream::Stdout }) if text.contains("--profile")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn check_takes_a_path_that_is_not_utf8() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        /// A byte that is no character in UTF-8.
        const NOT_UTF8: u8 = 0xFF;

        let name = OsString::from_vec(vec![NOT_UTF8]);
        let arguments = [
            OsString::from(PROGRAM),
            OsString::from("check"),
            name.clone(),
        ];

        assert_eq!(
            interpret(arguments),
            Invocation::Check(BatchRequest {
                paths: vec![PathBuf::from(name)],
                profile: None,
                list_findings: false,
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_argument_that_is_not_utf8_is_a_file_name() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        /// A byte that is no character in UTF-8.
        const NOT_UTF8: u8 = 0xFF;

        let name = OsString::from_vec(vec![NOT_UTF8]);
        let arguments = [OsString::from(PROGRAM), name.clone()];

        assert_eq!(
            interpret(arguments),
            Invocation::Start(Some(PathBuf::from(name)))
        );
    }
}
