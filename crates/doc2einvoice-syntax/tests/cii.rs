// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! A written Cross Industry Invoice passes the XML Schema of CII D16B and the
//! rules of EN 16931 and XRechnung without a single finding.

mod support;

use doc2einvoice_syntax::write_cii;
use doc2einvoice_validation::Profile;
use support::{EN16931, Payment, XRECHNUNG, complete, findings};

fn assert_clean(payment: Payment, specification: &str, profile: Profile) {
    let invoice = complete(payment, specification);
    let document = write_cii(&invoice).unwrap_or_else(|error| unreachable!("{error}"));
    let found = findings(&document, profile);
    assert!(
        found.is_empty(),
        "{} findings:\n{}\n\n{}",
        found.len(),
        found.join("\n"),
        String::from_utf8_lossy(&document)
    );
}

#[test]
fn a_complete_invoice_paid_by_transfer_conforms_to_en16931() {
    assert_clean(Payment::CreditTransfer, EN16931, Profile::En16931);
}

#[test]
fn a_complete_invoice_paid_by_direct_debit_conforms_to_en16931() {
    assert_clean(Payment::DirectDebit, EN16931, Profile::En16931);
}

#[test]
fn a_complete_invoice_paid_by_transfer_conforms_to_xrechnung() {
    assert_clean(Payment::CreditTransfer, XRECHNUNG, Profile::XRechnung);
}

#[test]
fn a_complete_invoice_paid_by_direct_debit_conforms_to_xrechnung() {
    assert_clean(Payment::DirectDebit, XRECHNUNG, Profile::XRechnung);
}
