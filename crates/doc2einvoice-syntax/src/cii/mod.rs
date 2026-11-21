// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing an invoice as UN/CEFACT Cross Industry Invoice D16B.
//!
//! Each business term goes to the CII element that holds it, in the order the
//! XML Schema of CII D16B prescribes.

mod names;

use std::io;

use doc2einvoice_model::{
    Amount, Buyer, Contact, CreditTransfer, Date, DocumentAdjustment, Invoice, InvoiceLine,
    ItemInformation, LineAdjustment, OptionalSchemeIdentifier, Payee, PaymentInstructions,
    Percentage, Period, PostalAddress, PriceDetails, Seller, SupportingDocument, TaxRepresentative,
    VatBreakdown, VatCategory, VatPoint,
};

use self::names as n;
use crate::error::{Gap, Syntax, WriteError};
use crate::format;
use crate::shared;
use crate::xml::{Attribute, Xml, given, optional_attribute};

/// Writes `invoice` as a Cross Industry Invoice.
///
/// # Errors
///
/// [`WriteError::NotExpressible`] if CII needs a value the invoice does not
/// have; nothing is written then.
pub fn write(invoice: &Invoice) -> Result<Vec<u8>, WriteError> {
    if let Some(gap) = gap(invoice) {
        return Err(WriteError::NotExpressible {
            syntax: Syntax::Cii,
            gap,
        });
    }
    let mut xml = Xml::new()?;
    let attributes = [
        (n::XMLNS_RSM, n::RSM_NAMESPACE),
        (n::XMLNS_RAM, n::RAM_NAMESPACE),
        (n::XMLNS_UDT, n::UDT_NAMESPACE),
        (n::XMLNS_QDT, n::QDT_NAMESPACE),
    ];
    let writer = CiiWriter { invoice };
    xml.required_element(n::CROSS_INDUSTRY_INVOICE, &attributes, |xml| {
        writer.document(xml)
    })?;
    Ok(xml.finish())
}

/// The first thing CII needs that `invoice` does not have, if any.
fn gap(invoice: &Invoice) -> Option<Gap> {
    if let Some(term) = shared::blank_required_value(invoice) {
        return Some(Gap::BlankRequiredValue(term));
    }
    let lines = || invoice.lines.iter();
    if invoice.preceding_invoices.len() > 1 {
        return Some(Gap::SeveralPrecedingInvoices);
    }
    if invoice.totals.vat_total_in_accounting_currency.is_some()
        && invoice.vat_accounting_currency.is_none()
    {
        return Some(Gap::AccountingCurrencyMissing);
    }
    if lines()
        .any(|line| line.price.base_quantity_unit.is_some() && line.price.base_quantity.is_none())
    {
        return Some(Gap::BaseUnitWithoutBaseQuantity);
    }
    if lines().any(|line| line.price.discount.is_some() && line.price.gross_price.is_none()) {
        return Some(Gap::DiscountWithoutGrossPrice);
    }
    None
}

fn date_attributes() -> [Attribute<'static>; 1] {
    [(n::FORMAT, format::CII_DATE_FORMAT)]
}

/// A date in the element `name`, in the date type `kind`.
fn date(xml: &mut Xml, name: &str, kind: &str, value: Date) -> io::Result<()> {
    xml.element(name, &[], |xml| {
        xml.text(kind, &date_attributes(), &format::cii_date(value))
    })
}

fn number(xml: &mut Xml, name: &str, value: rust_decimal::Decimal) -> io::Result<()> {
    xml.text(name, &[], &format::number(value))
}

fn amount(xml: &mut Xml, name: &str, value: Amount) -> io::Result<()> {
    number(xml, name, value.value())
}

fn optional_amount(xml: &mut Xml, name: &str, value: Option<Amount>) -> io::Result<()> {
    match value {
        Some(value) => amount(xml, name, value),
        None => Ok(()),
    }
}

fn percentage(xml: &mut Xml, name: &str, value: Option<Percentage>) -> io::Result<()> {
    match value {
        Some(value) => number(xml, name, value.0),
        None => Ok(()),
    }
}

fn indicator(xml: &mut Xml, charge: bool) -> io::Result<()> {
    xml.element(n::CHARGE_INDICATOR, &[], |xml| {
        xml.text(n::INDICATOR, &[], if charge { n::TRUE } else { n::FALSE })
    })
}

struct CiiWriter<'a> {
    invoice: &'a Invoice,
}

impl CiiWriter<'_> {
    fn document(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        xml.element(n::EXCHANGED_DOCUMENT_CONTEXT, &[], |xml| {
            if let Some(process) = &invoice.process_control.business_process {
                xml.element(n::BUSINESS_PROCESS_PARAMETER, &[], |xml| {
                    xml.text(n::ID, &[], process)
                })?;
            }
            xml.element(n::GUIDELINE_PARAMETER, &[], |xml| {
                xml.text(n::ID, &[], &invoice.process_control.specification.0)
            })
        })?;
        xml.element(n::EXCHANGED_DOCUMENT, &[], |xml| {
            xml.text(n::ID, &[], &invoice.number.0)?;
            xml.text(n::TYPE_CODE, &[], &invoice.type_code.0)?;
            date(
                xml,
                n::ISSUE_DATE_TIME,
                n::DATE_TIME_STRING,
                invoice.issue_date,
            )?;
            for note in &invoice.notes {
                xml.element(n::INCLUDED_NOTE, &[], |xml| {
                    xml.text(n::CONTENT, &[], &note.text)?;
                    xml.optional_text(
                        n::SUBJECT_CODE,
                        note.subject_code.as_ref().map(|code| code.0.as_str()),
                    )
                })?;
            }
            Ok(())
        })?;
        xml.element(n::SUPPLY_CHAIN_TRADE_TRANSACTION, &[], |xml| {
            for line in invoice.lines.iter() {
                line_item(xml, line)?;
            }
            xml.element(n::APPLICABLE_HEADER_TRADE_AGREEMENT, &[], |xml| {
                self.agreement(xml)
            })?;
            xml.required_element(n::APPLICABLE_HEADER_TRADE_DELIVERY, &[], |xml| {
                self.delivery(xml)
            })?;
            xml.element(n::APPLICABLE_HEADER_TRADE_SETTLEMENT, &[], |xml| {
                self.settlement(xml)
            })
        })
    }

    fn agreement(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        xml.optional_text(n::BUYER_REFERENCE, invoice.buyer_reference.as_deref())?;
        xml.element(n::SELLER_TRADE_PARTY, &[], |xml| {
            seller(xml, &invoice.seller)
        })?;
        xml.element(n::BUYER_TRADE_PARTY, &[], |xml| buyer(xml, &invoice.buyer))?;
        if let Some(representative) = &invoice.tax_representative {
            tax_representative(xml, representative)?;
        }
        referenced(
            xml,
            n::SELLER_ORDER_REFERENCED_DOCUMENT,
            invoice.sales_order_reference.as_deref(),
        )?;
        referenced(
            xml,
            n::BUYER_ORDER_REFERENCED_DOCUMENT,
            invoice.purchase_order_reference.as_deref(),
        )?;
        referenced(
            xml,
            n::CONTRACT_REFERENCED_DOCUMENT,
            invoice.contract_reference.as_deref(),
        )?;
        for supporting in &invoice.supporting_documents {
            supporting_document(xml, supporting)?;
        }
        if let Some(tender) = &invoice.tender_or_lot_reference {
            xml.element(n::ADDITIONAL_REFERENCED_DOCUMENT, &[], |xml| {
                xml.text(n::ISSUER_ASSIGNED_ID, &[], tender)?;
                xml.text(n::TYPE_CODE, &[], n::TENDER_TYPE)
            })?;
        }
        if let Some(object) = &invoice.invoiced_object {
            object_reference(xml, object)?;
        }
        if let Some(project) = &invoice.project_reference {
            xml.element(n::SPECIFIED_PROCURING_PROJECT, &[], |xml| {
                xml.text(n::ID, &[], project)?;
                xml.text(n::NAME, &[], n::PROJECT_NAME)
            })?;
        }
        Ok(())
    }

    fn delivery(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        if let Some(delivery) = &invoice.delivery {
            if delivery.party_name.is_some()
                || delivery.location.is_some()
                || delivery.address.is_some()
            {
                xml.element(n::SHIP_TO_TRADE_PARTY, &[], |xml| {
                    if let Some(location) = &delivery.location {
                        party_identifier(xml, location)?;
                    }
                    xml.optional_text(n::NAME, delivery.party_name.as_deref())?;
                    match &delivery.address {
                        Some(address) => postal_address(xml, address),
                        None => Ok(()),
                    }
                })?;
            }
            if let Some(actual) = delivery.actual_date {
                xml.element(n::ACTUAL_DELIVERY_SUPPLY_CHAIN_EVENT, &[], |xml| {
                    date(xml, n::OCCURRENCE_DATE_TIME, n::DATE_TIME_STRING, actual)
                })?;
            }
        }
        referenced(
            xml,
            n::DESPATCH_ADVICE_REFERENCED_DOCUMENT,
            invoice.despatch_advice_reference.as_deref(),
        )?;
        referenced(
            xml,
            n::RECEIVING_ADVICE_REFERENCED_DOCUMENT,
            invoice.receiving_advice_reference.as_deref(),
        )
    }

    fn settlement(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        let payment = invoice.payment_instructions.as_ref();
        let debit = payment.and_then(|payment| payment.direct_debit.as_ref());
        xml.optional_text(
            n::CREDITOR_REFERENCE_ID,
            debit
                .and_then(|debit| debit.creditor_identifier.as_ref())
                .map(|id| id.0.as_str()),
        )?;
        xml.optional_text(
            n::PAYMENT_REFERENCE,
            payment.and_then(|payment| payment.remittance_information.as_deref()),
        )?;
        if let Some(currency) = &invoice.vat_accounting_currency {
            xml.text(n::TAX_CURRENCY_CODE, &[], currency.as_str())?;
        }
        xml.text(n::INVOICE_CURRENCY_CODE, &[], invoice.currency.as_str())?;
        if let Some(payee) = &invoice.payee {
            payee_party(xml, payee)?;
        }
        if let Some(payment) = payment {
            payment_means(xml, payment)?;
        }
        for breakdown in invoice.vat_breakdown.iter() {
            self.vat_breakdown(xml, breakdown)?;
        }
        if let Some(period) = invoice
            .delivery
            .as_ref()
            .and_then(|delivery| delivery.invoicing_period.as_ref())
        {
            xml.element(n::BILLING_SPECIFIED_PERIOD, &[], |xml| {
                period_dates(xml, period)
            })?;
        }
        for allowance in &invoice.allowances {
            document_adjustment(xml, allowance, false)?;
        }
        for charge in &invoice.charges {
            document_adjustment(xml, charge, true)?;
        }
        self.payment_terms(xml)?;
        self.monetary_summation(xml)?;
        if let Some(preceding) = invoice.preceding_invoices.first() {
            xml.element(n::INVOICE_REFERENCED_DOCUMENT, &[], |xml| {
                xml.text(n::ISSUER_ASSIGNED_ID, &[], &preceding.reference)?;
                match preceding.issue_date {
                    Some(issued) => date(
                        xml,
                        n::FORMATTED_ISSUE_DATE_TIME,
                        n::QUALIFIED_DATE_TIME_STRING,
                        issued,
                    ),
                    None => Ok(()),
                }
            })?;
        }
        accounting_account(xml, invoice.buyer_accounting_reference.as_deref())
    }

    /// A VAT breakdown (BG-23), with the VAT point date (BT-7) or its code
    /// (BT-8), which CII repeats in every breakdown.
    fn vat_breakdown(&self, xml: &mut Xml, breakdown: &VatBreakdown) -> io::Result<()> {
        xml.element(n::APPLICABLE_TRADE_TAX, &[], |xml| {
            amount(xml, n::CALCULATED_AMOUNT, breakdown.tax_amount)?;
            xml.text(n::TYPE_CODE, &[], n::VAT_TYPE)?;
            xml.optional_text(n::EXEMPTION_REASON, breakdown.exemption_reason.as_deref())?;
            amount(xml, n::BASIS_AMOUNT, breakdown.taxable_amount)?;
            xml.text(n::CATEGORY_CODE, &[], breakdown.category.code())?;
            xml.optional_text(
                n::EXEMPTION_REASON_CODE,
                breakdown
                    .exemption_reason_code
                    .as_ref()
                    .map(|code| code.0.as_str()),
            )?;
            match &self.invoice.vat_point {
                Some(VatPoint::Date(point)) => {
                    date(xml, n::TAX_POINT_DATE, n::DATE_STRING, *point)?;
                }
                Some(VatPoint::Code(code)) => xml.text(n::DUE_DATE_TYPE_CODE, &[], &code.0)?,
                None => {}
            }
            percentage(xml, n::RATE_APPLICABLE_PERCENT, breakdown.rate)
        })
    }

    /// The payment terms (BT-20), the due date (BT-9) and the mandate
    /// reference (BT-89), which CII writes in the same element.
    fn payment_terms(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        let mandate = invoice
            .payment_instructions
            .as_ref()
            .and_then(|payment| payment.direct_debit.as_ref())
            .and_then(|debit| debit.mandate_reference.as_ref());
        if invoice.payment_terms.is_none()
            && invoice.payment_due_date.is_none()
            && mandate.is_none()
        {
            return Ok(());
        }
        xml.element(n::SPECIFIED_TRADE_PAYMENT_TERMS, &[], |xml| {
            xml.optional_text(n::DESCRIPTION, invoice.payment_terms.as_deref())?;
            if let Some(due) = invoice.payment_due_date {
                date(xml, n::DUE_DATE_DATE_TIME, n::DATE_TIME_STRING, due)?;
            }
            xml.optional_text(n::DIRECT_DEBIT_MANDATE_ID, mandate.map(|id| id.0.as_str()))
        })
    }

    fn monetary_summation(&self, xml: &mut Xml) -> io::Result<()> {
        let invoice = self.invoice;
        let totals = &invoice.totals;
        xml.element(
            n::SPECIFIED_TRADE_SETTLEMENT_HEADER_MONETARY_SUMMATION,
            &[],
            |xml| {
                amount(xml, n::LINE_TOTAL_AMOUNT, totals.line_net_total)?;
                optional_amount(xml, n::CHARGE_TOTAL_AMOUNT, totals.charge_total)?;
                optional_amount(xml, n::ALLOWANCE_TOTAL_AMOUNT, totals.allowance_total)?;
                amount(xml, n::TAX_BASIS_TOTAL_AMOUNT, totals.total_without_vat)?;
                for (total, currency) in shared::vat_totals(invoice) {
                    xml.text(
                        n::TAX_TOTAL_AMOUNT,
                        &[(n::CURRENCY_ID, currency)],
                        &format::number(total.value()),
                    )?;
                }
                optional_amount(xml, n::ROUNDING_AMOUNT, totals.rounding)?;
                amount(xml, n::GRAND_TOTAL_AMOUNT, totals.total_with_vat)?;
                optional_amount(xml, n::TOTAL_PREPAID_AMOUNT, totals.paid)?;
                amount(xml, n::DUE_PAYABLE_AMOUNT, totals.due_for_payment)
            },
        )
    }
}

/// A referenced document that holds only its identifier.
fn referenced(xml: &mut Xml, name: &str, identifier: Option<&str>) -> io::Result<()> {
    match identifier {
        Some(identifier) => xml.element(name, &[], |xml| {
            xml.text(n::ISSUER_ASSIGNED_ID, &[], identifier)
        }),
        None => Ok(()),
    }
}

/// An invoiced object (BT-18, BT-128): its identifier, its type code, and its
/// scheme as the reference type code.
fn object_reference(xml: &mut Xml, object: &OptionalSchemeIdentifier) -> io::Result<()> {
    xml.element(n::ADDITIONAL_REFERENCED_DOCUMENT, &[], |xml| {
        xml.text(n::ISSUER_ASSIGNED_ID, &[], &object.content)?;
        xml.text(n::TYPE_CODE, &[], n::INVOICED_OBJECT_TYPE)?;
        xml.optional_text(n::REFERENCE_TYPE_CODE, object.scheme.as_deref())
    })
}

fn supporting_document(xml: &mut Xml, document: &SupportingDocument) -> io::Result<()> {
    xml.element(n::ADDITIONAL_REFERENCED_DOCUMENT, &[], |xml| {
        xml.text(n::ISSUER_ASSIGNED_ID, &[], &document.reference)?;
        xml.optional_text(n::URI_ID, document.external_location.as_deref())?;
        xml.text(n::TYPE_CODE, &[], n::SUPPORTING_DOCUMENT_TYPE)?;
        xml.optional_text(n::NAME, document.description.as_deref())?;
        match &document.attachment {
            Some(file) => xml.text(
                n::ATTACHMENT_BINARY_OBJECT,
                &[
                    (n::MIME_CODE, &file.mime_code),
                    (n::FILENAME, &file.filename),
                ],
                &format::attachment(&file.content),
            ),
            None => Ok(()),
        }
    })
}

/// An identifier of a party: as its global identifier if it names a scheme,
/// otherwise as its own identifier.
fn party_identifier(xml: &mut Xml, identifier: &OptionalSchemeIdentifier) -> io::Result<()> {
    match given(identifier.scheme.as_deref()) {
        Some(scheme) => xml.text(n::GLOBAL_ID, &[(n::SCHEME_ID, scheme)], &identifier.content),
        None => xml.text(n::ID, &[], &identifier.content),
    }
}

/// Several identifiers of a party, those without a scheme first, as the
/// schema orders them.
fn party_identifiers(xml: &mut Xml, identifiers: &[&OptionalSchemeIdentifier]) -> io::Result<()> {
    for identifier in identifiers
        .iter()
        .filter(|identifier| given(identifier.scheme.as_deref()).is_none())
    {
        party_identifier(xml, identifier)?;
    }
    for identifier in identifiers
        .iter()
        .filter(|identifier| given(identifier.scheme.as_deref()).is_some())
    {
        party_identifier(xml, identifier)?;
    }
    Ok(())
}

fn legal_organization(
    xml: &mut Xml,
    registration: Option<&OptionalSchemeIdentifier>,
    trading_name: Option<&str>,
) -> io::Result<()> {
    if registration.is_none() && trading_name.is_none() {
        return Ok(());
    }
    xml.element(n::SPECIFIED_LEGAL_ORGANIZATION, &[], |xml| {
        if let Some(registration) = registration {
            let scheme = optional_attribute(n::SCHEME_ID, registration.scheme.as_deref());
            xml.text(n::ID, &scheme, &registration.content)?;
        }
        xml.optional_text(n::TRADING_BUSINESS_NAME, trading_name)
    })
}

fn contact(xml: &mut Xml, contact: Option<&Contact>) -> io::Result<()> {
    let Some(contact) = contact else {
        return Ok(());
    };
    xml.element(n::DEFINED_TRADE_CONTACT, &[], |xml| {
        xml.optional_text(n::PERSON_NAME, contact.point.as_deref())?;
        if let Some(telephone) = &contact.telephone {
            xml.element(n::TELEPHONE_UNIVERSAL_COMMUNICATION, &[], |xml| {
                xml.text(n::COMPLETE_NUMBER, &[], telephone)
            })?;
        }
        match &contact.email {
            Some(email) => xml.element(n::EMAIL_URI_UNIVERSAL_COMMUNICATION, &[], |xml| {
                xml.text(n::URI_ID, &[], email)
            }),
            None => Ok(()),
        }
    })
}

fn postal_address(xml: &mut Xml, address: &PostalAddress) -> io::Result<()> {
    xml.element(n::POSTAL_TRADE_ADDRESS, &[], |xml| {
        xml.optional_text(n::POSTCODE_CODE, address.post_code.as_deref())?;
        xml.optional_text(n::LINE_ONE, address.line_1.as_deref())?;
        xml.optional_text(n::LINE_TWO, address.line_2.as_deref())?;
        xml.optional_text(n::LINE_THREE, address.line_3.as_deref())?;
        xml.optional_text(n::CITY_NAME, address.city.as_deref())?;
        xml.text(n::COUNTRY_ID, &[], address.country.as_str())?;
        xml.optional_text(
            n::COUNTRY_SUB_DIVISION_NAME,
            address.country_subdivision.as_deref(),
        )
    })
}

fn electronic_address(
    xml: &mut Xml,
    address: Option<&doc2einvoice_model::RequiredSchemeIdentifier>,
) -> io::Result<()> {
    match address {
        Some(address) => xml.element(n::URI_UNIVERSAL_COMMUNICATION, &[], |xml| {
            xml.text(
                n::URI_ID,
                &[(n::SCHEME_ID, &address.scheme)],
                &address.content,
            )
        }),
        None => Ok(()),
    }
}

fn tax_registration(xml: &mut Xml, identifier: Option<&str>, scheme: &str) -> io::Result<()> {
    match identifier {
        Some(identifier) => xml.element(n::SPECIFIED_TAX_REGISTRATION, &[], |xml| {
            xml.text(n::ID, &[(n::SCHEME_ID, scheme)], identifier)
        }),
        None => Ok(()),
    }
}

fn seller(xml: &mut Xml, seller: &Seller) -> io::Result<()> {
    let identifiers: Vec<&OptionalSchemeIdentifier> = seller.identifiers.iter().collect();
    party_identifiers(xml, &identifiers)?;
    xml.text(n::NAME, &[], &seller.name)?;
    xml.optional_text(
        n::DESCRIPTION,
        seller.additional_legal_information.as_deref(),
    )?;
    legal_organization(
        xml,
        seller.legal_registration.as_ref(),
        seller.trading_name.as_deref(),
    )?;
    contact(xml, seller.contact.as_ref())?;
    postal_address(xml, &seller.address)?;
    electronic_address(xml, seller.electronic_address.as_ref())?;
    tax_registration(
        xml,
        seller.vat_identifier.as_ref().map(|id| id.0.as_str()),
        n::VAT_REGISTRATION,
    )?;
    tax_registration(
        xml,
        seller.tax_registration.as_ref().map(|id| id.0.as_str()),
        n::LOCAL_TAX_REGISTRATION,
    )
}

fn buyer(xml: &mut Xml, buyer: &Buyer) -> io::Result<()> {
    if let Some(identifier) = &buyer.identifier {
        party_identifier(xml, identifier)?;
    }
    xml.text(n::NAME, &[], &buyer.name)?;
    legal_organization(
        xml,
        buyer.legal_registration.as_ref(),
        buyer.trading_name.as_deref(),
    )?;
    contact(xml, buyer.contact.as_ref())?;
    postal_address(xml, &buyer.address)?;
    electronic_address(xml, buyer.electronic_address.as_ref())?;
    tax_registration(
        xml,
        buyer.vat_identifier.as_ref().map(|id| id.0.as_str()),
        n::VAT_REGISTRATION,
    )
}

fn tax_representative(xml: &mut Xml, representative: &TaxRepresentative) -> io::Result<()> {
    xml.element(n::SELLER_TAX_REPRESENTATIVE_TRADE_PARTY, &[], |xml| {
        xml.text(n::NAME, &[], &representative.name)?;
        postal_address(xml, &representative.address)?;
        tax_registration(
            xml,
            Some(&representative.vat_identifier.0),
            n::VAT_REGISTRATION,
        )
    })
}

fn payee_party(xml: &mut Xml, payee: &Payee) -> io::Result<()> {
    xml.element(n::PAYEE_TRADE_PARTY, &[], |xml| {
        if let Some(identifier) = &payee.identifier {
            party_identifier(xml, identifier)?;
        }
        xml.text(n::NAME, &[], &payee.name)?;
        legal_organization(xml, payee.legal_registration.as_ref(), None)
    })
}

/// The payment instructions (BG-16): one payment means for each account to
/// transfer to, or a single one if there is none. The card and the debited
/// account are written with the first.
fn payment_means(xml: &mut Xml, payment: &PaymentInstructions) -> io::Result<()> {
    let debited = payment
        .direct_debit
        .as_ref()
        .and_then(|debit| debit.debited_account.as_ref());
    for shared::PaymentMeans { account, first } in shared::payment_means(payment) {
        xml.element(n::SPECIFIED_TRADE_SETTLEMENT_PAYMENT_MEANS, &[], |xml| {
            xml.text(n::TYPE_CODE, &[], &payment.means_code.0)?;
            xml.optional_text(n::INFORMATION, payment.means_text.as_deref())?;
            if let (Some(card), true) = (&payment.card, first) {
                xml.element(n::APPLICABLE_TRADE_SETTLEMENT_FINANCIAL_CARD, &[], |xml| {
                    xml.text(n::ID, &[], &card.primary_account_number)?;
                    xml.optional_text(n::CARDHOLDER_NAME, card.holder_name.as_deref())
                })?;
            }
            if let (Some(debited), true) = (debited, first) {
                xml.element(n::PAYER_PARTY_DEBTOR_FINANCIAL_ACCOUNT, &[], |xml| {
                    xml.text(n::IBAN_ID, &[], &debited.0)
                })?;
            }
            let Some(account) = account else {
                return Ok(());
            };
            creditor_account(xml, account)?;
            match &account.service_provider {
                Some(provider) => xml.element(
                    n::PAYEE_SPECIFIED_CREDITOR_FINANCIAL_INSTITUTION,
                    &[],
                    |xml| xml.text(n::BIC_ID, &[], &provider.0),
                ),
                None => Ok(()),
            }
        })?;
    }
    Ok(())
}

/// The account to transfer to (BT-84): an IBAN as `IBANID`, anything else
/// as `ProprietaryID` (see [`shared::account`]).
fn creditor_account(xml: &mut Xml, transfer: &CreditTransfer) -> io::Result<()> {
    let account = shared::account(transfer);
    xml.element(n::PAYEE_PARTY_CREDITOR_FINANCIAL_ACCOUNT, &[], |xml| {
        if let shared::Account::Iban(iban) = &account {
            xml.text(n::IBAN_ID, &[], iban)?;
        }
        xml.optional_text(n::ACCOUNT_NAME, transfer.account_name.as_deref())?;
        match &account {
            shared::Account::Other(other) => xml.text(n::PROPRIETARY_ID, &[], other),
            shared::Account::Iban(_) => Ok(()),
        }
    })
}

fn period_dates(xml: &mut Xml, period: &Period) -> io::Result<()> {
    if let Some(start) = period.start {
        date(xml, n::START_DATE_TIME, n::DATE_TIME_STRING, start)?;
    }
    match period.end {
        Some(end) => date(xml, n::END_DATE_TIME, n::DATE_TIME_STRING, end),
        None => Ok(()),
    }
}

fn category_tax(
    xml: &mut Xml,
    name: &str,
    category: VatCategory,
    rate: Option<Percentage>,
) -> io::Result<()> {
    xml.element(name, &[], |xml| {
        xml.text(n::TYPE_CODE, &[], n::VAT_TYPE)?;
        xml.text(n::CATEGORY_CODE, &[], category.code())?;
        percentage(xml, n::RATE_APPLICABLE_PERCENT, rate)
    })
}

fn document_adjustment(
    xml: &mut Xml,
    adjustment: &DocumentAdjustment,
    charge: bool,
) -> io::Result<()> {
    xml.element(n::SPECIFIED_TRADE_ALLOWANCE_CHARGE, &[], |xml| {
        indicator(xml, charge)?;
        percentage(xml, n::CALCULATION_PERCENT, adjustment.percentage)?;
        optional_amount(xml, n::BASIS_AMOUNT, adjustment.base_amount)?;
        amount(xml, n::ACTUAL_AMOUNT, adjustment.amount)?;
        xml.optional_text(
            n::REASON_CODE,
            adjustment.reason_code.as_ref().map(|code| code.0.as_str()),
        )?;
        xml.optional_text(n::REASON, adjustment.reason.as_deref())?;
        category_tax(
            xml,
            n::CATEGORY_TRADE_TAX,
            adjustment.vat_category,
            adjustment.vat_rate,
        )
    })
}

fn line_adjustment(xml: &mut Xml, adjustment: &LineAdjustment, charge: bool) -> io::Result<()> {
    xml.element(n::SPECIFIED_TRADE_ALLOWANCE_CHARGE, &[], |xml| {
        indicator(xml, charge)?;
        percentage(xml, n::CALCULATION_PERCENT, adjustment.percentage)?;
        optional_amount(xml, n::BASIS_AMOUNT, adjustment.base_amount)?;
        amount(xml, n::ACTUAL_AMOUNT, adjustment.amount)?;
        xml.optional_text(
            n::REASON_CODE,
            adjustment.reason_code.as_ref().map(|code| code.0.as_str()),
        )?;
        xml.optional_text(n::REASON, adjustment.reason.as_deref())
    })
}

fn accounting_account(xml: &mut Xml, reference: Option<&str>) -> io::Result<()> {
    match reference {
        Some(reference) => xml.element(n::RECEIVABLE_ACCOUNTING_ACCOUNT, &[], |xml| {
            xml.text(n::ID, &[], reference)
        }),
        None => Ok(()),
    }
}

fn line_item(xml: &mut Xml, line: &InvoiceLine) -> io::Result<()> {
    xml.element(n::INCLUDED_SUPPLY_CHAIN_TRADE_LINE_ITEM, &[], |xml| {
        xml.element(n::ASSOCIATED_DOCUMENT_LINE_DOCUMENT, &[], |xml| {
            xml.text(n::LINE_ID, &[], &line.identifier.0)?;
            match &line.note {
                Some(note) => {
                    xml.element(n::INCLUDED_NOTE, &[], |xml| xml.text(n::CONTENT, &[], note))
                }
                None => Ok(()),
            }
        })?;
        product(xml, &line.item)?;
        xml.element(n::SPECIFIED_LINE_TRADE_AGREEMENT, &[], |xml| {
            if let Some(order_line) = &line.purchase_order_line {
                xml.element(n::BUYER_ORDER_REFERENCED_DOCUMENT, &[], |xml| {
                    xml.text(n::LINE_ID, &[], order_line)
                })?;
            }
            prices(xml, &line.price)
        })?;
        xml.element(n::SPECIFIED_LINE_TRADE_DELIVERY, &[], |xml| {
            xml.text(
                n::BILLED_QUANTITY,
                &[(n::UNIT_CODE, &line.unit.0)],
                &format::number(line.quantity.0),
            )
        })?;
        xml.element(n::SPECIFIED_LINE_TRADE_SETTLEMENT, &[], |xml| {
            category_tax(
                xml,
                n::APPLICABLE_TRADE_TAX,
                line.vat.category,
                line.vat.rate,
            )?;
            if let Some(period) = &line.period {
                xml.element(n::BILLING_SPECIFIED_PERIOD, &[], |xml| {
                    period_dates(xml, period)
                })?;
            }
            for allowance in &line.allowances {
                line_adjustment(xml, allowance, false)?;
            }
            for charge in &line.charges {
                line_adjustment(xml, charge, true)?;
            }
            xml.element(
                n::SPECIFIED_TRADE_SETTLEMENT_LINE_MONETARY_SUMMATION,
                &[],
                |xml| amount(xml, n::LINE_TOTAL_AMOUNT, line.net_amount),
            )?;
            if let Some(object) = &line.object_identifier {
                object_reference(xml, object)?;
            }
            accounting_account(xml, line.buyer_accounting_reference.as_deref())
        })
    })
}

fn product(xml: &mut Xml, item: &ItemInformation) -> io::Result<()> {
    xml.element(n::SPECIFIED_TRADE_PRODUCT, &[], |xml| {
        if let Some(identifier) = &item.standard_identifier {
            xml.text(
                n::GLOBAL_ID,
                &[(n::SCHEME_ID, &identifier.scheme)],
                &identifier.content,
            )?;
        }
        xml.optional_text(
            n::SELLER_ASSIGNED_ID,
            item.seller_identifier.as_ref().map(|id| id.0.as_str()),
        )?;
        xml.optional_text(
            n::BUYER_ASSIGNED_ID,
            item.buyer_identifier.as_ref().map(|id| id.0.as_str()),
        )?;
        xml.text(n::NAME, &[], &item.name)?;
        xml.optional_text(n::DESCRIPTION, item.description.as_deref())?;
        for attribute in &item.attributes {
            xml.element(n::APPLICABLE_PRODUCT_CHARACTERISTIC, &[], |xml| {
                xml.text(n::DESCRIPTION, &[], &attribute.name)?;
                xml.text(n::VALUE, &[], &attribute.value)
            })?;
        }
        for classification in &item.classifications {
            let mut attributes = vec![(n::LIST_ID, classification.scheme.as_str())];
            if let Some(version) = &classification.scheme_version {
                attributes.push((n::LIST_VERSION_ID, version));
            }
            xml.element(n::DESIGNATED_PRODUCT_CLASSIFICATION, &[], |xml| {
                xml.text(n::CLASS_CODE, &attributes, &classification.content)
            })?;
        }
        match &item.country_of_origin {
            Some(country) => xml.element(n::ORIGIN_TRADE_COUNTRY, &[], |xml| {
                xml.text(n::ID, &[], country.as_str())
            }),
            None => Ok(()),
        }
    })
}

/// The gross price with its discount (BT-148, BT-147) and the net price
/// (BT-146), each with the base quantity (BT-149, BT-150).
fn prices(xml: &mut Xml, price: &PriceDetails) -> io::Result<()> {
    let unit = optional_attribute(
        n::UNIT_CODE,
        price
            .base_quantity_unit
            .as_ref()
            .map(|unit| unit.0.as_str()),
    );
    let base_quantity = |xml: &mut Xml| match price.base_quantity {
        Some(base) => xml.text(n::BASIS_QUANTITY, &unit, &format::number(base.0)),
        None => Ok(()),
    };
    if let Some(gross) = price.gross_price {
        xml.element(n::GROSS_PRICE_PRODUCT_TRADE_PRICE, &[], |xml| {
            number(xml, n::CHARGE_AMOUNT, gross.0)?;
            base_quantity(xml)?;
            match price.discount {
                Some(discount) => xml.element(n::APPLIED_TRADE_ALLOWANCE_CHARGE, &[], |xml| {
                    indicator(xml, false)?;
                    number(xml, n::ACTUAL_AMOUNT, discount.0)
                }),
                None => Ok(()),
            }
        })?;
    }
    xml.element(n::NET_PRICE_PRODUCT_TRADE_PRICE, &[], |xml| {
        number(xml, n::CHARGE_AMOUNT, price.net_price.0)?;
        base_quantity(xml)
    })
}
