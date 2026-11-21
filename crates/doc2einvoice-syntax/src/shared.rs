// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What both syntaxes derive from an invoice in the same way before writing
//! it in their own elements.

use doc2einvoice_model::{Amount, CreditTransfer, Invoice, PaymentInstructions};

/// One payment means as both syntaxes write it: one for each account to
/// transfer to, or a single one if there is none. The card and the direct
/// debit belong to the first.
pub(crate) struct PaymentMeans<'a> {
    /// The account to transfer to, if any.
    pub(crate) account: Option<&'a CreditTransfer>,
    /// Whether this is the first payment means.
    pub(crate) first: bool,
}

/// The payment means of `payment`, in order.
///
/// | Accounts to transfer to | Payment means                       |
/// |-------------------------|-------------------------------------|
/// | none                    | one, without an account, first      |
/// | one                     | one, with that account, first       |
/// | two                     | two, one per account, the first first |
pub(crate) fn payment_means(payment: &PaymentInstructions) -> Vec<PaymentMeans<'_>> {
    if payment.credit_transfers.is_empty() {
        return vec![PaymentMeans {
            account: None,
            first: true,
        }];
    }
    payment
        .credit_transfers
        .iter()
        .enumerate()
        .map(|(index, account)| PaymentMeans {
            account: Some(account),
            first: index == 0,
        })
        .collect()
}

/// The VAT totals with the currency each is stated in: the total in the
/// invoice currency (BT-110), and the total in the VAT accounting currency
/// (BT-111) where both it and its currency (BT-6) are given.
pub(crate) fn vat_totals(invoice: &Invoice) -> Vec<(Amount, &str)> {
    let totals = &invoice.totals;
    let in_invoice_currency = totals
        .vat_total
        .map(|total| (total, invoice.currency.as_str()));
    let in_accounting_currency = totals
        .vat_total_in_accounting_currency
        .zip(invoice.vat_accounting_currency.as_ref())
        .map(|(total, currency)| (total, currency.as_str()));
    in_invoice_currency
        .into_iter()
        .chain(in_accounting_currency)
        .collect()
}
