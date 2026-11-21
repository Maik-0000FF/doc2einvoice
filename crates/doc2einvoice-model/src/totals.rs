// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The amounts of the invoice as a whole: allowances and charges on document
//! level (BG-20, BG-21), the totals (BG-22) and the VAT breakdown (BG-23).

use crate::values::{Amount, Code, Percentage, Text, VatCategory};

/// An allowance (BG-20) or a charge (BG-21) on the invoice as a whole. Both
/// have this same form; which of the two it is decides where it belongs in
/// [`Invoice`](crate::Invoice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentAdjustment {
    /// The amount without VAT (BT-92, BT-99).
    pub amount: Amount,
    /// The amount the percentage is applied to (BT-93, BT-100).
    pub base_amount: Option<Amount>,
    /// The percentage (BT-94, BT-101).
    pub percentage: Option<Percentage>,
    /// The VAT category of the amount (BT-95, BT-102).
    pub vat_category: VatCategory,
    /// The VAT rate of the amount (BT-96, BT-103).
    pub vat_rate: Option<Percentage>,
    /// The reason in words (BT-97, BT-104).
    pub reason: Option<Text>,
    /// The reason as a code: UNTDID 5189 for an allowance, UNTDID 7161 for a
    /// charge (BT-98, BT-105).
    pub reason_code: Option<Code>,
}

/// The totals of the invoice (BG-22).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTotals {
    /// The sum of the net amounts of all lines (BT-106).
    pub line_net_total: Amount,
    /// The sum of the allowances on document level (BT-107).
    pub allowance_total: Option<Amount>,
    /// The sum of the charges on document level (BT-108).
    pub charge_total: Option<Amount>,
    /// The total without VAT (BT-109).
    pub total_without_vat: Amount,
    /// The total VAT (BT-110).
    pub vat_total: Option<Amount>,
    /// The total VAT in the VAT accounting currency (BT-111).
    pub vat_total_in_accounting_currency: Option<Amount>,
    /// The total with VAT (BT-112).
    pub total_with_vat: Amount,
    /// The sum of amounts already paid (BT-113).
    pub paid: Option<Amount>,
    /// The amount the total was rounded by (BT-114).
    pub rounding: Option<Amount>,
    /// The amount still to be paid (BT-115).
    pub due_for_payment: Amount,
}

/// The VAT of one category and rate (BG-23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatBreakdown {
    /// The amount the VAT is due on (BT-116).
    pub taxable_amount: Amount,
    /// The VAT due (BT-117).
    pub tax_amount: Amount,
    /// The category (BT-118).
    pub category: VatCategory,
    /// The rate (BT-119); absent only for an invoice not subject to VAT.
    pub rate: Option<Percentage>,
    /// Why the amount is exempt, in words (BT-120).
    pub exemption_reason: Option<Text>,
    /// Why the amount is exempt, a code of the VATEX list (BT-121).
    pub exemption_reason_code: Option<Code>,
}
