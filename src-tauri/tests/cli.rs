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

#[test]
fn more_than_one_file_for_the_window_is_explained_with_the_usage_status() -> std::io::Result<()> {
    let output = run(&["first.xml", "second.xml"])?;

    assert_eq!(output.status.code(), Some(i32::from(USAGE_ERROR_STATUS)));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("one file"));
    Ok(())
}

/// An invoice of the conformance corpus that conforms to XRechnung.
fn conforming_invoice() -> String {
    format!(
        "{}/../tests/conformance/corpus/xrechnung/instances/standard/01.01a-INVOICE_ubl.xml",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn check_succeeds_for_an_invoice_that_conforms() -> std::io::Result<()> {
    let invoice = conforming_invoice();

    let output = run(&["check", &invoice])?;
    let text = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{text}");
    assert!(text.contains(": conforms (profile: xrechnung,"), "{text}");
    assert!(
        text.ends_with("conform: 1, do not conform: 0, not checked: 0\n"),
        "{text}"
    );
    assert!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn check_fails_if_one_path_is_no_conforming_invoice() -> std::io::Result<()> {
    let invoice = conforming_invoice();

    let output = run(&["check", &invoice, "no-such-file.xml"])?;
    let text = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("no-such-file.xml: not checked: the file cannot be read"),
        "{text}"
    );
    assert!(
        text.ends_with("conform: 1, do not conform: 0, not checked: 1\n"),
        "{text}"
    );
    Ok(())
}

#[test]
fn check_as_another_profile_reports_what_the_invoice_fails() -> std::io::Result<()> {
    let standard = format!(
        "{}/../tests/conformance/corpus/en16931/examples-cii/CII_business_example_01.xml",
        env!("CARGO_MANIFEST_DIR")
    );

    let output = run(&["check", "--profile", "xrechnung", "--findings", &standard])?;
    let text = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains(": does not conform (profile: xrechnung,"),
        "{text}"
    );
    assert!(text.contains("    error\tBR-DE-"), "{text}");
    Ok(())
}

#[test]
fn check_without_a_path_is_a_usage_error() -> std::io::Result<()> {
    let output = run(&["check"])?;

    assert_eq!(output.status.code(), Some(i32::from(USAGE_ERROR_STATUS)));
    assert!(output.stdout.is_empty());
    Ok(())
}
