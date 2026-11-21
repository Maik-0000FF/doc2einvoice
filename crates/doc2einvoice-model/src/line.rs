// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! A line of the invoice (BG-25) and what belongs to it.

use crate::delivery::Period;
use crate::values::{
    Amount, Code, CountryCode, DocumentReference, Identifier, Percentage, Quantity,
    SchemedIdentifier, Text, UnitPriceAmount, VatCategory,
};

/// A line of the invoice (BG-25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceLine {
    /// The identifier of the line (BT-126).
    pub identifier: Identifier,
    /// A note on the line (BT-127).
    pub note: Option<Text>,
    /// An object the line refers to, such as a meter number (BT-128).
    pub object_identifier: Option<Identifier>,
    /// The quantity invoiced (BT-129).
    pub quantity: Quantity,
    /// The unit of measure of the quantity, UN/ECE Recommendation 20 or 21
    /// (BT-130).
    pub unit: Code,
    /// The net amount: without VAT, with the allowances and charges of the
    /// line (BT-131).
    pub net_amount: Amount,
    /// The line of the purchase order (BT-132).
    pub purchase_order_line: Option<DocumentReference>,
    /// Where the buyer books the line (BT-133).
    pub buyer_accounting_reference: Option<Text>,
    /// The period of the line (BG-26).
    pub period: Option<Period>,
    /// Allowances on the line (BG-27).
    pub allowances: Vec<LineAdjustment>,
    /// Charges on the line (BG-28).
    pub charges: Vec<LineAdjustment>,
    /// The price (BG-29).
    pub price: PriceDetails,
    /// The VAT of the line (BG-30).
    pub vat: LineVat,
    /// The item (BG-31).
    pub item: ItemInformation,
}

/// An allowance (BG-27) or a charge (BG-28) on a line. Both have this same
/// form; which of the two it is decides where it belongs in [`InvoiceLine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAdjustment {
    /// The amount without VAT (BT-136, BT-141).
    pub amount: Amount,
    /// The amount the percentage is applied to (BT-137, BT-142).
    pub base_amount: Option<Amount>,
    /// The percentage (BT-138, BT-143).
    pub percentage: Option<Percentage>,
    /// The reason in words (BT-139, BT-144).
    pub reason: Option<Text>,
    /// The reason as a code: UNTDID 5189 for an allowance, UNTDID 7161 for a
    /// charge (BT-140, BT-145).
    pub reason_code: Option<Code>,
}

/// The price of the item (BG-29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceDetails {
    /// The net price, after the price discount (BT-146).
    pub net_price: UnitPriceAmount,
    /// The discount subtracted from the gross price (BT-147).
    pub discount: Option<UnitPriceAmount>,
    /// The gross price, before the discount (BT-148).
    pub gross_price: Option<UnitPriceAmount>,
    /// The number of units the price applies to (BT-149).
    pub base_quantity: Option<Quantity>,
    /// The unit of that number, the same as the unit of the line (BT-150).
    pub base_quantity_unit: Option<Code>,
}

/// The VAT of a line (BG-30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineVat {
    /// The category (BT-151).
    pub category: VatCategory,
    /// The rate (BT-152).
    pub rate: Option<Percentage>,
}

/// The item invoiced (BG-31).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInformation {
    /// The name (BT-153).
    pub name: Text,
    /// A description, in more detail than the name (BT-154).
    pub description: Option<Text>,
    /// The identifier the seller gives the item (BT-155).
    pub seller_identifier: Option<Identifier>,
    /// The identifier the buyer gives the item (BT-156).
    pub buyer_identifier: Option<Identifier>,
    /// An identifier after a registered scheme, such as a GTIN (BT-157).
    pub standard_identifier: Option<SchemedIdentifier>,
    /// Classifications of the item, after UNTDID 7143 (BT-158).
    pub classifications: Vec<SchemedIdentifier>,
    /// The country the item comes from (BT-159).
    pub country_of_origin: Option<CountryCode>,
    /// Properties of the item, such as its colour (BG-32).
    pub attributes: Vec<ItemAttribute>,
}

/// A property of the item (BG-32).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemAttribute {
    /// The name of the property, such as "colour" (BT-160).
    pub name: Text,
    /// Its value, such as "red" (BT-161).
    pub value: Text,
}
