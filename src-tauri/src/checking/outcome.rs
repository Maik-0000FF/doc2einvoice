// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What checking an invoice leads to, as the user interface receives it.
//!
//! Everything here is data and identifiers. The only wording is the one of
//! other publishers: the text of a rule and the explanation of the XML Schema
//! processor. What the interface says about an outcome, it says in its own
//! language files.

use doc2einvoice_validation::{
    Profile, RuleSetResult, SchemaFinding, Severity, Syntax, ValidationError, ValidationFinding,
    ValidationReport,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::document::Document;
use crate::app_info::Publication;

/// How a request to check an invoice ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CheckOutcome {
    /// The user closed the file dialog without choosing a file.
    Cancelled,
    /// The file is an invoice and was checked.
    Checked {
        /// The result of the check.
        check: InvoiceCheck,
    },
    /// The file could not be checked.
    Refused {
        /// Why not.
        refusal: Refusal,
    },
}

/// Why a file was not checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "reason", rename_all = "camelCase")]
#[ts(export)]
pub enum Refusal {
    /// The path leads to a directory, a device or anything else but a file.
    NotAFile,
    /// The file does not exist or may not be read.
    Unreadable,
    /// The file is larger than what is read.
    #[serde(rename_all = "camelCase")]
    TooLarge {
        /// The largest size that is read, in bytes.
        #[ts(type = "number")]
        limit_bytes: u64,
    },
    /// The file is not text in UTF-8.
    NotUtf8,
    /// The file is not well-formed XML.
    Malformed {
        /// The explanation of the XML parser, in English.
        detail: String,
    },
    /// The file is XML but none of the supported invoices.
    UnsupportedDocument {
        /// Local name of its document element.
        element: String,
    },
    /// There is no file to check again: none was checked before.
    NothingToCheck,
    /// The check failed for a reason that lies in the program, not in the file.
    Internal,
}

/// The result of checking one invoice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InvoiceCheck {
    /// The name of the file, without its directory.
    pub file_name: String,
    /// The syntax the invoice is written in.
    pub syntax: SyntaxKind,
    /// The profile the invoice was checked as.
    pub profile: ProfileKind,
    /// The profile its specification identifier suggests.
    pub suggested_profile: ProfileKind,
    /// The specification the invoice says it follows (BT-24), if it says so.
    pub specification_identifier: Option<String>,
    /// Whether the invoice is valid against the XML Schema and breaks no rule
    /// that makes it invalid.
    pub conforms: bool,
    /// What makes the invoice invalid against the XML Schema of its syntax.
    pub schema_findings: Vec<SchemaNote>,
    /// The result of every rule set of the profile, in the order applied.
    pub rule_sets: Vec<RuleSetCheck>,
}

/// The result of one rule set, with the release it is a statement about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleSetCheck {
    /// The release of the rule set the findings are a statement about.
    pub publication: Publication,
    /// The rules the invoice does not satisfy, in the order of the rule file.
    pub findings: Vec<RuleNote>,
}

/// One rule the invoice does not satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RuleNote {
    /// The identifier of the rule, for example `BR-CO-15`.
    pub rule_id: String,
    /// How serious the finding is.
    pub severity: SeverityKind,
    /// The wording of the rule as its publisher wrote it.
    pub message: String,
    /// The node the rule was checked on, as a path from the document element.
    pub location: String,
    /// Set if the rule could not be evaluated, to the error code of the XPath
    /// engine.
    pub evaluation_error: Option<String>,
}

/// One violation of the XML Schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SchemaNote {
    /// The validation rule of XML Schema that is broken.
    pub code: String,
    /// The explanation of the schema processor, in English.
    pub message: String,
    /// The node the finding concerns, as a path from the document element.
    pub location: String,
}

/// The XML syntax of an invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SyntaxKind {
    /// OASIS Universal Business Language 2.1.
    Ubl,
    /// UN/CEFACT Cross Industry Invoice D16B.
    Cii,
}

/// What an invoice is checked as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ProfileKind {
    /// The European standard EN 16931 on its own.
    En16931,
    /// The German XRechnung.
    #[serde(rename = "xrechnung")]
    XRechnung,
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SeverityKind {
    /// A note that does not question the invoice.
    Information,
    /// The invoice should be corrected but is not invalid.
    Warning,
    /// The invoice does not conform to the rule set.
    Fatal,
}

impl ProfileKind {
    /// Every profile, in the order they are offered.
    pub const ALL: [Self; 2] = [Self::En16931, Self::XRechnung];

    /// The name of the profile on the command line and in what the user
    /// interface receives.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::En16931 => "en16931",
            Self::XRechnung => "xrechnung",
        }
    }
}

impl InvoiceCheck {
    /// Describes a validation report for the user interface.
    #[must_use]
    pub fn new(
        document: &Document,
        specification_identifier: Option<String>,
        suggested_profile: Profile,
        report: &ValidationReport,
    ) -> Self {
        Self {
            file_name: document.file_name.clone(),
            syntax: report.syntax.into(),
            profile: report.profile.into(),
            suggested_profile: suggested_profile.into(),
            specification_identifier,
            conforms: report.conforms(),
            schema_findings: report
                .schema_findings
                .iter()
                .map(SchemaNote::from)
                .collect(),
            rule_sets: report.results.iter().map(RuleSetCheck::from).collect(),
        }
    }
}

impl From<&RuleSetResult> for RuleSetCheck {
    fn from(result: &RuleSetResult) -> Self {
        Self {
            publication: Publication::from(result.rule_set.info()),
            findings: result.findings.iter().map(RuleNote::from).collect(),
        }
    }
}

impl From<&ValidationFinding> for RuleNote {
    fn from(finding: &ValidationFinding) -> Self {
        Self {
            rule_id: finding.rule_id.clone(),
            severity: finding.severity.into(),
            message: finding.message.clone(),
            location: finding.location.clone(),
            evaluation_error: finding.evaluation_error.clone(),
        }
    }
}

impl From<&SchemaFinding> for SchemaNote {
    fn from(finding: &SchemaFinding) -> Self {
        Self {
            code: finding.code.clone(),
            message: finding.message.clone(),
            location: finding.location.clone(),
        }
    }
}

impl From<Syntax> for SyntaxKind {
    fn from(syntax: Syntax) -> Self {
        match syntax {
            Syntax::Ubl => Self::Ubl,
            Syntax::Cii => Self::Cii,
        }
    }
}

impl From<Profile> for ProfileKind {
    fn from(profile: Profile) -> Self {
        match profile {
            Profile::En16931 => Self::En16931,
            Profile::XRechnung => Self::XRechnung,
        }
    }
}

impl From<ProfileKind> for Profile {
    fn from(profile: ProfileKind) -> Self {
        match profile {
            ProfileKind::En16931 => Self::En16931,
            ProfileKind::XRechnung => Self::XRechnung,
        }
    }
}

impl From<Severity> for SeverityKind {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Information => Self::Information,
            Severity::Warning => Self::Warning,
            Severity::Fatal => Self::Fatal,
        }
    }
}

/// A file that cannot be validated is refused with the reason that lies in
/// the file. What lies in the program (a rule set or a schema that cannot be
/// used, an engine that failed) is an internal failure: the user can do
/// nothing about it, and its wording is not for the interface.
impl From<ValidationError> for Refusal {
    fn from(error: ValidationError) -> Self {
        match error {
            ValidationError::Malformed(detail) => Self::Malformed { detail },
            ValidationError::UnsupportedDocument { element } => {
                Self::UnsupportedDocument { element }
            }
            ValidationError::XmlSchema(_)
            | ValidationError::RuleSet(_)
            | ValidationError::Evaluation(_) => Self::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProfileKind;

    /// The command line and the user interface must call a profile the same.
    #[test]
    fn a_profile_has_one_name_everywhere() -> Result<(), serde_json::Error> {
        for profile in ProfileKind::ALL {
            assert_eq!(serde_json::to_value(profile)?, profile.name());
        }
        Ok(())
    }
}
