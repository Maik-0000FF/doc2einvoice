// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Desktop entry point.

// Release builds on Windows must not open a console window next to the GUI.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::Write;
use std::process::ExitCode;

use doc2einvoice_lib::Ending;
use doc2einvoice_lib::checking::batch::{self, BatchRequest};
use doc2einvoice_lib::cli::{Invocation, Output, Stream, USAGE_ERROR_STATUS, interpret};

/// What stderr says when the user interface was lost.
const WEB_VIEW_LOST: &str = "the web view that draws the window ended unexpectedly, \
    the user interface cannot be shown; a graphics driver that does not work is a common cause";

fn main() -> ExitCode {
    match interpret(std::env::args_os()) {
        Invocation::Start => start(),
        Invocation::Print(output) => print(&output),
        Invocation::Check(request) => check(&request),
    }
}

/// Checks invoice files and writes the result to stdout. The run succeeds only
/// if every path led to invoices and every one of them conforms; a result that
/// cannot be written is a failure as well.
fn check(request: &BatchRequest) -> ExitCode {
    match batch::run(request, &mut std::io::stdout().lock()) {
        Ok(tally) if tally.all_conform() => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

/// Runs the application until its last window closes. A run that could not
/// show its user interface is a failure, with the reason on stderr.
fn start() -> ExitCode {
    match doc2einvoice_lib::run() {
        Ok(Ending::Closed) => ExitCode::SUCCESS,
        Ok(Ending::WebViewLost) => {
            eprintln!("{}: {WEB_VIEW_LOST}", env!("CARGO_PKG_NAME"));
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{}: {error}", env!("CARGO_PKG_NAME"));
            ExitCode::FAILURE
        }
    }
}

/// Writes a text to its stream. An answer ends successfully, the explanation of
/// a command line that was not understood with the status of a usage error.
///
/// A write the operating system refuses is a failure: a pipe whose reader is
/// gone, a device that is full. A stream that was closed before the program
/// started is different: the standard library then reports the write as done,
/// so the program ends as if the text had been read.
fn print(output: &Output) -> ExitCode {
    let (written, status) = match output.stream {
        Stream::Stdout => (
            std::io::stdout().write_all(output.text.as_bytes()),
            ExitCode::SUCCESS,
        ),
        Stream::Stderr => (
            std::io::stderr().write_all(output.text.as_bytes()),
            ExitCode::from(USAGE_ERROR_STATUS),
        ),
    };
    match written {
        Ok(()) => status,
        Err(_) => ExitCode::FAILURE,
    }
}
