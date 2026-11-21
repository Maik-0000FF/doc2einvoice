// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The semantic data types of EN 16931: what a single value of a business term
//! is. Text, dates and document references need no type of their own; amounts,
//! codes and identifiers carry rules that their types keep.

use rust_decimal::Decimal;

/// A written text; it may span several lines and has no length limit.
pub type Text = String;

/// A calendar day, without time of day.
pub type Date = chrono::NaiveDate;

/// A reference that identifies a document or a line of one.
pub type DocumentReference = String;

/// Monetary amounts are stated with at most this many decimal places.
const AMOUNT_DECIMAL_PLACES: u32 = 2;

/// A value that does not have the form its data type demands.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValueError {
    /// An amount has more decimal places than an amount may have.
    #[error("an amount has at most {AMOUNT_DECIMAL_PLACES} decimal places, {0} has more")]
    AmountPrecision(Decimal),
    /// A code is not written in the letters its code list uses.
    #[error("\"{code}\" is not a code of {length} capital letters")]
    CodeForm {
        /// The code as it was given.
        code: String,
        /// The number of letters a code of this list has.
        length: usize,
    },
    /// A code is none of the codes of its list.
    #[error("\"{0}\" is not a VAT category code")]
    UnknownVatCategory(String),
}

/// A monetary amount, in the currency the invoice names (BT-5), or in the VAT
/// accounting currency for BT-111.
///
/// | Given          | Result                        |
/// |----------------|-------------------------------|
/// | 10.25          | 10.25                         |
/// | 10             | 10                            |
/// | -3.10          | -3.10 (negative amounts exist, e.g. BT-115) |
/// | 10.250         | 10.250, worth 10.25: trailing zeros do not count |
/// | 10.255         | error, three decimal places   |
/// | 0.001          | error                         |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(Decimal);

impl Amount {
    /// Takes an amount that has no more than two significant decimal places.
    ///
    /// # Errors
    ///
    /// [`ValueError::AmountPrecision`] if a third or further decimal place is
    /// not zero.
    pub fn new(value: Decimal) -> Result<Self, ValueError> {
        if value.normalize().scale() > AMOUNT_DECIMAL_PLACES {
            return Err(ValueError::AmountPrecision(value));
        }
        Ok(Self(value))
    }

    /// The amount as a number.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// The price of one unit of an item, with as many decimal places as needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitPriceAmount(pub Decimal);

/// A number of units, with as many decimal places as needed; its unit of
/// measure is a business term of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity(pub Decimal);

/// A percentage, as a part of a hundred: 34.78 is 34.78 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percentage(pub Decimal);

/// Whether `code` consists of exactly `length` capital letters A to Z.
///
/// | Given  | Length | Result |
/// |--------|--------|--------|
/// | EUR    | 3      | yes    |
/// | eur    | 3      | no, small letters |
/// | EU     | 3      | no, too short |
/// | EURO   | 3      | no, too long |
/// | EÜR    | 3      | no, a letter outside A to Z |
/// | E1R    | 3      | no, a digit |
/// | (empty)| 3      | no     |
fn is_capital_letters(code: &str, length: usize) -> bool {
    code.len() == length && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

/// Number of letters of an ISO 4217 alphabetic currency code.
const CURRENCY_CODE_LENGTH: usize = 3;

/// Number of letters of an ISO 3166-1 alpha-2 country code.
const COUNTRY_CODE_LENGTH: usize = 2;

/// A currency, as an alphabetic code of ISO 4217 (EUR, USD).
///
/// Only the form is checked here. Whether the code is in the list is checked
/// by the validation rules, which carry the list itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Takes a code of three capital letters.
    ///
    /// # Errors
    ///
    /// [`ValueError::CodeForm`] for anything else.
    pub fn new(code: &str) -> Result<Self, ValueError> {
        if !is_capital_letters(code, CURRENCY_CODE_LENGTH) {
            return Err(ValueError::CodeForm {
                code: code.to_owned(),
                length: CURRENCY_CODE_LENGTH,
            });
        }
        Ok(Self(code.to_owned()))
    }

    /// The code as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A country, as an alpha-2 code of ISO 3166-1 (DE, FR).
///
/// Only the form is checked here, as for [`CurrencyCode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryCode(String);

impl CountryCode {
    /// Takes a code of two capital letters.
    ///
    /// # Errors
    ///
    /// [`ValueError::CodeForm`] for anything else.
    pub fn new(code: &str) -> Result<Self, ValueError> {
        if !is_capital_letters(code, COUNTRY_CODE_LENGTH) {
            return Err(ValueError::CodeForm {
                code: code.to_owned(),
                length: COUNTRY_CODE_LENGTH,
            });
        }
        Ok(Self(code.to_owned()))
    }

    /// The code as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A code from a code list the model names but does not hold, such as the
/// invoice type (UNTDID 1001) or the unit of measure (UN/ECE Recommendation
/// 20). The validation rules check it against its list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code(pub String);

/// The VAT category of an amount, from the part of UNTDID 5305 that EN 16931
/// allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VatCategory {
    /// S: standard or reduced rate.
    StandardRate,
    /// Z: zero rated goods.
    ZeroRated,
    /// E: exempt from VAT.
    Exempt,
    /// AE: VAT reverse charge.
    ReverseCharge,
    /// K: VAT exempt for an intra-community supply of goods and services.
    IntraCommunitySupply,
    /// G: free export item, VAT not charged.
    Export,
    /// O: services outside the scope of VAT.
    NotSubjectToVat,
    /// L: Canary Islands general indirect tax (IGIC).
    CanaryIslands,
    /// M: tax for production, services and importation in Ceuta and Melilla
    /// (IPSI).
    CeutaAndMelilla,
}

impl VatCategory {
    /// Every category, in the order of the code list.
    pub const ALL: [Self; 9] = [
        Self::StandardRate,
        Self::ZeroRated,
        Self::Exempt,
        Self::ReverseCharge,
        Self::IntraCommunitySupply,
        Self::Export,
        Self::NotSubjectToVat,
        Self::CanaryIslands,
        Self::CeutaAndMelilla,
    ];

    /// The code of the category in UNTDID 5305.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::StandardRate => "S",
            Self::ZeroRated => "Z",
            Self::Exempt => "E",
            Self::ReverseCharge => "AE",
            Self::IntraCommunitySupply => "K",
            Self::Export => "G",
            Self::NotSubjectToVat => "O",
            Self::CanaryIslands => "L",
            Self::CeutaAndMelilla => "M",
        }
    }

    /// The category a code stands for; the comparison is exact, as in the
    /// code list.
    ///
    /// # Errors
    ///
    /// [`ValueError::UnknownVatCategory`] for a code that is none of the nine.
    pub fn from_code(code: &str) -> Result<Self, ValueError> {
        Self::ALL
            .into_iter()
            .find(|category| category.code() == code)
            .ok_or_else(|| ValueError::UnknownVatCategory(code.to_owned()))
    }
}

/// An identifier, optionally formed after a named scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    /// The identifier itself.
    pub content: String,
    /// The scheme it is formed after, where the business term allows one.
    pub scheme: Option<String>,
    /// The version of that scheme.
    pub scheme_version: Option<String>,
}

/// An identifier whose scheme the standard requires: the electronic addresses
/// (BT-34, BT-49), the item standard identifier (BT-157) and the item
/// classification (BT-158).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemedIdentifier {
    /// The identifier itself.
    pub content: String,
    /// The scheme it is formed after.
    pub scheme: String,
    /// The version of that scheme.
    pub scheme_version: Option<String>,
}

/// A file attached to the invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryObject {
    /// The content of the file.
    pub content: Vec<u8>,
    /// The MIME type of the file, such as application/pdf.
    pub mime_code: String,
    /// The name of the file, unique within the invoice.
    pub filename: String,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::{Amount, CountryCode, CurrencyCode, ValueError, VatCategory, is_capital_letters};

    fn decimal(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap_or_else(|error| unreachable!("{text}: {error}"))
    }

    #[test]
    fn amounts_follow_their_table() {
        for (given, accepted) in [
            ("10.25", true),
            ("10", true),
            ("-3.10", true),
            ("10.250", true),
            ("10.255", false),
            ("0.001", false),
        ] {
            assert_eq!(Amount::new(decimal(given)).is_ok(), accepted, "{given}");
        }
    }

    #[test]
    fn an_amount_keeps_its_value() {
        assert_eq!(
            Amount::new(decimal("10.250")).map(Amount::value),
            Ok(decimal("10.250"))
        );
    }

    #[test]
    fn capital_letters_follow_their_table() {
        for (given, accepted) in [
            ("EUR", true),
            ("eur", false),
            ("EU", false),
            ("EURO", false),
            ("EÜR", false),
            ("E1R", false),
            ("", false),
        ] {
            assert_eq!(is_capital_letters(given, 3), accepted, "{given}");
        }
    }

    #[test]
    fn codes_are_checked_for_their_length() {
        assert!(CurrencyCode::new("EUR").is_ok());
        assert!(CurrencyCode::new("DE").is_err());
        assert!(CountryCode::new("DE").is_ok());
        assert_eq!(
            CountryCode::new("DEU"),
            Err(ValueError::CodeForm {
                code: "DEU".to_owned(),
                length: 2
            })
        );
    }

    #[test]
    fn every_vat_category_reads_back_from_its_code() {
        for category in VatCategory::ALL {
            assert_eq!(VatCategory::from_code(category.code()), Ok(category));
        }
    }

    #[test]
    fn unknown_or_differently_written_vat_codes_are_refused() {
        for code in ["s", "X", "", "AE ", "VAT"] {
            assert!(VatCategory::from_code(code).is_err(), "{code:?}");
        }
    }
}
