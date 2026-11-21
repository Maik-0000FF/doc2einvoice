// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing an invoice as UBL 2.1, as an `Invoice` or a `CreditNote`.
//!
//! Each business term goes to the UBL element that holds it, in the order the
//! XML Schema of UBL 2.1 prescribes.

mod names;

use std::io;

use doc2einvoice_model::{
    Amount, Buyer, Contact, DocumentAdjustment, Invoice, InvoiceLine, ItemInformation,
    LineAdjustment, OptionalSchemeIdentifier, Payee, PaymentInstructions, Percentage, Period,
    PostalAddress, PriceDetails, Seller, SupportingDocument, TaxRepresentative, VatBreakdown,
    VatCategory, VatPoint,
};

use self::names as n;
use crate::error::{Gap, Syntax, WriteError};
use crate::format;
use crate::shared;
use crate::xml::{Attribute, Xml, optional_attribute};

/// Which UBL document an invoice is written as. UBL has a document of its own
/// for a credit note; the caller chooses it, from the invoice type (BT-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UblDocument {
    /// `Invoice`.
    Invoice,
    /// `CreditNote`.
    CreditNote,
}

impl UblDocument {
    fn root(self) -> (&'static str, &'static str) {
        match self {
            Self::Invoice => (n::INVOICE, n::INVOICE_NAMESPACE),
            Self::CreditNote => (n::CREDIT_NOTE, n::CREDIT_NOTE_NAMESPACE),
        }
    }

    fn line(self) -> (&'static str, &'static str) {
        match self {
            Self::Invoice => (n::INVOICE_LINE, n::INVOICED_QUANTITY),
            Self::CreditNote => (n::CREDIT_NOTE_LINE, n::CREDITED_QUANTITY),
        }
    }
}

/// Writes `invoice` as the UBL document `document`.
///
/// # Errors
///
/// [`WriteError::NotExpressible`] if UBL needs a value the invoice does not
/// have; nothing is written then.
pub fn write(invoice: &Invoice, document: UblDocument) -> Result<Vec<u8>, WriteError> {
    if let Some(gap) = gap(invoice, document) {
        return Err(WriteError::NotExpressible {
            syntax: Syntax::Ubl,
            gap,
        });
    }
    let mut xml = Xml::new()?;
    let (root, namespace) = document.root();
    let attributes = [
        (n::XMLNS, namespace),
        (n::XMLNS_CAC, n::CAC_NAMESPACE),
        (n::XMLNS_CBC, n::CBC_NAMESPACE),
    ];
    let writer = UblWriter {
        invoice,
        document,
        currency: invoice.currency.as_str(),
    };
    xml.required_element(root, &attributes, |xml| writer.document(xml))?;
    Ok(xml.finish())
}

/// The first thing UBL needs that `invoice` does not have, if any.
fn gap(invoice: &Invoice, document: UblDocument) -> Option<Gap> {
    if let Some(term) = shared::blank_required_value(invoice) {
        return Some(Gap::BlankRequiredValue(term));
    }
    let lines = || invoice.lines.iter();
    if invoice.sales_order_reference.is_some() && invoice.purchase_order_reference.is_none() {
        return Some(Gap::SalesOrderWithoutPurchaseOrder);
    }
    if invoice.totals.vat_total.is_none() {
        return Some(Gap::VatTotalMissing);
    }
    if invoice.totals.vat_total_in_accounting_currency.is_some()
        && invoice.vat_accounting_currency.is_none()
    {
        return Some(Gap::AccountingCurrencyMissing);
    }
    if lines().any(|line| line.price.gross_price.is_some() && line.price.discount.is_none()) {
        return Some(Gap::GrossPriceWithoutDiscount);
    }
    if lines()
        .any(|line| line.price.base_quantity_unit.is_some() && line.price.base_quantity.is_none())
    {
        return Some(Gap::BaseUnitWithoutBaseQuantity);
    }
    if invoice
        .payment_instructions
        .as_ref()
        .is_some_and(|payment| payment.card.is_some())
    {
        return Some(Gap::CardNetworkMissing);
    }
    if document == UblDocument::CreditNote
        && invoice.payment_due_date.is_some()
        && invoice.payment_instructions.is_none()
    {
        return Some(Gap::DueDateWithoutPaymentInstructions);
    }
    if invoice
        .notes
        .iter()
        .any(|note| note.subject_code.is_none() && reads_like_subject(&note.text))
    {
        return Some(Gap::NoteTextLikeSubject);
    }
    None
}

/// Whether `text` begins as a note with a subject is written, `#Subject#`:
/// a receiver would read a subject from it.
///
/// | Text                | Reads like a subject |
/// |---------------------|----------------------|
/// | `#AAI#Text`         | yes                  |
/// | `#3#`               | yes                  |
/// | `#3 Teillieferung`  | no, no second `#`    |
/// | `##Text`            | no, nothing between  |
/// | `Text #AAI#`        | no, not at the start |
/// | `#`                 | no                   |
fn reads_like_subject(text: &str) -> bool {
    text.strip_prefix(n::NOTE_SUBJECT_DELIMITER)
        .and_then(|rest| rest.find(n::NOTE_SUBJECT_DELIMITER))
        .is_some_and(|end| end > 0)
}

/// The attribute that names the scheme of an identifier, if it has one.
fn scheme(scheme: Option<&str>) -> Vec<Attribute<'_>> {
    optional_attribute(n::SCHEME_ID, scheme)
}

struct UblWriter<'a> {
    invoice: &'a Invoice,
    document: UblDocument,
    currency: &'a str,
}

impl UblWriter<'_> {
    fn document(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        let credit_note = self.document == UblDocument::CreditNote;
        xml.text(
            n::CUSTOMIZATION_ID,
            &[],
            &invoice.process_control.specification.0,
        )?;
        xml.optional_text(
            n::PROFILE_ID,
            invoice.process_control.business_process.as_deref(),
        )?;
        xml.text(n::ID, &[], &invoice.number.0)?;
        xml.text(n::ISSUE_DATE, &[], &format::ubl_date(invoice.issue_date))?;
        if credit_note {
            self.tax_point_date(xml)?;
            xml.text(n::CREDIT_NOTE_TYPE_CODE, &[], &invoice.type_code.0)?;
            self.notes(xml)?;
        } else {
            if let Some(due) = invoice.payment_due_date {
                xml.text(n::DUE_DATE, &[], &format::ubl_date(due))?;
            }
            xml.text(n::INVOICE_TYPE_CODE, &[], &invoice.type_code.0)?;
            self.notes(xml)?;
            self.tax_point_date(xml)?;
        }
        xml.text(n::DOCUMENT_CURRENCY_CODE, &[], self.currency)?;
        if let Some(currency) = &invoice.vat_accounting_currency {
            xml.text(n::TAX_CURRENCY_CODE, &[], currency.as_str())?;
        }
        xml.optional_text(
            n::ACCOUNTING_COST,
            invoice.buyer_accounting_reference.as_deref(),
        )?;
        xml.optional_text(n::BUYER_REFERENCE, invoice.buyer_reference.as_deref())?;
        self.invoice_period(xml)?;
        self.references(xml)?;
        xml.element(n::ACCOUNTING_SUPPLIER_PARTY, &[], |xml| {
            self.seller(xml, &invoice.seller)
        })?;
        xml.element(n::ACCOUNTING_CUSTOMER_PARTY, &[], |xml| {
            buyer(xml, &invoice.buyer)
        })?;
        if let Some(payee) = &invoice.payee {
            self.payee(xml, payee)?;
        }
        if let Some(representative) = &invoice.tax_representative {
            tax_representative(xml, representative)?;
        }
        self.delivery(xml)?;
        if let Some(payment) = &invoice.payment_instructions {
            self.payment_means(xml, payment)?;
        }
        if let Some(terms) = &invoice.payment_terms {
            xml.element(n::PAYMENT_TERMS, &[], |xml| xml.text(n::NOTE, &[], terms))?;
        }
        for allowance in &invoice.allowances {
            self.document_adjustment(xml, allowance, false)?;
        }
        for charge in &invoice.charges {
            self.document_adjustment(xml, charge, true)?;
        }
        self.tax_totals(xml)?;
        self.monetary_total(xml)?;
        for line in invoice.lines.iter() {
            self.line(xml, line)?;
        }
        Ok(())
    }

    /// The references to other documents, in the order of the document.
    fn references(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        let credit_note = self.document == UblDocument::CreditNote;
        if let Some(order) = &invoice.purchase_order_reference {
            xml.element(n::ORDER_REFERENCE, &[], |xml| {
                xml.text(n::ID, &[], order)?;
                xml.optional_text(n::SALES_ORDER_ID, invoice.sales_order_reference.as_deref())
            })?;
        }
        for preceding in &invoice.preceding_invoices {
            xml.element(n::BILLING_REFERENCE, &[], |xml| {
                xml.element(n::INVOICE_DOCUMENT_REFERENCE, &[], |xml| {
                    xml.text(n::ID, &[], &preceding.reference)?;
                    match preceding.issue_date {
                        Some(date) => xml.text(n::ISSUE_DATE, &[], &format::ubl_date(date)),
                        None => Ok(()),
                    }
                })
            })?;
        }
        reference(
            xml,
            n::DESPATCH_DOCUMENT_REFERENCE,
            invoice.despatch_advice_reference.as_deref(),
        )?;
        reference(
            xml,
            n::RECEIPT_DOCUMENT_REFERENCE,
            invoice.receiving_advice_reference.as_deref(),
        )?;
        if credit_note {
            reference(
                xml,
                n::CONTRACT_DOCUMENT_REFERENCE,
                invoice.contract_reference.as_deref(),
            )?;
            self.additional_references(xml)?;
            reference(
                xml,
                n::ORIGINATOR_DOCUMENT_REFERENCE,
                invoice.tender_or_lot_reference.as_deref(),
            )?;
        } else {
            reference(
                xml,
                n::ORIGINATOR_DOCUMENT_REFERENCE,
                invoice.tender_or_lot_reference.as_deref(),
            )?;
            reference(
                xml,
                n::CONTRACT_DOCUMENT_REFERENCE,
                invoice.contract_reference.as_deref(),
            )?;
            self.additional_references(xml)?;
            reference(
                xml,
                n::PROJECT_REFERENCE,
                invoice.project_reference.as_deref(),
            )?;
        }
        Ok(())
    }

    fn amount(&self, xml: &mut Xml, name: &str, amount: Amount) -> io::Result<()> {
        xml.text(
            name,
            &[(n::CURRENCY_ID, self.currency)],
            &format::number(amount.value()),
        )
    }

    fn optional_amount(&self, xml: &mut Xml, name: &str, amount: Option<Amount>) -> io::Result<()> {
        match amount {
            Some(amount) => self.amount(xml, name, amount),
            None => Ok(()),
        }
    }

    fn notes(&self, xml: &mut Xml) -> io::Result<()> {
        for note in &self.invoice.notes {
            let text = match &note.subject_code {
                Some(code) => format!(
                    "{d}{code}{d}{text}",
                    d = n::NOTE_SUBJECT_DELIMITER,
                    code = code.0,
                    text = note.text
                ),
                None => note.text.clone(),
            };
            xml.text(n::NOTE, &[], &text)?;
        }
        Ok(())
    }

    fn tax_point_date(&self, xml: &mut Xml) -> io::Result<()> {
        match &self.invoice.vat_point {
            Some(VatPoint::Date(date)) => {
                xml.text(n::TAX_POINT_DATE, &[], &format::ubl_date(*date))
            }
            _ => Ok(()),
        }
    }

    /// The invoicing period (BG-14) and the VAT point date code (BT-8), which
    /// UBL writes in the same element.
    fn invoice_period(&self, xml: &mut Xml) -> io::Result<()> {
        let period = self
            .invoice
            .delivery
            .as_ref()
            .and_then(|delivery| delivery.invoicing_period.as_ref());
        let code = match &self.invoice.vat_point {
            Some(VatPoint::Code(code)) => Some(code.0.as_str()),
            _ => None,
        };
        if period.is_none() && code.is_none() {
            return Ok(());
        }
        xml.element(n::INVOICE_PERIOD, &[], |xml| {
            if let Some(period) = period {
                period_dates(xml, period)?;
            }
            xml.optional_text(n::DESCRIPTION_CODE, code)
        })
    }

    /// The invoiced object (BT-18), the supporting documents (BG-24) and, in a
    /// credit note, the project (BT-11).
    fn additional_references(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        if let Some(object) = &invoice.invoiced_object {
            typed_reference(
                xml,
                n::ADDITIONAL_DOCUMENT_REFERENCE,
                object,
                n::INVOICED_OBJECT_TYPE,
            )?;
        }
        if self.document == UblDocument::CreditNote
            && let Some(project) = &invoice.project_reference
        {
            xml.element(n::ADDITIONAL_DOCUMENT_REFERENCE, &[], |xml| {
                xml.text(n::ID, &[], project)?;
                xml.text(n::DOCUMENT_TYPE_CODE, &[], n::PROJECT_TYPE)
            })?;
        }
        for supporting in &invoice.supporting_documents {
            supporting_document(xml, supporting)?;
        }
        Ok(())
    }

    fn seller(&self, xml: &mut Xml, seller: &Seller) -> io::Result<()> {
        let creditor = self.creditor_identifier(false);
        xml.element(n::PARTY, &[], |xml| {
            if let Some(address) = &seller.electronic_address {
                xml.text(
                    n::ENDPOINT_ID,
                    &[(n::SCHEME_ID, &address.scheme)],
                    &address.content,
                )?;
            }
            for identifier in &seller.identifiers {
                party_identification(xml, &identifier.content, identifier.scheme.as_deref())?;
            }
            if let Some(creditor) = creditor {
                party_identification(xml, creditor, Some(n::SEPA_SCHEME))?;
            }
            party_name(xml, seller.trading_name.as_deref())?;
            postal_address(xml, n::POSTAL_ADDRESS, &seller.address)?;
            tax_scheme(
                xml,
                seller.vat_identifier.as_ref().map(|id| id.0.as_str()),
                n::VAT_SCHEME,
            )?;
            tax_scheme(
                xml,
                seller.tax_registration.as_ref().map(|id| id.0.as_str()),
                n::LOCAL_TAX_SCHEME,
            )?;
            xml.element(n::PARTY_LEGAL_ENTITY, &[], |xml| {
                xml.text(n::REGISTRATION_NAME, &[], &seller.name)?;
                company_id(xml, seller.legal_registration.as_ref())?;
                xml.optional_text(
                    n::COMPANY_LEGAL_FORM,
                    seller.additional_legal_information.as_deref(),
                )
            })?;
            contact(xml, seller.contact.as_ref())
        })
    }

    fn payee(&self, xml: &mut Xml, payee: &Payee) -> io::Result<()> {
        let creditor = self.creditor_identifier(true);
        xml.element(n::PAYEE_PARTY, &[], |xml| {
            if let Some(identifier) = &payee.identifier {
                party_identification(xml, &identifier.content, identifier.scheme.as_deref())?;
            }
            if let Some(creditor) = creditor {
                party_identification(xml, creditor, Some(n::SEPA_SCHEME))?;
            }
            party_name(xml, Some(&payee.name))?;
            if let Some(registration) = &payee.legal_registration {
                xml.element(n::PARTY_LEGAL_ENTITY, &[], |xml| {
                    company_id(xml, Some(registration))
                })?;
            }
            Ok(())
        })
    }

    /// The SEPA creditor identifier (BT-90), which UBL writes as an
    /// identifier of the payee if there is one, otherwise of the seller.
    fn creditor_identifier(&self, for_payee: bool) -> Option<&str> {
        let invoice = self.invoice;
        if invoice.payee.is_some() != for_payee {
            return None;
        }
        invoice
            .payment_instructions
            .as_ref()
            .and_then(|payment| payment.direct_debit.as_ref())
            .and_then(|debit| debit.creditor_identifier.as_ref())
            .map(|identifier| identifier.0.as_str())
    }

    fn delivery(&self, xml: &mut Xml) -> io::Result<()> {
        let Some(delivery) = &self.invoice.delivery else {
            return Ok(());
        };
        if delivery.party_name.is_none()
            && delivery.location.is_none()
            && delivery.actual_date.is_none()
            && delivery.address.is_none()
        {
            return Ok(());
        }
        xml.element(n::DELIVERY, &[], |xml| {
            if let Some(date) = delivery.actual_date {
                xml.text(n::ACTUAL_DELIVERY_DATE, &[], &format::ubl_date(date))?;
            }
            if delivery.location.is_some() || delivery.address.is_some() {
                xml.element(n::DELIVERY_LOCATION, &[], |xml| {
                    if let Some(location) = &delivery.location {
                        xml.text(
                            n::ID,
                            &scheme(location.scheme.as_deref()),
                            &location.content,
                        )?;
                    }
                    match &delivery.address {
                        Some(address) => postal_address(xml, n::ADDRESS, address),
                        None => Ok(()),
                    }
                })?;
            }
            if let Some(name) = &delivery.party_name {
                xml.element(n::DELIVERY_PARTY, &[], |xml| party_name(xml, Some(name)))?;
            }
            Ok(())
        })
    }

    /// The payment instructions (BG-16): one `PaymentMeans` for each account
    /// to transfer to, or a single one if there is none.
    fn payment_means(&self, xml: &mut Xml, payment: &PaymentInstructions) -> io::Result<()> {
        let code_attributes = optional_attribute(n::NAME, payment.means_text.as_deref());
        for shared::PaymentMeans { account, first } in shared::payment_means(payment) {
            xml.element(n::PAYMENT_MEANS, &[], |xml| {
                xml.text(
                    n::PAYMENT_MEANS_CODE,
                    &code_attributes,
                    &payment.means_code.0,
                )?;
                if first
                    && self.document == UblDocument::CreditNote
                    && let Some(due) = self.invoice.payment_due_date
                {
                    xml.text(n::PAYMENT_DUE_DATE, &[], &format::ubl_date(due))?;
                }
                xml.optional_text(n::PAYMENT_ID, payment.remittance_information.as_deref())?;
                if let Some(account) = account {
                    xml.element(n::PAYEE_FINANCIAL_ACCOUNT, &[], |xml| {
                        xml.text(n::ID, &[], shared::account(account).value())?;
                        xml.optional_text(n::NAME_ELEMENT, account.account_name.as_deref())?;
                        match &account.service_provider {
                            Some(provider) => {
                                xml.element(n::FINANCIAL_INSTITUTION_BRANCH, &[], |xml| {
                                    xml.text(n::ID, &[], &provider.0)
                                })
                            }
                            None => Ok(()),
                        }
                    })?;
                }
                match (&payment.direct_debit, first) {
                    (Some(debit), true)
                        if debit.mandate_reference.is_some() || debit.debited_account.is_some() =>
                    {
                        xml.element(n::PAYMENT_MANDATE, &[], |xml| {
                            xml.optional_text(
                                n::ID,
                                debit.mandate_reference.as_ref().map(|id| id.0.as_str()),
                            )?;
                            match &debit.debited_account {
                                Some(account) => {
                                    xml.element(n::PAYER_FINANCIAL_ACCOUNT, &[], |xml| {
                                        xml.text(n::ID, &[], &account.0)
                                    })
                                }
                                None => Ok(()),
                            }
                        })
                    }
                    _ => Ok(()),
                }
            })?;
        }
        Ok(())
    }

    fn document_adjustment(
        &self,
        xml: &mut Xml,
        adjustment: &DocumentAdjustment,
        charge: bool,
    ) -> io::Result<()> {
        xml.element(n::ALLOWANCE_CHARGE, &[], |xml| {
            xml.text(
                n::CHARGE_INDICATOR,
                &[],
                if charge { n::TRUE } else { n::FALSE },
            )?;
            xml.optional_text(
                n::ALLOWANCE_CHARGE_REASON_CODE,
                adjustment.reason_code.as_ref().map(|code| code.0.as_str()),
            )?;
            xml.optional_text(n::ALLOWANCE_CHARGE_REASON, adjustment.reason.as_deref())?;
            percentage(xml, n::MULTIPLIER_FACTOR_NUMERIC, adjustment.percentage)?;
            self.amount(xml, n::AMOUNT, adjustment.amount)?;
            self.optional_amount(xml, n::BASE_AMOUNT, adjustment.base_amount)?;
            tax_category(
                xml,
                n::TAX_CATEGORY,
                adjustment.vat_category,
                adjustment.vat_rate,
                None,
                None,
            )
        })
    }

    /// The VAT totals, one `TaxTotal` each. The first is the total in the
    /// invoice currency, which UBL requires and the check for gaps ensures; it
    /// holds the breakdown.
    fn tax_totals(&self, xml: &mut Xml) -> io::Result<()> {
        for (index, (total, currency)) in shared::vat_totals(self.invoice).into_iter().enumerate() {
            xml.element(n::TAX_TOTAL, &[], |xml| {
                xml.text(
                    n::TAX_AMOUNT,
                    &[(n::CURRENCY_ID, currency)],
                    &format::number(total.value()),
                )?;
                if index > 0 {
                    return Ok(());
                }
                for breakdown in self.invoice.vat_breakdown.iter() {
                    self.vat_breakdown(xml, breakdown)?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    fn vat_breakdown(&self, xml: &mut Xml, breakdown: &VatBreakdown) -> io::Result<()> {
        xml.element(n::TAX_SUBTOTAL, &[], |xml| {
            self.amount(xml, n::TAXABLE_AMOUNT, breakdown.taxable_amount)?;
            self.amount(xml, n::TAX_AMOUNT, breakdown.tax_amount)?;
            tax_category(
                xml,
                n::TAX_CATEGORY,
                breakdown.category,
                breakdown.rate,
                breakdown
                    .exemption_reason_code
                    .as_ref()
                    .map(|code| code.0.as_str()),
                breakdown.exemption_reason.as_deref(),
            )
        })
    }

    fn monetary_total(&self, xml: &mut Xml) -> io::Result<()> {
        let totals = &self.invoice.totals;
        xml.element(n::LEGAL_MONETARY_TOTAL, &[], |xml| {
            self.amount(xml, n::LINE_EXTENSION_AMOUNT, totals.line_net_total)?;
            self.amount(xml, n::TAX_EXCLUSIVE_AMOUNT, totals.total_without_vat)?;
            self.amount(xml, n::TAX_INCLUSIVE_AMOUNT, totals.total_with_vat)?;
            self.optional_amount(xml, n::ALLOWANCE_TOTAL_AMOUNT, totals.allowance_total)?;
            self.optional_amount(xml, n::CHARGE_TOTAL_AMOUNT, totals.charge_total)?;
            self.optional_amount(xml, n::PREPAID_AMOUNT, totals.paid)?;
            self.optional_amount(xml, n::PAYABLE_ROUNDING_AMOUNT, totals.rounding)?;
            self.amount(xml, n::PAYABLE_AMOUNT, totals.due_for_payment)
        })
    }

    fn line(&self, xml: &mut Xml, line: &InvoiceLine) -> io::Result<()> {
        let (element, quantity) = self.document.line();
        xml.element(element, &[], |xml| {
            xml.text(n::ID, &[], &line.identifier.0)?;
            xml.optional_text(n::NOTE, line.note.as_deref())?;
            xml.text(
                quantity,
                &[(n::UNIT_CODE, &line.unit.0)],
                &format::number(line.quantity.0),
            )?;
            self.amount(xml, n::LINE_EXTENSION_AMOUNT, line.net_amount)?;
            xml.optional_text(
                n::ACCOUNTING_COST,
                line.buyer_accounting_reference.as_deref(),
            )?;
            if let Some(period) = &line.period {
                xml.element(n::INVOICE_PERIOD, &[], |xml| period_dates(xml, period))?;
            }
            if let Some(order_line) = &line.purchase_order_line {
                xml.element(n::ORDER_LINE_REFERENCE, &[], |xml| {
                    xml.text(n::LINE_ID, &[], order_line)
                })?;
            }
            if let Some(object) = &line.object_identifier {
                typed_reference(xml, n::DOCUMENT_REFERENCE, object, n::INVOICED_OBJECT_TYPE)?;
            }
            for allowance in &line.allowances {
                self.line_adjustment(xml, allowance, false)?;
            }
            for charge in &line.charges {
                self.line_adjustment(xml, charge, true)?;
            }
            item(xml, &line.item, line.vat.category, line.vat.rate)?;
            self.price(xml, &line.price)
        })
    }

    fn line_adjustment(
        &self,
        xml: &mut Xml,
        adjustment: &LineAdjustment,
        charge: bool,
    ) -> io::Result<()> {
        xml.element(n::ALLOWANCE_CHARGE, &[], |xml| {
            xml.text(
                n::CHARGE_INDICATOR,
                &[],
                if charge { n::TRUE } else { n::FALSE },
            )?;
            xml.optional_text(
                n::ALLOWANCE_CHARGE_REASON_CODE,
                adjustment.reason_code.as_ref().map(|code| code.0.as_str()),
            )?;
            xml.optional_text(n::ALLOWANCE_CHARGE_REASON, adjustment.reason.as_deref())?;
            percentage(xml, n::MULTIPLIER_FACTOR_NUMERIC, adjustment.percentage)?;
            self.amount(xml, n::AMOUNT, adjustment.amount)?;
            self.optional_amount(xml, n::BASE_AMOUNT, adjustment.base_amount)
        })
    }

    fn price(&self, xml: &mut Xml, price: &PriceDetails) -> io::Result<()> {
        let currency = [(n::CURRENCY_ID, self.currency)];
        xml.element(n::PRICE, &[], |xml| {
            xml.text(
                n::PRICE_AMOUNT,
                &currency,
                &format::number(price.net_price.0),
            )?;
            if let Some(base) = price.base_quantity {
                let unit = optional_attribute(
                    n::UNIT_CODE,
                    price
                        .base_quantity_unit
                        .as_ref()
                        .map(|unit| unit.0.as_str()),
                );
                xml.text(n::BASE_QUANTITY, &unit, &format::number(base.0))?;
            }
            if let Some(discount) = price.discount {
                xml.element(n::ALLOWANCE_CHARGE, &[], |xml| {
                    xml.text(n::CHARGE_INDICATOR, &[], n::FALSE)?;
                    xml.text(n::AMOUNT, &currency, &format::number(discount.0))?;
                    match price.gross_price {
                        Some(gross) => {
                            xml.text(n::BASE_AMOUNT, &currency, &format::number(gross.0))
                        }
                        None => Ok(()),
                    }
                })?;
            }
            Ok(())
        })
    }
}

/// A document reference that holds only its identifier.
fn reference(xml: &mut Xml, name: &str, identifier: Option<&str>) -> io::Result<()> {
    match identifier {
        Some(identifier) => xml.element(name, &[], |xml| xml.text(n::ID, &[], identifier)),
        None => Ok(()),
    }
}

/// A document reference to an object, marked by its document type code.
fn typed_reference(
    xml: &mut Xml,
    name: &str,
    object: &OptionalSchemeIdentifier,
    type_code: &str,
) -> io::Result<()> {
    xml.element(name, &[], |xml| {
        xml.text(n::ID, &scheme(object.scheme.as_deref()), &object.content)?;
        xml.text(n::DOCUMENT_TYPE_CODE, &[], type_code)
    })
}

fn supporting_document(xml: &mut Xml, document: &SupportingDocument) -> io::Result<()> {
    xml.element(n::ADDITIONAL_DOCUMENT_REFERENCE, &[], |xml| {
        xml.text(n::ID, &[], &document.reference)?;
        xml.optional_text(n::DOCUMENT_DESCRIPTION, document.description.as_deref())?;
        if document.attachment.is_none() && document.external_location.is_none() {
            return Ok(());
        }
        xml.element(n::ATTACHMENT, &[], |xml| {
            if let Some(file) = &document.attachment {
                xml.text(
                    n::EMBEDDED_DOCUMENT_BINARY_OBJECT,
                    &[
                        (n::MIME_CODE, &file.mime_code),
                        (n::FILENAME, &file.filename),
                    ],
                    &format::attachment(&file.content),
                )?;
            }
            match &document.external_location {
                Some(location) => xml.element(n::EXTERNAL_REFERENCE, &[], |xml| {
                    xml.text(n::URI, &[], location)
                }),
                None => Ok(()),
            }
        })
    })
}

fn buyer(xml: &mut Xml, buyer: &Buyer) -> io::Result<()> {
    xml.element(n::PARTY, &[], |xml| {
        if let Some(address) = &buyer.electronic_address {
            xml.text(
                n::ENDPOINT_ID,
                &[(n::SCHEME_ID, &address.scheme)],
                &address.content,
            )?;
        }
        if let Some(identifier) = &buyer.identifier {
            party_identification(xml, &identifier.content, identifier.scheme.as_deref())?;
        }
        party_name(xml, buyer.trading_name.as_deref())?;
        postal_address(xml, n::POSTAL_ADDRESS, &buyer.address)?;
        tax_scheme(
            xml,
            buyer.vat_identifier.as_ref().map(|id| id.0.as_str()),
            n::VAT_SCHEME,
        )?;
        xml.element(n::PARTY_LEGAL_ENTITY, &[], |xml| {
            xml.text(n::REGISTRATION_NAME, &[], &buyer.name)?;
            company_id(xml, buyer.legal_registration.as_ref())
        })?;
        contact(xml, buyer.contact.as_ref())
    })
}

fn tax_representative(xml: &mut Xml, representative: &TaxRepresentative) -> io::Result<()> {
    xml.element(n::TAX_REPRESENTATIVE_PARTY, &[], |xml| {
        party_name(xml, Some(&representative.name))?;
        postal_address(xml, n::POSTAL_ADDRESS, &representative.address)?;
        tax_scheme(xml, Some(&representative.vat_identifier.0), n::VAT_SCHEME)
    })
}

fn party_identification(xml: &mut Xml, content: &str, scheme_id: Option<&str>) -> io::Result<()> {
    xml.element(n::PARTY_IDENTIFICATION, &[], |xml| {
        xml.text(n::ID, &scheme(scheme_id), content)
    })
}

fn party_name(xml: &mut Xml, name: Option<&str>) -> io::Result<()> {
    match name {
        Some(name) => xml.element(n::PARTY_NAME, &[], |xml| {
            xml.text(n::NAME_ELEMENT, &[], name)
        }),
        None => Ok(()),
    }
}

fn company_id(xml: &mut Xml, registration: Option<&OptionalSchemeIdentifier>) -> io::Result<()> {
    match registration {
        Some(registration) => xml.text(
            n::COMPANY_ID,
            &scheme(registration.scheme.as_deref()),
            &registration.content,
        ),
        None => Ok(()),
    }
}

/// A tax registration of a party under the tax scheme `scheme_id`.
fn tax_scheme(xml: &mut Xml, company: Option<&str>, scheme_id: &str) -> io::Result<()> {
    match company {
        Some(company) => xml.element(n::PARTY_TAX_SCHEME, &[], |xml| {
            xml.text(n::COMPANY_ID, &[], company)?;
            xml.element(n::TAX_SCHEME, &[], |xml| xml.text(n::ID, &[], scheme_id))
        }),
        None => Ok(()),
    }
}

fn postal_address(xml: &mut Xml, name: &str, address: &PostalAddress) -> io::Result<()> {
    xml.element(name, &[], |xml| {
        xml.optional_text(n::STREET_NAME, address.line_1.as_deref())?;
        xml.optional_text(n::ADDITIONAL_STREET_NAME, address.line_2.as_deref())?;
        xml.optional_text(n::CITY_NAME, address.city.as_deref())?;
        xml.optional_text(n::POSTAL_ZONE, address.post_code.as_deref())?;
        xml.optional_text(n::COUNTRY_SUBENTITY, address.country_subdivision.as_deref())?;
        if let Some(line) = &address.line_3 {
            xml.element(n::ADDRESS_LINE, &[], |xml| xml.text(n::LINE, &[], line))?;
        }
        xml.element(n::COUNTRY, &[], |xml| {
            xml.text(n::IDENTIFICATION_CODE, &[], address.country.as_str())
        })
    })
}

fn contact(xml: &mut Xml, contact: Option<&Contact>) -> io::Result<()> {
    match contact {
        Some(contact) => xml.element(n::CONTACT, &[], |xml| {
            xml.optional_text(n::NAME_ELEMENT, contact.point.as_deref())?;
            xml.optional_text(n::TELEPHONE, contact.telephone.as_deref())?;
            xml.optional_text(n::ELECTRONIC_MAIL, contact.email.as_deref())
        }),
        None => Ok(()),
    }
}

fn period_dates(xml: &mut Xml, period: &Period) -> io::Result<()> {
    if let Some(start) = period.start {
        xml.text(n::START_DATE, &[], &format::ubl_date(start))?;
    }
    match period.end {
        Some(end) => xml.text(n::END_DATE, &[], &format::ubl_date(end)),
        None => Ok(()),
    }
}

fn percentage(xml: &mut Xml, name: &str, value: Option<Percentage>) -> io::Result<()> {
    match value {
        Some(value) => xml.text(name, &[], &format::number(value.0)),
        None => Ok(()),
    }
}

fn tax_category(
    xml: &mut Xml,
    name: &str,
    category: VatCategory,
    rate: Option<Percentage>,
    exemption_code: Option<&str>,
    exemption_reason: Option<&str>,
) -> io::Result<()> {
    xml.element(name, &[], |xml| {
        xml.text(n::ID, &[], category.code())?;
        percentage(xml, n::PERCENT, rate)?;
        xml.optional_text(n::TAX_EXEMPTION_REASON_CODE, exemption_code)?;
        xml.optional_text(n::TAX_EXEMPTION_REASON, exemption_reason)?;
        xml.element(n::TAX_SCHEME, &[], |xml| {
            xml.text(n::ID, &[], n::VAT_SCHEME)
        })
    })
}

fn item(
    xml: &mut Xml,
    item: &ItemInformation,
    category: VatCategory,
    rate: Option<Percentage>,
) -> io::Result<()> {
    xml.element(n::ITEM, &[], |xml| {
        xml.optional_text(n::DESCRIPTION, item.description.as_deref())?;
        xml.text(n::NAME_ELEMENT, &[], &item.name)?;
        if let Some(identifier) = &item.buyer_identifier {
            xml.element(n::BUYERS_ITEM_IDENTIFICATION, &[], |xml| {
                xml.text(n::ID, &[], &identifier.0)
            })?;
        }
        if let Some(identifier) = &item.seller_identifier {
            xml.element(n::SELLERS_ITEM_IDENTIFICATION, &[], |xml| {
                xml.text(n::ID, &[], &identifier.0)
            })?;
        }
        if let Some(identifier) = &item.standard_identifier {
            xml.element(n::STANDARD_ITEM_IDENTIFICATION, &[], |xml| {
                xml.text(
                    n::ID,
                    &[(n::SCHEME_ID, &identifier.scheme)],
                    &identifier.content,
                )
            })?;
        }
        if let Some(country) = &item.country_of_origin {
            xml.element(n::ORIGIN_COUNTRY, &[], |xml| {
                xml.text(n::IDENTIFICATION_CODE, &[], country.as_str())
            })?;
        }
        for classification in &item.classifications {
            let mut attributes = vec![(n::LIST_ID, classification.scheme.as_str())];
            if let Some(version) = &classification.scheme_version {
                attributes.push((n::LIST_VERSION_ID, version));
            }
            xml.element(n::COMMODITY_CLASSIFICATION, &[], |xml| {
                xml.text(
                    n::ITEM_CLASSIFICATION_CODE,
                    &attributes,
                    &classification.content,
                )
            })?;
        }
        tax_category(xml, n::CLASSIFIED_TAX_CATEGORY, category, rate, None, None)?;
        for attribute in &item.attributes {
            xml.element(n::ADDITIONAL_ITEM_PROPERTY, &[], |xml| {
                xml.text(n::NAME_ELEMENT, &[], &attribute.name)?;
                xml.text(n::VALUE, &[], &attribute.value)
            })?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::reads_like_subject;

    #[test]
    fn notes_read_like_a_subject_as_their_table_says() {
        for (text, subject) in [
            ("#AAI#Text", true),
            ("#3#", true),
            ("#3 Teillieferung", false),
            ("##Text", false),
            ("Text #AAI#", false),
            ("#", false),
        ] {
            assert_eq!(reads_like_subject(text), subject, "{text}");
        }
    }
}
