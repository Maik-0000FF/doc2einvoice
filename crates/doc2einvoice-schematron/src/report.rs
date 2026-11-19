// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The result of validating one document.

/// Everything a rule file has to say about one document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    findings: Vec<Finding>,
}

impl Report {
    pub(crate) fn new(findings: Vec<Finding>) -> Self {
        Self { findings }
    }

    /// The assertions the document does not satisfy, in the order the rule
    /// file states them. Empty if the document satisfies every assertion.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Whether the document satisfies every assertion of the rule file.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// One assertion that a node of the document does not satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The identifier of the assertion, for example `BR-CO-15`.
    pub assertion_id: String,
    /// The `flag` the rule file gives the assertion, usually `fatal` or
    /// `warning`. What a flag means is defined by the rule file.
    pub flag: Option<String>,
    /// The text of the assertion as written in the rule file.
    pub message: String,
    /// The node the assertion was checked on, as a path from the document
    /// element, for example `/Invoice[1]/InvoiceLine[2]`.
    pub location: String,
    /// Why the assertion counts as not satisfied.
    pub outcome: Outcome,
}

/// Why an assertion counts as not satisfied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The test of the assertion is false.
    Violated,
    /// The test could not be evaluated, for example because a value that must
    /// be a number is not one. The assertion is treated as violated.
    EvaluationError {
        /// The error as reported by the XPath engine, for example `FORG0001`.
        error: String,
    },
}
