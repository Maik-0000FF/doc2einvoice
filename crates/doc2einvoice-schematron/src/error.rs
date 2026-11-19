// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors of loading a rule file and of reading a document.

/// Why a rule file could not be turned into a [`crate::Schema`].
///
/// Every variant means the rule file as a whole is unusable. A schema is never
/// compiled partially: a rule that silently went missing would let invalid
/// documents pass.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    /// The rule file is not well-formed XML.
    #[error("the rule file is not well-formed XML: {0}")]
    Xml(#[from] roxmltree::Error),

    /// The document element is not a Schematron `schema`.
    #[error("the rule file is not a Schematron schema")]
    NotASchema,

    /// An `include` names a file the resolver does not know.
    #[error("the included file `{href}` is not available")]
    UnresolvedInclude {
        /// The `href` of the `include`.
        href: String,
    },

    /// A file includes itself, directly or through other files.
    #[error("the included file `{href}` includes itself")]
    IncludeCycle {
        /// The `href` that closes the cycle.
        href: String,
    },

    /// The rule file contains an element the interpreter does not implement.
    /// Ignoring it could drop a rule, so the whole file is refused.
    #[error("the element `{element}` inside `{parent}` is not supported")]
    UnsupportedElement {
        /// Local name of the element.
        element: String,
        /// Local name of the element that contains it.
        parent: String,
    },

    /// An attribute asks for behaviour the interpreter does not implement.
    #[error("the attribute `{attribute}=\"{value}\"` of `{element}` is not supported")]
    UnsupportedAttribute {
        /// Local name of the element.
        element: &'static str,
        /// Name of the attribute.
        attribute: &'static str,
        /// The value that is not supported.
        value: String,
    },

    /// Two variables of the same name are declared where only one can be
    /// meant: on the schema and its patterns, within one rule, or in a rule
    /// under the name of a variable of the schema or of a pattern.
    #[error("the variable `{name}` is declared more than once")]
    DuplicateVariable {
        /// The name declared twice.
        name: String,
    },

    /// An element lacks an attribute the interpreter cannot work without.
    #[error("the element `{element}` lacks the attribute `{attribute}`")]
    MissingAttribute {
        /// Local name of the element.
        element: &'static str,
        /// Name of the missing attribute.
        attribute: &'static str,
    },

    /// A user-defined function contains an instruction outside the supported
    /// subset.
    #[error("the function `{function}` uses the unsupported instruction `{instruction}`")]
    UnsupportedInstruction {
        /// Name of the function as called in the rules.
        function: String,
        /// Local name of the instruction.
        instruction: String,
    },

    /// An expression could not be compiled.
    #[error("{subject}: the expression cannot be compiled: {reason}")]
    Expression {
        /// What the expression belongs to, for example an assertion id.
        subject: String,
        /// The message of the XPath engine.
        reason: String,
    },
}

/// Why a document could not be validated at all.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// The document is not well-formed XML.
    #[error("the document is not well-formed XML: {0}")]
    Xml(String),

    /// The XPath engine failed in a way it could not report as an error.
    #[error("the XPath engine failed while evaluating the rules")]
    EngineFailure,
}
