// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Checking one document: the step the window and the command line share.

use doc2einvoice_validation::{Profile, Validator, specification_identifier};

use super::document::Document;
use super::outcome::{InvoiceCheck, Refusal};

/// Checks documents, one after the other. It keeps what its validator has
/// compiled, so one checker should be reused, and it is bound to the thread it
/// is used on.
#[derive(Default)]
pub struct Checker {
    validator: Validator,
}

impl Checker {
    /// A checker with no rule set compiled yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks a document as the given profile, or as the one its specification
    /// identifier suggests.
    ///
    /// # Errors
    ///
    /// Returns the [`Refusal`] that says why the document was not checked.
    pub fn check(
        &mut self,
        document: &Document,
        profile: Option<Profile>,
    ) -> Result<InvoiceCheck, Refusal> {
        let identifier = specification_identifier(&document.text)?;
        let suggested = Profile::suggested_for(identifier.as_deref());
        let report = self
            .validator
            .validate(&document.text, profile.unwrap_or(suggested))?;

        Ok(InvoiceCheck::new(document, identifier, &report))
    }
}
