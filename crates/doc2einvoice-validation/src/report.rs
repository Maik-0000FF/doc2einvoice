// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The result of validating one invoice.

use doc2einvoice_schematron::{Finding, Outcome};

use crate::profile::Profile;
use crate::rule_set::RuleSet;
use crate::syntax::Syntax;

const FLAG_WARNING: &str = "warning";
const FLAG_INFORMATION: &str = "information";

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// A note that does not question the invoice.
    Information,
    /// The invoice should be corrected but is not invalid.
    Warning,
    /// The invoice does not conform to the rule set.
    Fatal,
}

impl Severity {
    /// The severity a rule file assigns through the `flag` of an assertion.
    ///
    /// A missing or unknown flag is read as fatal: a finding whose weight is
    /// not known must not be played down.
    fn of_flag(flag: Option<&str>) -> Self {
        match flag {
            Some(FLAG_WARNING) => Self::Warning,
            Some(FLAG_INFORMATION) => Self::Information,
            _ => Self::Fatal,
        }
    }
}

/// One rule an invoice does not satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFinding {
    /// The identifier of the rule, for example `BR-CO-15`.
    pub rule_id: String,
    /// How serious the finding is.
    pub severity: Severity,
    /// The wording of the rule as its publisher wrote it.
    pub message: String,
    /// The node the rule was checked on, as a path from the document element.
    pub location: String,
    /// Set if the rule could not be evaluated, to the error the XPath engine
    /// reported (for example `FORG0001`). Such a finding is always fatal.
    pub evaluation_error: Option<String>,
}

impl From<&Finding> for ValidationFinding {
    fn from(finding: &Finding) -> Self {
        let (severity, evaluation_error) = match &finding.outcome {
            Outcome::Violated => (Severity::of_flag(finding.flag.as_deref()), None),
            // A rule that cannot be evaluated cannot be shown to hold, whatever
            // weight the rule file gives it.
            Outcome::EvaluationError { error } => (Severity::Fatal, Some(error.clone())),
        };
        Self {
            rule_id: finding.assertion_id.clone(),
            severity,
            message: finding.message.clone(),
            location: finding.location.clone(),
            evaluation_error,
        }
    }
}

/// One reason a document is not valid against the XML Schema of its syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaFinding {
    /// The validation rule of XML Schema that is broken, for example
    /// `cvc-complex-type.2.4`.
    pub code: String,
    /// The explanation of the schema processor, in English.
    pub message: String,
    /// The node the finding concerns, as a path from the document element.
    pub location: String,
}

/// The findings of one rule set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetResult {
    /// The rule set the invoice was checked against.
    pub rule_set: RuleSet,
    /// The rules the invoice does not satisfy, in the order of the rule file.
    pub findings: Vec<ValidationFinding>,
}

/// Everything the rule sets of a profile have to say about one invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// The profile the invoice was validated as.
    pub profile: Profile,
    /// The syntax the invoice is written in.
    pub syntax: Syntax,
    /// What makes the invoice invalid against the XML Schema of its syntax.
    /// Empty if it is valid.
    pub schema_findings: Vec<SchemaFinding>,
    /// The result of every rule set of the profile, in the order applied.
    pub results: Vec<RuleSetResult>,
}

impl ValidationReport {
    /// Whether the invoice conforms to the profile: it is valid against the
    /// XML Schema of its syntax and satisfies the rules.
    #[must_use]
    pub fn conforms(&self) -> bool {
        self.is_schema_valid() && self.satisfies_rules()
    }

    /// Whether the invoice is valid against the XML Schema of its syntax.
    #[must_use]
    pub fn is_schema_valid(&self) -> bool {
        self.schema_findings.is_empty()
    }

    /// Whether the invoice satisfies the rules: no rule set reports a fatal
    /// finding. The rules are applied to every document, also to one that is
    /// not valid against the XML Schema, so that all problems are reported in
    /// one pass.
    #[must_use]
    pub fn satisfies_rules(&self) -> bool {
        self.findings()
            .all(|finding| finding.severity < Severity::Fatal)
    }

    /// All findings of all rule sets.
    pub fn findings(&self) -> impl Iterator<Item = &ValidationFinding> {
        self.results.iter().flat_map(|result| &result.findings)
    }
}

#[cfg(test)]
mod tests {
    use doc2einvoice_schematron::{Finding, Outcome};

    use super::{Severity, ValidationFinding};

    fn finding(flag: Option<&str>, outcome: Outcome) -> Finding {
        Finding {
            assertion_id: "R-1".to_owned(),
            flag: flag.map(str::to_owned),
            message: "text".to_owned(),
            location: "/Invoice[1]".to_owned(),
            outcome,
        }
    }

    // Input table for the severity of a finding:
    //
    // | flag          | outcome          | severity    |
    // |---------------|------------------|-------------|
    // | fatal         | violated         | Fatal       |
    // | warning       | violated         | Warning     |
    // | information   | violated         | Information |
    // | missing       | violated         | Fatal       |
    // | unknown word  | violated         | Fatal       |
    // | warning       | evaluation error | Fatal       |
    #[test]
    fn the_flag_decides_the_severity_of_a_violated_rule() {
        for (flag, expected) in [
            (Some("fatal"), Severity::Fatal),
            (Some("warning"), Severity::Warning),
            (Some("information"), Severity::Information),
            (None, Severity::Fatal),
            (Some("unheard-of"), Severity::Fatal),
        ] {
            let converted = ValidationFinding::from(&finding(flag, Outcome::Violated));

            assert_eq!(converted.severity, expected, "flag {flag:?}");
            assert_eq!(converted.evaluation_error, None);
        }
    }

    #[test]
    fn a_rule_that_cannot_be_evaluated_is_fatal_whatever_its_flag() {
        let outcome = Outcome::EvaluationError {
            error: "FORG0001".to_owned(),
        };
        let converted = ValidationFinding::from(&finding(Some("warning"), outcome));

        assert_eq!(converted.severity, Severity::Fatal);
        assert_eq!(converted.evaluation_error.as_deref(), Some("FORG0001"));
    }
}
