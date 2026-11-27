// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What both syntaxes derive from an invoice in the same way before writing
//! it in their own elements.

use doc2einvoice_model::{Amount, CreditTransfer, Invoice, PaymentInstructions};
use iban::{Iban, IbanLike};

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

/// The account to transfer to (BT-84), as both syntaxes write it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Account<'a> {
    /// An IBAN, in its electronic form: no spaces, capital letters.
    Iban(String),
    /// Any other identifier of an account, as given.
    Other(&'a str),
}

impl Account<'_> {
    /// The identifier as it is written.
    pub(crate) fn value(&self) -> &str {
        match self {
            Self::Iban(iban) => iban,
            Self::Other(other) => other,
        }
    }
}

/// The account of `transfer`, by its form and check digits.
///
/// | Account                         | Form                                 |
/// |---------------------------------|--------------------------------------|
/// | `DE75512108001245126199`        | IBAN DE75512108001245126199          |
/// | `DE75 5121 0800 1245 1261 99`   | IBAN DE75512108001245126199          |
/// | `DE75512108001245126198`        | other, as given: the check digits fail |
/// | `0123456789`                    | other, as given                      |
pub(crate) fn account(transfer: &CreditTransfer) -> Account<'_> {
    let given = transfer.account.0.as_str();
    match given.parse::<Iban>() {
        Ok(iban) => Account::Iban(iban.electronic_str().to_owned()),
        Err(_) => Account::Other(given),
    }
}

/// The first business term that the invoice requires and gives only as a
/// blank value, if any. Written, it would be left out and the document
/// incomplete.
///
/// Required is what the standard requires of the invoice, and what it
/// requires within a group or an identifier that is given at all: an
/// electronic address that is given needs its scheme, an attached file its
/// content.
///
/// | Value                                                 | Result   |
/// |-------------------------------------------------------|----------|
/// | the seller name (BT-27) is `Muster`                   | none     |
/// | the seller name is empty or only spaces               | `BT-27`  |
/// | the seller has no electronic address (BT-34)          | none     |
/// | the electronic address is given, its scheme is blank  | `BT-34`  |
/// | a file is attached (BT-125), its content is empty     | `BT-125` |
/// | an optional value is blank                            | none: it is left out, which it may be |
pub(crate) fn blank_required_value(invoice: &Invoice) -> Option<&'static str> {
    if invoice
        .supporting_documents
        .iter()
        .filter_map(|document| document.attachment.as_ref())
        .any(|file| file.content.is_empty())
    {
        return Some(ATTACHED_DOCUMENT);
    }
    required_texts(invoice)
        .into_iter()
        .find(|(_, text)| text.trim().is_empty())
        .map(|(term, _)| term)
}

/// The business term of an attached document, whose file content is checked
/// apart from the texts.
const ATTACHED_DOCUMENT: &str = "BT-125";

/// Every required text of `invoice`, with the business term it belongs to,
/// group by group in the order of the standard.
fn required_texts(invoice: &Invoice) -> Vec<(&'static str, &str)> {
    let mut required: Vec<(&'static str, &str)> =
        vec![("BT-1", &invoice.number.0), ("BT-3", &invoice.type_code.0)];
    for note in &invoice.notes {
        required.push(("BT-22", &note.text));
    }
    required.push(("BT-24", &invoice.process_control.specification.0));
    for preceding in &invoice.preceding_invoices {
        required.push(("BT-25", &preceding.reference));
    }
    required.push(("BT-27", &invoice.seller.name));
    if let Some(address) = &invoice.seller.electronic_address {
        required.push(("BT-34", &address.content));
        required.push(("BT-34", &address.scheme));
    }
    required.push(("BT-44", &invoice.buyer.name));
    if let Some(address) = &invoice.buyer.electronic_address {
        required.push(("BT-49", &address.content));
        required.push(("BT-49", &address.scheme));
    }
    if let Some(payee) = &invoice.payee {
        required.push(("BT-59", &payee.name));
    }
    if let Some(representative) = &invoice.tax_representative {
        required.push(("BT-62", &representative.name));
        required.push(("BT-63", &representative.vat_identifier.0));
    }
    if let Some(payment) = &invoice.payment_instructions {
        required.push(("BT-81", &payment.means_code.0));
        for transfer in &payment.credit_transfers {
            required.push(("BT-84", &transfer.account.0));
        }
        if let Some(card) = &payment.card {
            required.push(("BT-87", &card.primary_account_number));
        }
    }
    for document in &invoice.supporting_documents {
        required.push(("BT-122", &document.reference));
        if let Some(file) = &document.attachment {
            required.push((ATTACHED_DOCUMENT, &file.mime_code));
            required.push((ATTACHED_DOCUMENT, &file.filename));
        }
    }
    for line in invoice.lines.iter() {
        required.push(("BT-126", &line.identifier.0));
        required.push(("BT-130", &line.unit.0));
        let item = &line.item;
        required.push(("BT-153", &item.name));
        if let Some(standard) = &item.standard_identifier {
            required.push(("BT-157", &standard.content));
            required.push(("BT-157", &standard.scheme));
        }
        for classification in &item.classifications {
            required.push(("BT-158", &classification.content));
            required.push(("BT-158", &classification.scheme));
        }
        for attribute in &item.attributes {
            required.push(("BT-160", &attribute.name));
            required.push(("BT-161", &attribute.value));
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use doc2einvoice_model::{CreditTransfer, Identifier};

    use super::{Account, account};

    fn of(given: &str) -> CreditTransfer {
        CreditTransfer {
            account: Identifier(given.to_owned()),
            account_name: None,
            service_provider: None,
        }
    }

    #[test]
    fn accounts_follow_their_table() {
        let iban = "DE75512108001245126199";
        assert_eq!(account(&of(iban)), Account::Iban(iban.to_owned()));
        assert_eq!(
            account(&of("DE75 5121 0800 1245 1261 99")),
            Account::Iban(iban.to_owned())
        );
        assert_eq!(
            account(&of("DE75512108001245126198")).value(),
            "DE75512108001245126198"
        );
        assert_eq!(account(&of("0123456789")), Account::Other("0123456789"));
    }
}
