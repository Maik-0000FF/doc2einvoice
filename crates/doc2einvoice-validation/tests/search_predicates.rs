// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The predicates the embedded rule files put on a step after a search
//! through all descendants, as in `//name[predicate]`.
//!
//! The XPath engine evaluates such a search as one descent where no predicate
//! of the step can depend on a position. It decides that itself, on the shape
//! of the predicate. This test is the second look: every predicate the rule
//! files use there is listed below, and each was read by a person before it
//! was listed.
//!
//! A predicate belongs on the list if it only keeps or drops a node, whatever
//! the position of the node among its siblings:
//!
//! - its value is never a number: `name[2]` and `name[count(a)]` select by
//!   position;
//! - nothing in it asks for a position or a size, neither `position()` nor
//!   `last()`, nor a function that hands them out.
//!
//! Position and size inside a further predicate, as in `name[a[1] = 'x']`,
//! are no concern: they have a focus of their own.
//!
//! The test fails when a rule file is updated and brings a predicate that is
//! on neither list below, and when a listed predicate is no longer used. A
//! new predicate is read and then put on one of the two lists:
//!
//! - [`REVIEWED`] if it meets the conditions above.
//! - [`DEPENDS_ON_POSITION`] if it does not. Such a search must be left as
//!   it is written, and the engine has to be shown to do so: before the
//!   predicate is listed, a search with a predicate of the same shape is added
//!   to the tests of the search in the interpreter, where a search and its
//!   written out form must select the same nodes.
//!
//! A predicate never goes on `REVIEWED` to make the test pass. That list is a
//! statement about every entry on it.
//!
//! The predicates come from the parsed expressions, with white space
//! collapsed, so the list does not depend on how a rule file is laid out.

use std::collections::BTreeSet;

use doc2einvoice_validation::RuleSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Every predicate that was read and found not to depend on a position, in
/// the order the test reports them.
const REVIEWED: [&str; 69] = [
    // A variable that holds a boolean. The engine does not look into a
    // variable and leaves this search as written.
    r"$isExtension",
    r"@currencyID",
    r"@format = '102'",
    r"@listID",
    r"@mimeCode",
    r"@schemeID",
    r"@schemeID and $isExtension",
    r"@unitCode",
    r"cac:InvoicePeriod/cbc:EndDate",
    r"cac:InvoicePeriod/cbc:StartDate",
    r"cac:Item/cac:ClassifiedTaxCategory/normalize-space(cbc:ID) = 'S'",
    r"cac:Item/cac:ClassifiedTaxCategory/xs:decimal(cbc:Percent) =$rate",
    r"cac:TaxCategory/normalize-space(cbc:ID)='S'",
    r"cac:TaxCategory/xs:decimal(cbc:Percent) = $rate",
    r"cac:TaxScheme/normalize-space(upper-case(cbc:ID))='VAT'",
    r"cbc:ChargeIndicator = false()",
    r"cbc:ChargeIndicator = true()",
    r"cbc:ChargeIndicator=false()",
    r"cbc:ChargeIndicator=true()",
    r"cbc:DocumentTypeCode != '130' or not(cbc:DocumentTypeCode)",
    r"cbc:DocumentTypeCode = '130'",
    r"cbc:PaymentMeansCode='30' or cbc:PaymentMeansCode='58'",
    // `count` yields a number, but the predicate is the comparison.
    r"count(cac:SubInvoiceLine) > 0",
    r"count(cac:SubInvoiceLine) > 0 and xs:decimal(cbc:LineExtensionAmount) = sum(cac:SubInvoiceLine/xs:decimal(cbc:LineExtensionAmount))",
    r"ends-with(name(), 'Amount') and not (self::ram:TaxTotalAmount)",
    r"ends-with(name(), 'Amount') and not(ends-with(name(),'PriceAmount')) and not(ancestor::cac:Price/cac:AllowanceCharge)",
    r"ends-with(name(), 'BinaryObject')",
    r"ends-with(name(), 'DocumentContextParameter')",
    r"ends-with(name(), 'ID')",
    r"ends-with(name(), 'Quantity')",
    r"ends-with(name(), 'ReferencedDocument')",
    r"ends-with(name(), 'TradeTax')",
    r"normalize-space(cbc:ID) != 'O'",
    r"normalize-space(cbc:ID) = 'AE'",
    r"normalize-space(cbc:ID) = 'E'",
    r"normalize-space(cbc:ID) = 'G'",
    r"normalize-space(cbc:ID) = 'K'",
    r"normalize-space(cbc:ID) = 'L'",
    r"normalize-space(cbc:ID) = 'M'",
    r"normalize-space(cbc:ID) = 'O'",
    r"normalize-space(cbc:ID) = 'S'",
    r"normalize-space(cbc:ID) = 'Z'",
    r"not(*) and not(normalize-space(string(.)))",
    // `preceding-sibling` and `preceding` look at where a node stands in the
    // document, which a descent does not change; they do not count siblings.
    r"not(./ram:AttachmentBinaryObject/@filename = preceding-sibling::ram:AdditionalReferencedDocument/ram:AttachmentBinaryObject/@filename)",
    r"not(ancestor::ram:SpecifiedTaxRegistration)",
    r"not(ancestor::ram:SpecifiedTradeProduct) and not(ancestor::ram:ShipToTradeParty)",
    r"not(name() = 'ram:ApplicableHeaderTradeDelivery') and not(*) and not(normalize-space(string(.)))",
    r"not(preceding::cbc:PaymentMeansCode/. = .)",
    r"not(ram:CalculationPercent) and ram:BasisAmount",
    r"ram:ApplicableHeaderTradeSettlement/ram:BillingSpecifiedPeriod/ram:EndDateTime",
    r"ram:ApplicableHeaderTradeSettlement/ram:BillingSpecifiedPeriod/ram:StartDateTime",
    r"ram:CalculationPercent and not(ram:BasisAmount)",
    r"ram:CategoryCode != 'O'",
    r"ram:CategoryCode='AE'",
    r"ram:CategoryCode='E'",
    r"ram:CategoryCode='G'",
    r"ram:CategoryCode='K'",
    r"ram:CategoryCode='L'",
    r"ram:CategoryCode='M'",
    r"ram:CategoryCode='O'",
    r"ram:CategoryCode='S'",
    r"ram:CategoryCode='Z'",
    r"ram:ChargeIndicator/udt:Indicator = false()",
    r"ram:ChargeIndicator/udt:Indicator = true()",
    r"ram:ChargeIndicator/udt:Indicator=false()",
    r"ram:ChargeIndicator/udt:Indicator=true()",
    r"ram:TypeCode='30' or ram:TypeCode='58'",
    r"some $code in tokenize('49 59', '\s') satisfies normalize-space(cbc:PaymentMeansCode) = $code",
    r"some $code in tokenize('49 59', '\s') satisfies normalize-space(ram:TypeCode) = $code",
];

/// Every predicate that was read and found to depend on a position, with the
/// test of the interpreter that shows the engine leaves its search as
/// written. The rule files use none so far.
const DEPENDS_ON_POSITION: [&str; 0] = [];

/// The predicates every embedded rule file uses after a search.
fn used() -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut used = BTreeSet::new();
    for rule_set in RuleSet::ALL {
        used.extend(rule_set.compile()?.search_predicates()?);
    }
    Ok(used)
}

#[test]
fn every_predicate_after_a_search_was_reviewed() -> TestResult {
    let reviewed: BTreeSet<&str> = REVIEWED.into_iter().chain(DEPENDS_ON_POSITION).collect();
    let used = used()?;
    let not_reviewed: Vec<&str> = used
        .iter()
        .map(String::as_str)
        .filter(|predicate| !reviewed.contains(predicate))
        .collect();

    assert!(
        not_reviewed.is_empty(),
        "predicates after a search that are on neither list: {not_reviewed:#?}"
    );
    Ok(())
}

#[test]
fn the_lists_name_only_predicates_that_are_used() -> TestResult {
    let used = used()?;
    let not_used: Vec<&str> = REVIEWED
        .into_iter()
        .chain(DEPENDS_ON_POSITION)
        .filter(|predicate| !used.contains(*predicate))
        .collect();

    assert!(
        not_used.is_empty(),
        "listed predicates that no rule file uses any more: {not_used:#?}"
    );
    Ok(())
}

#[test]
fn no_predicate_is_listed_twice() {
    let distinct: BTreeSet<&str> = REVIEWED.into_iter().chain(DEPENDS_ON_POSITION).collect();

    assert_eq!(distinct.len(), REVIEWED.len() + DEPENDS_ON_POSITION.len());
}
