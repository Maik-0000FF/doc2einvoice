// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recognising the XML syntax of an invoice.

use crate::error::ValidationError;

const UBL_INVOICE_NAMESPACE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
const UBL_INVOICE_ELEMENT: &str = "Invoice";
const UBL_CREDIT_NOTE_NAMESPACE: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
const UBL_CREDIT_NOTE_ELEMENT: &str = "CreditNote";
const CII_NAMESPACE: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
const CII_ELEMENT: &str = "CrossIndustryInvoice";

/// The XML syntax an invoice is written in.
///
/// EN 16931 defines one semantic model and two syntaxes that carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Syntax {
    /// OASIS Universal Business Language 2.1: an `Invoice` or a `CreditNote`.
    Ubl,
    /// UN/CEFACT Cross Industry Invoice D16B.
    Cii,
}

impl Syntax {
    /// Every syntax, the one place they are listed. What handles each syntax
    /// iterates over this, so that a new one cannot be left out unnoticed.
    pub const ALL: [Self; 2] = [Self::Ubl, Self::Cii];

    /// Recognises the syntax of a document by its document element.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::Malformed`] if the document is not
    /// well-formed XML, and [`ValidationError::UnsupportedDocument`] if its
    /// document element is none of the supported invoices.
    pub fn of(document: &str) -> Result<Self, ValidationError> {
        let parsed = roxmltree::Document::parse(document)
            .map_err(|error| ValidationError::Malformed(error.to_string()))?;
        Self::of_document_element(parsed.root_element())
    }

    /// Recognises the syntax by the document element of a parsed document.
    pub(crate) fn of_document_element(
        element: roxmltree::Node<'_, '_>,
    ) -> Result<Self, ValidationError> {
        let root = element.tag_name();

        match (root.namespace(), root.name()) {
            (Some(UBL_INVOICE_NAMESPACE), UBL_INVOICE_ELEMENT)
            | (Some(UBL_CREDIT_NOTE_NAMESPACE), UBL_CREDIT_NOTE_ELEMENT) => Ok(Self::Ubl),
            (Some(CII_NAMESPACE), CII_ELEMENT) => Ok(Self::Cii),
            (_, element) => Err(ValidationError::UnsupportedDocument {
                element: element.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Syntax;
    use crate::error::ValidationError;

    // Input table for `Syntax::of`:
    //
    // | document element                         | result               |
    // |------------------------------------------|----------------------|
    // | UBL Invoice in its namespace             | Ubl                  |
    // | UBL CreditNote in its namespace          | Ubl                  |
    // | CII CrossIndustryInvoice in its namespace| Cii                  |
    // | right name, wrong or missing namespace   | UnsupportedDocument  |
    // | right namespace, wrong name              | UnsupportedDocument  |
    // | any other element                        | UnsupportedDocument  |
    // | not well-formed, empty                   | Malformed            |

    #[test]
    fn recognises_the_supported_document_elements() -> Result<(), ValidationError> {
        assert_eq!(
            Syntax::of(
                r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#
            )?,
            Syntax::Ubl
        );
        assert_eq!(
            Syntax::of(
                r#"<c:CreditNote xmlns:c="urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2"/>"#
            )?,
            Syntax::Ubl
        );
        assert_eq!(
            Syntax::of(
                r#"<r:CrossIndustryInvoice xmlns:r="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"/>"#
            )?,
            Syntax::Cii
        );
        Ok(())
    }

    #[test]
    fn refuses_other_document_elements() {
        for document in [
            "<Invoice/>",
            r#"<Invoice xmlns="urn:example:other"/>"#,
            r#"<Order xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#,
            r#"<CreditNote xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"/>"#,
            "<html/>",
        ] {
            assert!(
                matches!(
                    Syntax::of(document),
                    Err(ValidationError::UnsupportedDocument { .. })
                ),
                "`{document}` must be refused"
            );
        }
    }

    #[test]
    fn refuses_what_is_not_well_formed() {
        for document in ["", "<Invoice", "plain text"] {
            assert!(
                matches!(Syntax::of(document), Err(ValidationError::Malformed(_))),
                "`{document}` must be refused"
            );
        }
    }
}
