// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Validating invoices.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use doc2einvoice_schematron::Schema;

use crate::error::ValidationError;
use crate::profile::Profile;
use crate::report::{RuleSetResult, ValidationFinding, ValidationReport};
use crate::rule_set::RuleSet;
use crate::syntax::Syntax;
use crate::xml_schema::{XmlSchema, XmlSchemaInfo};

/// Validates invoices against the rule sets of a profile.
///
/// A document is validated in the two steps a specification requires: against
/// the XML Schema of its syntax, and against the rule sets of the profile.
///
/// A schema or a rule set is compiled the first time a document needs it and
/// kept for the documents that follow, so one validator should be reused. A
/// validator is bound to the thread it is used on.
#[derive(Default)]
pub struct Validator {
    schemas: HashMap<Syntax, XmlSchema>,
    compiled: HashMap<RuleSet, Schema>,
}

impl Validator {
    /// A validator with no rule set compiled yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates an invoice as the given profile.
    ///
    /// # Errors
    ///
    /// Returns a [`ValidationError`] if the document cannot be validated at
    /// all: it is not well-formed XML, it is not a supported invoice, or a
    /// rule set cannot be used. Rules the invoice does not satisfy are not
    /// errors; they are the findings of the returned report.
    pub fn validate(
        &mut self,
        document: &str,
        profile: Profile,
    ) -> Result<ValidationReport, ValidationError> {
        let syntax = Syntax::of(document)?;
        let schema_findings = self.xml_schema(syntax)?.validate(document)?;
        let mut results = Vec::new();

        for &rule_set in profile.rule_sets(syntax) {
            let report = self.schema(rule_set)?.validate(document)?;
            results.push(RuleSetResult {
                rule_set,
                findings: report
                    .findings()
                    .iter()
                    .map(ValidationFinding::from)
                    .collect(),
            });
        }
        Ok(ValidationReport {
            profile,
            syntax,
            schema_findings,
            results,
        })
    }

    fn xml_schema(&mut self, syntax: Syntax) -> Result<&XmlSchema, ValidationError> {
        match self.schemas.entry(syntax) {
            Entry::Occupied(compiled) => Ok(compiled.into_mut()),
            Entry::Vacant(vacant) => Ok(vacant.insert(XmlSchemaInfo::of(syntax).compile()?)),
        }
    }

    fn schema(&mut self, rule_set: RuleSet) -> Result<&Schema, ValidationError> {
        match self.compiled.entry(rule_set) {
            Entry::Occupied(compiled) => Ok(compiled.into_mut()),
            Entry::Vacant(vacant) => Ok(vacant.insert(rule_set.compile()?)),
        }
    }
}
