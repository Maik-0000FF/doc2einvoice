// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! How the invoice is to be paid (BG-16).

use crate::values::{Code, Identifier, Text};

/// Payment instructions (BG-16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentInstructions {
    /// The means of payment, a code of UNTDID 4461 (BT-81).
    pub means_code: Code,
    /// The means of payment in words (BT-82).
    pub means_text: Option<Text>,
    /// The text that links the payment to the invoice (BT-83).
    pub remittance_information: Option<Text>,
    /// Accounts to transfer the payment to (BG-17).
    pub credit_transfers: Vec<CreditTransfer>,
    /// The card used for the payment (BG-18).
    pub card: Option<PaymentCard>,
    /// The direct debit (BG-19).
    pub direct_debit: Option<DirectDebit>,
}

/// An account to transfer the payment to (BG-17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditTransfer {
    /// The account, an IBAN within the SEPA area (BT-84).
    pub account: Identifier,
    /// The name of the account (BT-85).
    pub account_name: Option<Text>,
    /// The payment service provider, such as a BIC (BT-86).
    pub service_provider: Option<Identifier>,
}

/// The card used for the payment (BG-18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentCard {
    /// The last four to six digits of the card number, never the full number
    /// (BT-87).
    pub primary_account_number: Text,
    /// The name of the card holder (BT-88).
    pub holder_name: Option<Text>,
}

/// A direct debit (BG-19).
///
/// The three business terms of this group are optional in EN 16931: no rule
/// of the standard demands them. XRechnung makes them mandatory with rules of
/// its own (PEPPOL-EN16931-R061, BR-DE-30, BR-DE-31), which its validation
/// rules check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectDebit {
    /// The mandate reference (BT-89).
    pub mandate_reference: Option<Identifier>,
    /// The creditor identifier of the seller or payee (BT-90).
    pub creditor_identifier: Option<Identifier>,
    /// The account debited, an IBAN within the SEPA area (BT-91).
    pub debited_account: Option<Identifier>,
}
