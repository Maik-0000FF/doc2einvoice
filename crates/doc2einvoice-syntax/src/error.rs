// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Why an invoice could not be written.

/// The syntaxes an invoice is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// UBL 2.1, as an invoice or a credit note.
    Ubl,
    /// UN/CEFACT Cross Industry Invoice D16B.
    Cii,
}

/// What an invoice lacks that the syntax needs to write it. The model allows
/// it, the syntax has no way to express it without a value nobody gave; the
/// writer refuses rather than invent one.
///
/// Not a gap: a fixed code word the syntax requires and no business term
/// holds, such as the tax scheme of a local tax registration or the name of
/// a project in CII. It says nothing about the invoice and is written as the
/// published examples of the standard write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gap {
    /// The sales order (BT-14) is given without the purchase order (BT-13),
    /// which UBL writes in the same element and requires.
    SalesOrderWithoutPurchaseOrder,
    /// The VAT total (BT-110) is missing; UBL requires it beside the VAT
    /// breakdown.
    VatTotalMissing,
    /// The gross price (BT-148) is given without the price discount (BT-147),
    /// which UBL writes in the same element and requires.
    GrossPriceWithoutDiscount,
    /// The unit of the price base quantity (BT-150) is given without the
    /// quantity (BT-149), whose attribute it is in UBL.
    BaseUnitWithoutBaseQuantity,
    /// A payment card (BG-18) is given; UBL requires the card network, which
    /// no business term holds.
    CardNetworkMissing,
    /// A credit note has a due date (BT-9) but no payment instructions
    /// (BG-16); UBL writes the due date of a credit note inside them.
    DueDateWithoutPaymentInstructions,
    /// The VAT total in accounting currency (BT-111) is given without that
    /// currency (BT-6).
    AccountingCurrencyMissing,
    /// More than one preceding invoice (BG-3) is given; CII holds one.
    SeveralPrecedingInvoices,
    /// The price discount (BT-147) is given without the gross price
    /// (BT-148), inside which CII writes it.
    DiscountWithoutGrossPrice,
    /// A note without a subject (BT-21) has a text (BT-22) that begins like
    /// the subject UBL writes in front of a note, `#`; a receiver would read
    /// a subject nobody gave.
    NoteTextLikeSubject,
}

/// An invoice that could not be written.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The syntax cannot express the invoice as it is.
    #[error("the invoice cannot be written in {syntax:?}: {gap:?}")]
    NotExpressible {
        /// The syntax.
        syntax: Syntax,
        /// What is missing.
        gap: Gap,
    },
    /// Writing the XML failed.
    #[error("writing the XML failed: {0}")]
    Xml(#[from] std::io::Error),
}
