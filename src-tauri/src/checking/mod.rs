// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Checking an invoice the user chose: load the file, validate it on the
//! validation thread, and describe the result for the user interface.
//!
//! - [`document`]: reads the chosen file into memory, within limits.
//! - [`checker`]: checks one document; the step every way of checking shares.
//! - [`service`]: the checker on a thread of its own, for the window.
//! - [`batch`]: checks many files for the command line.
//! - [`outcome`]: the result as data for the interface.

pub mod batch;
pub mod checker;
pub mod document;
pub mod outcome;
pub mod service;

use std::path::Path;

use doc2einvoice_validation::Profile;

use self::outcome::CheckOutcome;
use self::service::ValidationService;

/// Loads the file at the given path and checks it.
#[must_use]
pub fn check_file(service: &ValidationService, path: &Path) -> CheckOutcome {
    outcome_of(document::load(path).and_then(|document| service.check(document)))
}

/// Checks the invoice of the last check as another profile.
#[must_use]
pub fn check_again(service: &ValidationService, profile: Profile) -> CheckOutcome {
    outcome_of(service.check_again(profile))
}

fn outcome_of(result: Result<outcome::InvoiceCheck, outcome::Refusal>) -> CheckOutcome {
    match result {
        Ok(check) => CheckOutcome::Checked { check },
        Err(refusal) => CheckOutcome::Refused { refusal },
    }
}
