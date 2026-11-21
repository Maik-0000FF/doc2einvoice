// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The invoice as a whole: the business terms on document level and the
//! groups that hang directly from it.

use crate::delivery::DeliveryInformation;
use crate::line::InvoiceLine;
use crate::list::NonEmpty;
use crate::party::{Buyer, Payee, Seller, TaxRepresentative};
use crate::payment::PaymentInstructions;
use crate::totals::{DocumentAdjustment, DocumentTotals, VatBreakdown};
use crate::values::{BinaryObject, Code, CurrencyCode, Date, DocumentReference, Identifier, Text};

/// An electronic invoice after the semantic data model of EN 16931.
///
/// Each business term has the cardinality the standard gives it. A stricter
/// usage, such as XRechnung, is checked by its own validation rules and does
/// not narrow this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    /// The number of the invoice in the system of the seller (BT-1).
    pub number: Identifier,
    /// The day the invoice was issued (BT-2).
    pub issue_date: Date,
    /// The kind of invoice, a code of UNTDID 1001 (BT-3).
    pub type_code: Code,
    /// The currency of all amounts, except BT-111 (BT-5).
    pub currency: CurrencyCode,
    /// The currency VAT is accounted in, where it differs (BT-6).
    pub vat_accounting_currency: Option<CurrencyCode>,
    /// When VAT becomes accountable (BT-7 or BT-8).
    pub vat_point: Option<VatPoint>,
    /// The day the amount is due (BT-9).
    pub payment_due_date: Option<Date>,
    /// The reference the buyer uses to route the invoice; in XRechnung the
    /// Leitweg-ID (BT-10).
    pub buyer_reference: Option<Text>,
    /// The project the invoice refers to (BT-11).
    pub project_reference: Option<DocumentReference>,
    /// The contract (BT-12).
    pub contract_reference: Option<DocumentReference>,
    /// The purchase order of the buyer (BT-13).
    pub purchase_order_reference: Option<DocumentReference>,
    /// The sales order of the seller (BT-14).
    pub sales_order_reference: Option<DocumentReference>,
    /// The receiving advice (BT-15).
    pub receiving_advice_reference: Option<DocumentReference>,
    /// The despatch advice (BT-16).
    pub despatch_advice_reference: Option<DocumentReference>,
    /// The tender or lot (BT-17).
    pub tender_or_lot_reference: Option<DocumentReference>,
    /// An object the invoice refers to, such as a meter or a subscription
    /// (BT-18).
    pub invoiced_object: Option<Identifier>,
    /// Where the buyer books the invoice (BT-19).
    pub buyer_accounting_reference: Option<Text>,
    /// The terms of payment in words (BT-20).
    pub payment_terms: Option<Text>,
    /// Notes on the invoice as a whole (BG-1).
    pub notes: Vec<InvoiceNote>,
    /// The business process and the specification followed (BG-2).
    pub process_control: ProcessControl,
    /// Earlier invoices this one refers to (BG-3).
    pub preceding_invoices: Vec<PrecedingInvoice>,
    /// The seller (BG-4).
    pub seller: Seller,
    /// The buyer (BG-7).
    pub buyer: Buyer,
    /// The payee, where it is not the seller (BG-10).
    pub payee: Option<Payee>,
    /// The tax representative of the seller (BG-11).
    pub tax_representative: Option<TaxRepresentative>,
    /// Where and when the delivery took place (BG-13).
    pub delivery: Option<DeliveryInformation>,
    /// How the invoice is to be paid (BG-16).
    pub payment_instructions: Option<PaymentInstructions>,
    /// Allowances on the invoice as a whole (BG-20).
    pub allowances: Vec<DocumentAdjustment>,
    /// Charges on the invoice as a whole (BG-21).
    pub charges: Vec<DocumentAdjustment>,
    /// The totals (BG-22).
    pub totals: DocumentTotals,
    /// The VAT by category and rate (BG-23).
    pub vat_breakdown: NonEmpty<VatBreakdown>,
    /// Documents that support the invoice (BG-24).
    pub supporting_documents: Vec<SupportingDocument>,
    /// The lines (BG-25).
    pub lines: NonEmpty<InvoiceLine>,
}

/// When VAT becomes accountable: as a day, or as a code for a day not yet
/// known when the invoice is issued. The standard allows one of the two,
/// never both (BR-CO-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VatPoint {
    /// The day (BT-7).
    Date(Date),
    /// A code of UNTDID 2005 (BT-8).
    Code(Code),
}

/// A note on the invoice as a whole (BG-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceNote {
    /// The subject of the note, a code of UNTDID 4451 (BT-21).
    pub subject_code: Option<Code>,
    /// The note (BT-22).
    pub text: Text,
}

/// The business process and the specification followed (BG-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessControl {
    /// The business process, as the buyer specifies it (BT-23).
    pub business_process: Option<Text>,
    /// The specification the invoice conforms to, such as
    /// "urn:cen.eu:en16931:2017" (BT-24).
    pub specification: Identifier,
}

/// An earlier invoice this one refers to (BG-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedingInvoice {
    /// The number of the earlier invoice (BT-25).
    pub reference: DocumentReference,
    /// The day it was issued (BT-26).
    pub issue_date: Option<Date>,
}

/// A document that supports the invoice (BG-24).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportingDocument {
    /// The identifier of the document (BT-122).
    pub reference: DocumentReference,
    /// A description (BT-123).
    pub description: Option<Text>,
    /// Where the document can be downloaded (BT-124).
    pub external_location: Option<Text>,
    /// The document itself, attached (BT-125).
    pub attachment: Option<BinaryObject>,
}
