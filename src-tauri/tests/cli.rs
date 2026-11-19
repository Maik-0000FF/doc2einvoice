// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The command line of the built program: what it prints, where, and the
//! status it ends with. None of these calls opens a window.

use std::process::{Command, Output};

use doc2einvoice_lib::cli::USAGE_ERROR_STATUS;

/// Runs the built program with the given arguments.
fn run(arguments: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_doc2einvoice"))
        .args(arguments)
        .output()
}

#[test]
fn version_prints_name_and_version_and_succeeds() -> std::io::Result<()> {
    let expected = format!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    for argument in ["--version", "-V"] {
        let output = run(&[argument])?;

        assert!(output.status.success(), "{argument}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
        assert!(output.stderr.is_empty(), "{argument}");
    }
    Ok(())
}

#[test]
fn help_goes_to_stdout_and_succeeds() -> std::io::Result<()> {
    let output = run(&["--help"])?;

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("--version"));
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn an_unknown_argument_is_explained_on_stderr_with_the_usage_status() -> std::io::Result<()> {
    let output = run(&["--unknown"])?;

    assert_eq!(output.status.code(), Some(i32::from(USAGE_ERROR_STATUS)));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    Ok(())
}
