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
//! | `--version`, `-V`         | name and version on stdout, success       |
//! | `--help`, `-h`            | the help text on stdout, success          |
//! | `--version --help`        | the version: the first of the two decides |
//! | `--help --version`        | the help: the first of the two decides    |
//! | `--version invoice.pdf`   | the version: what follows is not read     |
//! | `invoice.pdf --version`   | usage error on stderr                     |
//! | `--version=1`             | usage error on stderr                     |
//! | `--unknown`, `-x`         | usage error on stderr                     |
//! | `--versio`                | usage error on stderr, no abbreviation    |
//! | `invoice.pdf`             | usage error on stderr, no file is opened  |
//! | `""`                      | usage error on stderr                     |
//! | `-- --version`            | usage error on stderr, it is a value      |
//! | an argument not in UTF-8  | usage error on stderr                     |
//!
//! Arguments are read from left to right. `--version` and `--help` answer at
//! once, as is customary; whatever follows them is not read. Up to that point
//! nothing is guessed: an argument that is not understood never starts the
//! application.
//!
//! On Windows the released program has no console of its own. Its answers
//! reach a pipe or a file, but a terminal it was called from shows nothing.
//! Attaching to that terminal takes a call of the operating system that Rust
//! cannot check, and this project contains no such code. That is a decision,
//! not an open point.

use std::ffi::OsString;

use clap::Command;

/// Exit status of a command line that was not understood, as is customary for
/// usage errors.
pub const USAGE_ERROR_STATUS: u8 = 2;

/// What a command line asks the program to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// Start the application.
    Start,
    /// Print a text and end without starting the application.
    Print(Output),
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
        Ok(_) => Invocation::Start,
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

/// The command line as the argument parser knows it. Name, version and
/// description come from the crate manifest, the one place they are stated.
fn command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .infer_long_args(false)
}

#[cfg(test)]
mod tests {
    use super::{Invocation, Output, Stream, interpret};

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
        assert_eq!(invocation(&[]), Invocation::Start);
    }

    #[test]
    fn a_bare_separator_starts_the_application() {
        assert_eq!(invocation(&["--"]), Invocation::Start);
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
    fn the_first_of_version_and_help_answers_and_the_rest_is_not_read() {
        assert!(is_version(&invocation(&["--version", "--help"])));
        assert!(is_help(&invocation(&["--help", "--version"])));
        assert!(is_version(&invocation(&["--version", "invoice.pdf"])));
    }

    #[test]
    fn what_is_not_understood_is_a_usage_error() {
        for arguments in [
            vec!["invoice.pdf", "--version"],
            vec!["--version=1"],
            vec!["--unknown"],
            vec!["-x"],
            vec!["--versio"],
            vec!["invoice.pdf"],
            vec![""],
            vec!["--", "--version"],
        ] {
            assert!(is_usage_error(&invocation(&arguments)), "{arguments:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn an_argument_that_is_not_utf8_is_a_usage_error() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        /// A byte that is no character in UTF-8.
        const NOT_UTF8: u8 = 0xFF;

        let arguments = [OsString::from(PROGRAM), OsString::from_vec(vec![NOT_UTF8])];

        assert!(is_usage_error(&interpret(arguments)));
    }
}
