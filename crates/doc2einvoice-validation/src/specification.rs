// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The specification an invoice says it follows, and the profile that
//! statement suggests.
//!
//! EN 16931 calls this statement the specification identifier (BT-24). It is
//! what the invoice claims, nothing more: only validation shows whether the
//! claim holds.

use crate::error::ValidationError;
use crate::nesting;
use crate::profile::Profile;
use crate::syntax::Syntax;

const UBL_BASIC_NAMESPACE: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
const UBL_IDENTIFIER_ELEMENT: &str = "CustomizationID";

const CII_DOCUMENT_NAMESPACE: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
const CII_ENTITY_NAMESPACE: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
const CII_CONTEXT_ELEMENT: &str = "ExchangedDocumentContext";
const CII_GUIDELINE_ELEMENT: &str = "GuidelineSpecifiedDocumentContextParameter";
const CII_IDENTIFIER_ELEMENT: &str = "ID";

/// The specification identifier of EN 16931 itself, without anything added.
const EN16931_IDENTIFIER: &str = "urn:cen.eu:en16931:2017";

/// What every specification identifier of an XRechnung contains, in lower
/// case: the publisher names its specification `xrechnung` as one part of a
/// URN, in every version and for its extensions.
const XRECHNUNG_MARKER: &str = ":xrechnung";

/// Reads the specification identifier (BT-24) of an invoice.
///
/// | Document                                             | Result              |
/// |------------------------------------------------------|---------------------|
/// | UBL, `CustomizationID` below the document element    | its text, trimmed   |
/// | CII, `ID` of the guideline in the document context   | its text, trimmed   |
/// | no such element                                      | none                |
/// | the element is empty or holds white space only       | none                |
/// | a comment or a processing instruction in the text    | the text around it, |
/// |                                                      | joined              |
/// | a comment in front of the text                       | the text            |
/// | the text or a part of it in a CDATA section          | the text            |
/// | an element inside the element                        | its text as well    |
/// | several such elements                                | the first one       |
/// | an element of that name deeper in the document       | none: not BT-24     |
/// | an element of that name in another namespace         | none                |
/// | not well-formed XML                                  | an error            |
/// | nested deeper than is read                           | an error            |
/// | not a supported invoice                              | an error            |
///
/// What is read is the string value of the element, as the rules and the XML
/// Schema see it: all the text inside it, whatever markup lies in between. If
/// only a part were read, an invoice could name one specification to the
/// rules and another one here.
///
/// Several identifiers and elements inside an identifier are not allowed; the
/// XML Schema step reports them. This function only reads what the invoice
/// says.
///
/// # Errors
///
/// Returns [`ValidationError::TooDeeplyNested`] if the document is nested
/// deeper than is read, [`ValidationError::Malformed`] if it is not
/// well-formed XML, and [`ValidationError::UnsupportedDocument`] if its
/// document element is none of the supported invoices.
pub fn specification_identifier(document: &str) -> Result<Option<String>, ValidationError> {
    let parsed = nesting::parse(document)?;
    let root = parsed.root_element();

    let element = match Syntax::of_document_element(root)? {
        Syntax::Ubl => child(root, UBL_BASIC_NAMESPACE, UBL_IDENTIFIER_ELEMENT),
        Syntax::Cii => child(root, CII_DOCUMENT_NAMESPACE, CII_CONTEXT_ELEMENT)
            .and_then(|context| child(context, CII_ENTITY_NAMESPACE, CII_GUIDELINE_ELEMENT))
            .and_then(|guideline| child(guideline, CII_ENTITY_NAMESPACE, CII_IDENTIFIER_ELEMENT)),
    };

    Ok(element
        .map(string_value)
        .map(|value| value.trim().to_owned())
        .filter(|identifier| !identifier.is_empty()))
}

/// The string value of an element as XPath defines it: the text of all the
/// text nodes inside it, in document order.
fn string_value(element: roxmltree::Node<'_, '_>) -> String {
    element
        .descendants()
        .filter(roxmltree::Node::is_text)
        .filter_map(|node| node.text())
        .collect()
}

/// The first child element of the given name.
fn child<'a, 'input>(
    parent: roxmltree::Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    parent
        .children()
        .find(|node| node.is_element() && node.has_tag_name((namespace, name)))
}

/// Whether the rules of the specification an invoice names were applied when
/// it was validated.
///
/// A verdict only covers the rules that were applied. If the invoice names a
/// specification whose rules were not among them, it was checked against less
/// than it claims, and the verdict must say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleCoverage {
    /// The invoice names no specification.
    NotNamed,
    /// The rules of the named specification were applied.
    Applied,
    /// The rules of the named specification were not applied: this crate does
    /// not include them, or the invoice was validated as another profile.
    NotApplied,
}

impl RuleCoverage {
    /// The coverage of an invoice that names the given specification and was
    /// validated as the given profile.
    ///
    /// | Identifier                                  | Validated as | Coverage    |
    /// |---------------------------------------------|--------------|-------------|
    /// | none                                        | any          | not named   |
    /// | `urn:cen.eu:en16931:2017`                   | EN 16931     | applied     |
    /// | `urn:cen.eu:en16931:2017`                   | XRechnung    | applied:    |
    /// |                                             |              | XRechnung   |
    /// |                                             |              | contains    |
    /// |                                             |              | the rules   |
    /// |                                             |              | of EN 16931 |
    /// | an XRechnung, in any version or spelling    | XRechnung    | applied:    |
    /// |                                             |              | its rules   |
    /// |                                             |              | judge the   |
    /// |                                             |              | identifier  |
    /// | an XRechnung                                | EN 16931     | not applied |
    /// | a profile of Factur-X, Peppol BIS Billing   | any          | not applied |
    /// | `URN:CEN.EU:EN16931:2017`, wrong case       | any          | not applied |
    /// | `urn:cen.eu:en16931:2017#`, anything added  | any          | not applied |
    /// | `123`, any other text                       | any          | not applied |
    ///
    /// Identifiers are compared exactly. What is not known to be covered counts
    /// as not applied: saying too often that rules are missing is harmless,
    /// saying it too rarely lets a verdict claim more than was checked.
    #[must_use]
    pub fn of(identifier: Option<&str>, validated_as: Profile) -> Self {
        let Some(identifier) = identifier else {
            return Self::NotNamed;
        };
        // The profile whose rules are the rules of the named specification, if
        // this crate has such a profile.
        let named = if identifier == EN16931_IDENTIFIER {
            Some(Profile::En16931)
        } else if Profile::suggested_for(Some(identifier)) == Profile::XRechnung {
            Some(Profile::XRechnung)
        } else {
            None
        };
        match named {
            Some(named) if validated_as.applies_the_rules_of(named) => Self::Applied,
            _ => Self::NotApplied,
        }
    }
}

impl Profile {
    /// The profile a specification identifier suggests validating as.
    ///
    /// | Identifier                                           | Profile            |
    /// |------------------------------------------------------|--------------------|
    /// | none                                                 | EN 16931           |
    /// | `urn:cen.eu:en16931:2017`                            | EN 16931           |
    /// | `…#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0`    | XRechnung          |
    /// | the same with an extension appended                  | XRechnung          |
    /// | `urn:…:kosit:standard:xrechnung_2.3`, an old version | XRechnung          |
    /// | `…:XRECHNUNG_3.0`, wrong case                        | XRechnung          |
    /// | `…#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0` | EN 16931         |
    /// | `xrechnung`, the bare word                           | EN 16931: no URN   |
    /// | `123`, any other text                                | EN 16931           |
    ///
    /// An invoice that names XRechnung in any version or spelling is validated
    /// as XRechnung: the rules then report an identifier they do not accept.
    /// Validating it as plain EN 16931 instead would hide that it fails the
    /// specification it claims.
    #[must_use]
    pub fn suggested_for(identifier: Option<&str>) -> Self {
        match identifier {
            Some(identifier) if identifier.to_ascii_lowercase().contains(XRECHNUNG_MARKER) => {
                Self::XRechnung
            }
            _ => Self::En16931,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RuleCoverage, specification_identifier};
    use crate::error::ValidationError;
    use crate::profile::Profile;

    const XRECHNUNG_3: &str =
        "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0";

    fn ubl(content: &str) -> String {
        format!(
            r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
                xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
                xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
                xmlns:other="urn:example:other">{content}</Invoice>"#
        )
    }

    fn cii(content: &str) -> String {
        format!(
            r#"<rsm:CrossIndustryInvoice
                xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"
                xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100"
                >{content}</rsm:CrossIndustryInvoice>"#
        )
    }

    fn identifier(document: &str) -> Result<Option<String>, ValidationError> {
        specification_identifier(document)
    }

    #[test]
    fn reads_the_identifier_of_a_ubl_invoice() -> Result<(), ValidationError> {
        let document = ubl(&format!(
            "<cbc:CustomizationID>\n  {XRECHNUNG_3} </cbc:CustomizationID>"
        ));

        assert_eq!(identifier(&document)?.as_deref(), Some(XRECHNUNG_3));
        Ok(())
    }

    #[test]
    fn reads_the_identifier_of_a_cii_invoice() -> Result<(), ValidationError> {
        let document = cii(&format!(
            "<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter>\
             <ram:ID>{XRECHNUNG_3}</ram:ID>\
             </ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>"
        ));

        assert_eq!(identifier(&document)?.as_deref(), Some(XRECHNUNG_3));
        Ok(())
    }

    #[test]
    fn an_invoice_without_the_element_states_none() -> Result<(), ValidationError> {
        assert_eq!(identifier(&ubl(""))?, None);
        assert_eq!(identifier(&cii(""))?, None);
        Ok(())
    }

    #[test]
    fn an_empty_identifier_is_none() -> Result<(), ValidationError> {
        for content in [
            "<cbc:CustomizationID/>",
            "<cbc:CustomizationID></cbc:CustomizationID>",
            "<cbc:CustomizationID> \n </cbc:CustomizationID>",
        ] {
            assert_eq!(identifier(&ubl(content))?, None, "{content}");
        }
        Ok(())
    }

    #[test]
    fn markup_inside_the_identifier_does_not_cut_it_short() -> Result<(), ValidationError> {
        const HEAD: &str = "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit";
        const TAIL: &str = ":xrechnung_3.0";

        for content in [
            format!("{HEAD}<!--c-->{TAIL}"),
            format!("<!--c-->{HEAD}{TAIL}"),
            format!("{HEAD}{TAIL}<!--c-->"),
            format!("{HEAD}<?p i?>{TAIL}"),
            format!("{HEAD}<![CDATA[{TAIL}]]>"),
            format!("<![CDATA[{HEAD}{TAIL}]]>"),
            format!("{HEAD}<cbc:Note>{TAIL}</cbc:Note>"),
            format!(" <!--c--> {HEAD}<!--c-->{TAIL} <!--c--> "),
        ] {
            let document = ubl(&format!(
                "<cbc:CustomizationID>{content}</cbc:CustomizationID>"
            ));
            let read = identifier(&document)?;

            assert_eq!(read.as_deref(), Some(XRECHNUNG_3), "{content}");
            assert_eq!(Profile::suggested_for(read.as_deref()), Profile::XRechnung);
        }
        Ok(())
    }

    #[test]
    fn markup_alone_is_no_identifier() -> Result<(), ValidationError> {
        for content in ["<!--c-->", "<?p i?>", " <!--c--> ", "<cbc:Note/>"] {
            let document = ubl(&format!(
                "<cbc:CustomizationID>{content}</cbc:CustomizationID>"
            ));

            assert_eq!(identifier(&document)?, None, "{content}");
        }
        Ok(())
    }

    #[test]
    fn the_first_of_several_identifiers_is_read() -> Result<(), ValidationError> {
        let document = ubl("<cbc:CustomizationID>first</cbc:CustomizationID>\
             <cbc:CustomizationID>second</cbc:CustomizationID>");

        assert_eq!(identifier(&document)?.as_deref(), Some("first"));
        Ok(())
    }

    #[test]
    fn an_element_of_that_name_elsewhere_is_not_the_identifier() -> Result<(), ValidationError> {
        let deeper = ubl(
            "<cac:InvoiceLine><cbc:CustomizationID>deeper</cbc:CustomizationID></cac:InvoiceLine>",
        );
        let other_namespace = ubl("<other:CustomizationID>other</other:CustomizationID>");
        let outside_the_guideline = cii(
            "<rsm:ExchangedDocumentContext><ram:BusinessProcessSpecifiedDocumentContextParameter>\
             <ram:ID>process</ram:ID>\
             </ram:BusinessProcessSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>",
        );

        assert_eq!(identifier(&deeper)?, None);
        assert_eq!(identifier(&other_namespace)?, None);
        assert_eq!(identifier(&outside_the_guideline)?, None);
        Ok(())
    }

    #[test]
    fn what_is_no_invoice_is_an_error() {
        assert!(matches!(
            identifier("<Invoice"),
            Err(ValidationError::Malformed(_))
        ));
        assert!(matches!(
            identifier("<Order/>"),
            Err(ValidationError::UnsupportedDocument { .. })
        ));
    }

    #[test]
    fn rules_count_as_applied_only_if_they_were() {
        const STANDARD: &str = "urn:cen.eu:en16931:2017";
        const EXTENDED: &str = "urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended";
        const PEPPOL: &str =
            "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0";
        const OLD_XRECHNUNG: &str =
            "urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_2.3";

        for (identifier, validated_as, expected) in [
            (None, Profile::En16931, RuleCoverage::NotNamed),
            (None, Profile::XRechnung, RuleCoverage::NotNamed),
            (Some(STANDARD), Profile::En16931, RuleCoverage::Applied),
            (Some(STANDARD), Profile::XRechnung, RuleCoverage::Applied),
            (Some(XRECHNUNG_3), Profile::XRechnung, RuleCoverage::Applied),
            (
                Some(OLD_XRECHNUNG),
                Profile::XRechnung,
                RuleCoverage::Applied,
            ),
            (
                Some(XRECHNUNG_3),
                Profile::En16931,
                RuleCoverage::NotApplied,
            ),
            (Some(EXTENDED), Profile::En16931, RuleCoverage::NotApplied),
            (Some(EXTENDED), Profile::XRechnung, RuleCoverage::NotApplied),
            (Some(PEPPOL), Profile::En16931, RuleCoverage::NotApplied),
            (
                Some("URN:CEN.EU:EN16931:2017"),
                Profile::En16931,
                RuleCoverage::NotApplied,
            ),
            (
                Some("urn:cen.eu:en16931:2017#"),
                Profile::En16931,
                RuleCoverage::NotApplied,
            ),
            (Some("123"), Profile::XRechnung, RuleCoverage::NotApplied),
        ] {
            assert_eq!(
                RuleCoverage::of(identifier, validated_as),
                expected,
                "{identifier:?} as {validated_as:?}"
            );
        }
    }

    #[test]
    fn the_identifier_suggests_the_profile() {
        let extension =
            format!("{XRECHNUNG_3}#conformant#urn:xeinkauf.de:kosit:extension:xrechnung_3.0");

        for (identifier, expected) in [
            (None, Profile::En16931),
            (Some("urn:cen.eu:en16931:2017"), Profile::En16931),
            (Some(XRECHNUNG_3), Profile::XRechnung),
            (Some(extension.as_str()), Profile::XRechnung),
            (
                Some("urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_2.3"),
                Profile::XRechnung,
            ),
            (
                Some("urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:XRECHNUNG_3.0"),
                Profile::XRechnung,
            ),
            (
                Some("urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0"),
                Profile::En16931,
            ),
            (Some("xrechnung"), Profile::En16931),
            (Some("123"), Profile::En16931),
        ] {
            assert_eq!(
                Profile::suggested_for(identifier),
                expected,
                "{identifier:?}"
            );
        }
    }
}
