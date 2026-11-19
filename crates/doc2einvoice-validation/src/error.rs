// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors of loading a rule set and of validating a document.

use doc2einvoice_schematron::{DocumentError, SchemaError};

/// Why an embedded rule set cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum RuleSetError {
    /// An embedded rule file is not the file its publisher released.
    #[error("the rule file `{file}` does not have its published checksum")]
    Integrity {
        /// Name of the rule file.
        file: &'static str,
    },

    /// The rule file cannot be compiled.
    #[error(transparent)]
    Schema(#[from] SchemaError),
}

/// Why an embedded XML Schema cannot be used.
#[derive(Debug, thiserror::Error)]
pub enum XmlSchemaError {
    /// An embedded schema file is not the file its publisher released.
    #[error("the schema file `{file}` does not have its published checksum")]
    Integrity {
        /// Name of the schema file.
        file: &'static str,
    },

    /// The schema processor failed in a way it could not report as an error.
    #[error("the XML Schema processor failed")]
    ProcessorFailure,

    /// The schema cannot be assembled from the embedded files.
    #[error("the XML Schema cannot be compiled: {reason}")]
    Compile {
        /// The message of the schema processor.
        reason: String,
    },
}

/// Why a document could not be validated.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// The document is not well-formed XML.
    #[error("the document is not well-formed XML: {0}")]
    Malformed(String),

    /// The document is none of the invoice syntaxes this crate validates.
    #[error("the document element `{element}` is not a supported invoice")]
    UnsupportedDocument {
        /// Local name of the document element.
        element: String,
    },

    /// An XML Schema cannot be used.
    #[error(transparent)]
    XmlSchema(#[from] XmlSchemaError),

    /// A rule set cannot be used.
    #[error(transparent)]
    RuleSet(#[from] RuleSetError),

    /// The rules could not be evaluated on the document.
    #[error(transparent)]
    Evaluation(#[from] DocumentError),
}
