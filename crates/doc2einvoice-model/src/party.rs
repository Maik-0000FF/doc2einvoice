// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The parties of an invoice: seller (BG-4), buyer (BG-7), payee (BG-10) and
//! the tax representative of the seller (BG-11), with the address and contact
//! groups they share.

use crate::values::{
    CountryCode, Identifier, OptionalSchemeIdentifier, RequiredSchemeIdentifier, Text,
};

/// A postal address. The standard gives the seller (BG-5), the buyer (BG-8),
/// the tax representative (BG-12) and the place of delivery (BG-15) an address
/// of this same form, each with business terms of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostalAddress {
    /// The main line: street and number, or a post office box
    /// (BT-35, BT-50, BT-64, BT-75).
    pub line_1: Option<Text>,
    /// A further line (BT-36, BT-51, BT-65, BT-76).
    pub line_2: Option<Text>,
    /// A third line (BT-162, BT-163, BT-164, BT-165).
    pub line_3: Option<Text>,
    /// The city or municipality (BT-37, BT-52, BT-66, BT-77).
    pub city: Option<Text>,
    /// The post code (BT-38, BT-53, BT-67, BT-78).
    pub post_code: Option<Text>,
    /// The subdivision of the country, such as a region or state
    /// (BT-39, BT-54, BT-68, BT-79).
    pub country_subdivision: Option<Text>,
    /// The country (BT-40, BT-55, BT-69, BT-80).
    pub country: CountryCode,
}

/// A point of contact. The seller (BG-6) and the buyer (BG-9) have one of this
/// same form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    /// A person, department or office (BT-41, BT-56).
    pub point: Option<Text>,
    /// A telephone number (BT-42, BT-57).
    pub telephone: Option<Text>,
    /// An email address (BT-43, BT-58).
    pub email: Option<Text>,
}

/// The seller (BG-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seller {
    /// The full name under which the seller is registered or trades as a
    /// legal person (BT-27).
    pub name: Text,
    /// Another name the seller is known by (BT-28).
    pub trading_name: Option<Text>,
    /// Identifiers of the seller, usually given by the buyer (BT-29).
    pub identifiers: Vec<OptionalSchemeIdentifier>,
    /// The identifier from an official register (BT-30).
    pub legal_registration: Option<OptionalSchemeIdentifier>,
    /// The VAT identifier, prefixed with the country code (BT-31).
    pub vat_identifier: Option<Identifier>,
    /// A local tax identifier or a reference to the tax status (BT-32).
    pub tax_registration: Option<Identifier>,
    /// Further legal information, such as the share capital (BT-33).
    pub additional_legal_information: Option<Text>,
    /// Where a response on the application level can be sent (BT-34).
    pub electronic_address: Option<RequiredSchemeIdentifier>,
    /// The postal address (BG-5).
    pub address: PostalAddress,
    /// The contact (BG-6).
    pub contact: Option<Contact>,
}

/// The buyer (BG-7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buyer {
    /// The full name (BT-44).
    pub name: Text,
    /// Another name the buyer is known by (BT-45).
    pub trading_name: Option<Text>,
    /// An identifier of the buyer, usually given by the seller (BT-46).
    pub identifier: Option<OptionalSchemeIdentifier>,
    /// The identifier from an official register (BT-47).
    pub legal_registration: Option<OptionalSchemeIdentifier>,
    /// The VAT identifier, prefixed with the country code (BT-48).
    pub vat_identifier: Option<Identifier>,
    /// Where the invoice should be sent (BT-49).
    pub electronic_address: Option<RequiredSchemeIdentifier>,
    /// The postal address (BG-8).
    pub address: PostalAddress,
    /// The contact (BG-9).
    pub contact: Option<Contact>,
}

/// The payee, where it is not the seller (BG-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    /// The name (BT-59).
    pub name: Text,
    /// An identifier (BT-60).
    pub identifier: Option<OptionalSchemeIdentifier>,
    /// The identifier from an official register (BT-61).
    pub legal_registration: Option<OptionalSchemeIdentifier>,
}

/// The tax representative of the seller (BG-11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxRepresentative {
    /// The full name (BT-62).
    pub name: Text,
    /// The VAT identifier, prefixed with the country code (BT-63).
    pub vat_identifier: Identifier,
    /// The postal address (BG-12).
    pub address: PostalAddress,
}
