// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The semantic data model of an electronic invoice after EN 16931-1.
//!
//! This is the one description of an invoice that reading a document,
//! writing UBL and CII, and the form the user confirms all work towards. It
//! reads and writes nothing itself.
//!
//! Every business term (BT) and business group (BG) of the standard is a
//! field or a type here, named after what it means; the identifier of the
//! standard is given in its documentation. Its cardinality is its type: a
//! mandatory term is a plain field, an optional one an [`Option`], a repeated
//! one a [`Vec`], one that must occur at least once a [`NonEmpty`].
//!
//! The cardinalities are those of EN 16931, not those of a usage that makes
//! more terms mandatory, such as XRechnung. Such a usage is checked by its own
//! validation rules.

mod delivery;
mod draft;
mod invoice;
mod line;
mod list;
mod party;
mod payment;
mod terms;
mod totals;
mod values;

pub use delivery::{DeliveryInformation, Period};
pub use draft::{DATE_FORMAT, Draft, Entry, Location, Occurrence, Origin};
pub use invoice::{
    Invoice, InvoiceNote, PrecedingInvoice, ProcessControl, SupportingDocument, VatPoint,
};
pub use line::{
    InvoiceLine, ItemAttribute, ItemInformation, LineAdjustment, LineVat, PriceDetails,
};
pub use list::NonEmpty;
pub use party::{Buyer, Contact, Payee, PostalAddress, Seller, TaxRepresentative};
pub use payment::{CreditTransfer, DirectDebit, PaymentCard, PaymentInstructions};
pub use terms::{
    DataType, LAST_GROUP, LAST_TERM, TERMS, Term, UNASSIGNED_TERMS, group_id, group_number,
    term_by_id, term_by_number,
};
pub use totals::{DocumentAdjustment, DocumentTotals, VatBreakdown};
pub use values::{
    Amount, BinaryObject, Code, CountryCode, CurrencyCode, Date, DocumentReference, Identifier,
    ItemClassification, LetterCode, OptionalSchemeIdentifier, Percentage, Quantity,
    RequiredSchemeIdentifier, Text, UnitPriceAmount, ValueError, VatCategory,
};
