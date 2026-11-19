// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! A Schematron interpreter.
//!
//! It evaluates a Schematron rule file as published, without compiling it to
//! XSLT and without an XSLT processor. The crate knows nothing about invoices:
//! which rule files exist and what their findings mean is decided by its
//! callers.
//!
//! Supported is the subset the European e-invoicing rule files use:
//!
//! - `ns`, `pattern`, `rule` and `assert`, with a query binding of XPath 2.0
//!   or later
//! - `include`
//! - `let` on schema, pattern and rule level
//! - `name` and `value-of` in the text of an assertion
//! - user-defined XSLT functions built from `param`, `variable`, `sequence`
//!   and `choose`
//!
//! A rule file that uses anything else is refused as a whole (for example
//! `report`, abstract patterns or a default phase): a construct that was
//! skipped could be a rule that is never checked.
//!
//! Two deliberate rules of evaluation, both needed to agree with the reference
//! implementations:
//!
//! - Within one pattern a node is checked by the first rule whose context
//!   matches it, and by no other.
//! - An assertion that cannot be evaluated counts as violated. It is reported
//!   as [`Outcome::EvaluationError`], never silently passed. That holds as well
//!   for an assertion that reads a variable which could not be evaluated, and
//!   for every assertion of a rule whose context could not be evaluated.
//!
//! ```
//! use doc2einvoice_schematron::{NoIncludes, Schema};
//!
//! let rules = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
//!   <pattern>
//!     <rule context="/invoice">
//!       <assert id="R-1" flag="fatal" test="total">An invoice has a total.</assert>
//!     </rule>
//!   </pattern>
//! </schema>"#;
//!
//! let schema = Schema::compile(rules, &NoIncludes)?;
//! let report = schema.validate("<invoice/>")?;
//!
//! assert_eq!(report.findings().len(), 1);
//! assert_eq!(report.findings()[0].assertion_id, "R-1");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod engine;
mod error;
mod expression;
mod functions;
mod include;
mod location;
mod report;
mod schema;

pub use engine::Schema;
pub use error::{DocumentError, SchemaError};
pub use include::{IncludeResolver, IncludedFiles, NoIncludes};
pub use report::{Finding, Outcome, Report};
