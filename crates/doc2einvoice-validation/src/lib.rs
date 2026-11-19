// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Validation of e-invoices against published rule sets.
//!
//! The rule files are embedded unchanged, exactly as their publishers released
//! them, and evaluated by the Schematron interpreter. Every rule set states
//! its version and the date it was published: a validation result is a
//! statement about those versions and nothing else.
//!
//! The layers:
//!
//! - [`XmlSchemaInfo`]: the XML Schema of a syntax with its provenance.
//! - [`RuleSet`]: one published rule file with its provenance and checksum.
//! - [`Syntax`]: which XML syntax a document is written in.
//! - [`Profile`]: what a document is validated as, which decides the rule sets.
//! - [`specification_identifier`]: the specification a document says it follows.
//! - [`Validator`]: validates documents and returns a [`ValidationReport`].
//!
//! A document is checked in the two steps a specification requires: against
//! the XML Schema of its syntax, and against the Schematron rules.
//!
//! Findings carry identifiers and data, never text for a user interface. The
//! only text in a finding is the wording of the rule as its publisher wrote it.
//!
//! ```
//! use doc2einvoice_validation::{Profile, Validator};
//!
//! let invoice = r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#;
//! let report = Validator::new().validate(invoice, Profile::En16931)?;
//!
//! // An empty invoice breaks many rules of the standard.
//! assert!(!report.conforms());
//! # Ok::<(), doc2einvoice_validation::ValidationError>(())
//! ```

mod embedded;
mod error;
mod nesting;
mod profile;
mod report;
mod rule_set;
mod specification;
mod syntax;
mod validator;
mod xml_schema;

pub use embedded::EmbeddedFile;
pub use error::{RuleSetError, ValidationError, XmlSchemaError};
pub use nesting::MAX_NESTING_DEPTH;
pub use profile::Profile;
pub use report::{RuleSetResult, SchemaFinding, Severity, ValidationFinding, ValidationReport};
pub use rule_set::{RuleSet, RuleSetInfo};
pub use specification::specification_identifier;
pub use syntax::Syntax;
pub use validator::Validator;
pub use xml_schema::{XmlSchema, XmlSchemaInfo};

/// The Schematron interpreter the rule sets are evaluated with.
pub use doc2einvoice_schematron as schematron;
