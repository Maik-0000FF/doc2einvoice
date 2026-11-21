// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Invoices other than the complete one: a credit note, a payment by card, an
//! invoice with only what is required, and each invoice a syntax cannot
//! express.

mod support;

use doc2einvoice_model::{Code, NonEmpty, PrecedingInvoice, UnitPriceAmount};
use doc2einvoice_syntax::{Gap, Syntax, UblDocument, WriteError, write_cii, write_ubl};
use doc2einvoice_validation::Profile;
use support::{
    EN16931, Payment, XRECHNUNG, complete, date, decimal, exempt, findings, minimal, paid_by_card,
};

/// A credit note (UNTDID 1001).
const CREDIT_NOTE: &str = "381";

fn assert_clean(document: &[u8], profile: Profile) {
    let found = findings(document, profile);
    assert!(
        found.is_empty(),
        "{} findings:\n{}\n\n{}",
        found.len(),
        found.join("\n"),
        String::from_utf8_lossy(document)
    );
}

fn ubl(invoice: &doc2einvoice_model::Invoice, document: UblDocument) -> Vec<u8> {
    write_ubl(invoice, document).unwrap_or_else(|error| unreachable!("{error}"))
}

fn cii(invoice: &doc2einvoice_model::Invoice) -> Vec<u8> {
    write_cii(invoice).unwrap_or_else(|error| unreachable!("{error}"))
}

fn credit_note(specification: &str) -> doc2einvoice_model::Invoice {
    let mut invoice = complete(Payment::CreditTransfer, specification);
    invoice.type_code = Code(CREDIT_NOTE.to_owned());
    invoice
}

#[test]
fn a_credit_note_conforms_in_ubl() {
    assert_clean(
        &ubl(&credit_note(EN16931), UblDocument::CreditNote),
        Profile::En16931,
    );
    assert_clean(
        &ubl(&credit_note(XRECHNUNG), UblDocument::CreditNote),
        Profile::XRechnung,
    );
}

#[test]
fn a_credit_note_conforms_in_cii() {
    assert_clean(&cii(&credit_note(EN16931)), Profile::En16931);
    assert_clean(&cii(&credit_note(XRECHNUNG)), Profile::XRechnung);
}

#[test]
fn a_payment_by_card_conforms_in_cii() {
    assert_clean(&cii(&paid_by_card(EN16931)), Profile::En16931);
    assert_clean(&cii(&paid_by_card(XRECHNUNG)), Profile::XRechnung);
}

#[test]
fn an_invoice_exempt_from_vat_conforms() {
    assert_clean(&ubl(&exempt(), UblDocument::Invoice), Profile::En16931);
    assert_clean(&cii(&exempt()), Profile::En16931);
}

#[test]
fn an_invoice_with_only_what_is_required_conforms() {
    assert_clean(&ubl(&minimal(), UblDocument::Invoice), Profile::En16931);
    assert_clean(&cii(&minimal()), Profile::En16931);
}

/// Changes the one line of `invoice`; the list of lines is rebuilt, since it
/// can only be read.
fn change_line(
    invoice: &mut doc2einvoice_model::Invoice,
    change: impl FnOnce(&mut doc2einvoice_model::InvoiceLine),
) {
    let mut line = invoice.lines.first().clone();
    change(&mut line);
    invoice.lines = NonEmpty::one(line);
}

fn gap_of(result: Result<Vec<u8>, WriteError>) -> Option<(Syntax, Gap)> {
    match result.err()? {
        WriteError::NotExpressible { syntax, gap } => Some((syntax, gap)),
        WriteError::Xml(_) => None,
    }
}

#[test]
fn ubl_refuses_what_it_cannot_express() {
    let ubl =
        |invoice: &doc2einvoice_model::Invoice, document| gap_of(write_ubl(invoice, document));
    let refused = |gap| Some((Syntax::Ubl, gap));

    let mut invoice = minimal();
    invoice.sales_order_reference = Some("AUFTRAG-1".to_owned());
    assert_eq!(
        ubl(&invoice, UblDocument::Invoice),
        refused(Gap::SalesOrderWithoutPurchaseOrder)
    );

    let mut invoice = minimal();
    invoice.totals.vat_total = None;
    assert_eq!(
        ubl(&invoice, UblDocument::Invoice),
        refused(Gap::VatTotalMissing)
    );

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    invoice.vat_accounting_currency = None;
    assert_eq!(
        ubl(&invoice, UblDocument::Invoice),
        refused(Gap::AccountingCurrencyMissing)
    );

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    change_line(&mut invoice, |line| {
        line.price.discount = None;
    });
    assert_eq!(
        ubl(&invoice, UblDocument::Invoice),
        refused(Gap::GrossPriceWithoutDiscount)
    );

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    change_line(&mut invoice, |line| {
        line.price.base_quantity = None;
    });
    assert_eq!(
        ubl(&invoice, UblDocument::Invoice),
        refused(Gap::BaseUnitWithoutBaseQuantity)
    );

    assert_eq!(
        ubl(&paid_by_card(EN16931), UblDocument::Invoice),
        refused(Gap::CardNetworkMissing)
    );

    let mut invoice = credit_note(EN16931);
    invoice.payment_instructions = None;
    assert_eq!(
        ubl(&invoice, UblDocument::CreditNote),
        refused(Gap::DueDateWithoutPaymentInstructions)
    );
}

#[test]
fn cii_refuses_what_it_cannot_express() {
    let refused = |gap| Some((Syntax::Cii, gap));

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    invoice.preceding_invoices.push(PrecedingInvoice {
        reference: "RE-0".to_owned(),
        issue_date: Some(date(2026, 1, 2)),
    });
    assert_eq!(
        gap_of(write_cii(&invoice)),
        refused(Gap::SeveralPrecedingInvoices)
    );

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    invoice.vat_accounting_currency = None;
    assert_eq!(
        gap_of(write_cii(&invoice)),
        refused(Gap::AccountingCurrencyMissing)
    );

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    change_line(&mut invoice, |line| {
        line.price.base_quantity = None;
    });
    assert_eq!(
        gap_of(write_cii(&invoice)),
        refused(Gap::BaseUnitWithoutBaseQuantity)
    );

    let mut invoice = complete(Payment::CreditTransfer, EN16931);
    change_line(&mut invoice, |line| {
        line.price.gross_price = None;
        line.price.discount = Some(UnitPriceAmount(decimal("5")));
    });
    assert_eq!(
        gap_of(write_cii(&invoice)),
        refused(Gap::DiscountWithoutGrossPrice)
    );
}
