// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The semantic data types of EN 16931: what a single value of a business term
//! is. Text, dates and document references need no type of their own; amounts,
//! codes and identifiers carry rules that their types keep.
//!
//! A type keeps a rule only where the rule is about the form of one value.
//! Rules about content, such as a net price that must not be negative
//! (BR-27), are left to the validation rules, which check them against the
//! whole invoice; a type here would only check them a second time.

use rust_decimal::Decimal;

/// A written text; it may span several lines and has no length limit.
pub type Text = String;

/// A calendar day, without time of day.
pub type Date = chrono::NaiveDate;

/// A reference that identifies a document or a line of one.
pub type DocumentReference = String;

/// Monetary amounts are stated with at most this many decimal places.
const AMOUNT_DECIMAL_PLACES: u32 = 2;

/// Number of letters of an ISO 4217 alphabetic currency code.
const CURRENCY_CODE_LENGTH: usize = 3;

/// Number of letters of an ISO 3166-1 alpha-2 country code.
const COUNTRY_CODE_LENGTH: usize = 2;

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
    /// A currency code is none of the codes the standard allows.
    #[error("\"{0}\" is not a currency code the standard allows")]
    UnknownCurrency(String),
}

/// A monetary amount, in the currency the invoice names, or in the VAT
/// accounting currency for the VAT total in that currency.
///
/// The amount is kept without trailing zeros: the validation rules count the
/// decimal places as written, so 10.250 must not reach the XML as written.
///
/// | Given   | Kept    | Why                                        |
/// |---------|---------|--------------------------------------------|
/// | 10.25   | 10.25   |                                            |
/// | 10      | 10      |                                            |
/// | -3.10   | -3.1    | negative amounts exist, such as an amount due after overpayment |
/// | 10.250  | 10.25   | trailing zeros are dropped                 |
/// | 10.00   | 10      | trailing zeros are dropped                 |
/// | 10.255  | error   | three significant decimal places           |
/// | 0.001   | error   |                                            |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(Decimal);

impl Amount {
    /// Takes an amount that has no more than two significant decimal places,
    /// and keeps it without trailing zeros.
    ///
    /// # Errors
    ///
    /// [`ValueError::AmountPrecision`] if a third or further decimal place is
    /// not zero.
    pub fn new(value: Decimal) -> Result<Self, ValueError> {
        let normalized = value.normalize();
        if normalized.scale() > AMOUNT_DECIMAL_PLACES {
            return Err(ValueError::AmountPrecision(value));
        }
        Ok(Self(normalized))
    }

    /// The amount as a number.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }
}

/// The price of one unit of an item, with as many decimal places as needed.
/// Any number is a valid form; see the module documentation for the rules on
/// content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitPriceAmount(pub Decimal);

/// A number of units, with as many decimal places as needed; its unit of
/// measure is a business term of its own. Any number is a valid form, a
/// negative one included, as on a line that is credited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity(pub Decimal);

/// A percentage, as a part of a hundred: 34.78 is 34.78 %. Any number is a
/// valid form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Percentage(pub Decimal);

/// Whether `code` consists of exactly `length` capital letters A to Z.
///
/// | Given   | Length | Result              |
/// |---------|--------|---------------------|
/// | EUR     | 3      | yes                 |
/// | eur     | 3      | no, small letters   |
/// | EU      | 3      | no, too short       |
/// | EURO    | 3      | no, too long        |
/// | EÜR     | 3      | no, a letter outside A to Z |
/// | E1R     | 3      | no, a digit         |
/// | (empty) | 3      | no                  |
fn is_capital_letters(code: &str, length: usize) -> bool {
    code.len() == length && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

/// A code of `N` capital letters A to Z, as the ISO lists of countries use
/// them.
///
/// Only the form is checked here. Whether the code is in its list is checked
/// by the validation rules, which carry the list itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetterCode<const N: usize>(String);

impl<const N: usize> LetterCode<N> {
    /// Takes a code of `N` capital letters.
    ///
    /// # Errors
    ///
    /// [`ValueError::CodeForm`] for anything else.
    pub fn new(code: &str) -> Result<Self, ValueError> {
        if !is_capital_letters(code, N) {
            return Err(ValueError::CodeForm {
                code: code.to_owned(),
                length: N,
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

/// The currency codes the standard allows (rule BR-CL-04): the alphabetic
/// codes of ISO 4217, in the order of the rule.
pub const CURRENCY_CODES: [&str; 178] = [
    "AED", "AFN", "ALL", "AMD", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT", "BHD",
    "BIF", "BMD", "BND", "BOB", "BOV", "BRL", "BSD", "BTN", "BWP", "BYN", "BZD", "CAD", "CDF",
    "CHE", "CHF", "CHW", "CLF", "CLP", "CNH", "CNY", "COP", "COU", "CRC", "CUP", "CVE", "CZK",
    "DJF", "DKK", "DOP", "DZD", "EGP", "ERN", "ETB", "EUR", "FJD", "FKP", "GBP", "GEL", "GHS",
    "GIP", "GMD", "GNF", "GTQ", "GYD", "HKD", "HNL", "HTG", "HUF", "IDR", "ILS", "INR", "IQD",
    "IRR", "ISK", "JMD", "JOD", "JPY", "KES", "KGS", "KHR", "KMF", "KPW", "KRW", "KWD", "KYD",
    "KZT", "LAK", "LBP", "LKR", "LRD", "LSL", "LYD", "MAD", "MDL", "MGA", "MKD", "MMK", "MNT",
    "MOP", "MRU", "MUR", "MVR", "MWK", "MXN", "MXV", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK",
    "NPR", "NZD", "OMR", "PAB", "PEN", "PGK", "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD",
    "RUB", "RWF", "SAR", "SBD", "SCR", "SDG", "SEK", "SGD", "SHP", "SLE", "SOS", "SRD", "SSP",
    "STN", "SVC", "SYP", "SZL", "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TWD", "TZS",
    "UAH", "UGX", "USD", "USN", "UYI", "UYU", "UYW", "UZS", "VES", "VED", "VND", "VUV", "WST",
    "XAF", "XAG", "XAU", "XBA", "XBB", "XBC", "XBD", "XCD", "XCG", "XDR", "XOF", "XPD", "XPF",
    "XPT", "XSU", "XTS", "XUA", "XXX", "YER", "ZAR", "ZMW", "ZWG",
];

/// A currency, as an alphabetic code of ISO 4217 that the standard allows
/// (EUR, USD).
///
/// | Given | Result                              |
/// |-------|-------------------------------------|
/// | EUR   | the code                            |
/// | CHF   | the code                            |
/// | eur   | error, not three capital letters    |
/// | EURO  | error, not three capital letters    |
/// | VAT   | error, three capitals but no currency |
/// | DEM   | error, no longer a currency of ISO 4217 |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyCode(LetterCode<CURRENCY_CODE_LENGTH>);

impl CurrencyCode {
    /// Takes a currency code the standard allows.
    ///
    /// # Errors
    ///
    /// [`ValueError::CodeForm`] for anything but three capital letters,
    /// [`ValueError::UnknownCurrency`] for a code outside the list.
    pub fn new(code: &str) -> Result<Self, ValueError> {
        let form = LetterCode::new(code)?;
        if !CURRENCY_CODES.contains(&code) {
            return Err(ValueError::UnknownCurrency(code.to_owned()));
        }
        Ok(Self(form))
    }

    /// The code as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// A country, as an alpha-2 code of ISO 3166-1 (DE, FR).
pub type CountryCode = LetterCode<COUNTRY_CODE_LENGTH>;

/// A code from a code list the model names but does not hold, such as the
/// invoice type (UNTDID 1001) or the unit of measure (UN/ECE Recommendation
/// 20). Any text is a valid form; the validation rules check it against its
/// list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code(pub String);

/// The VAT category of an amount, from the part of UNTDID 5305 that EN 16931
/// allows (BR-CL-18).
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
    /// B: transferred VAT, the split payment of Italy.
    TransferredVat,
}

impl VatCategory {
    /// Every category, in the order of the code list.
    pub const ALL: [Self; 10] = [
        Self::StandardRate,
        Self::ZeroRated,
        Self::Exempt,
        Self::ReverseCharge,
        Self::IntraCommunitySupply,
        Self::Export,
        Self::NotSubjectToVat,
        Self::CanaryIslands,
        Self::CeutaAndMelilla,
        Self::TransferredVat,
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
            Self::TransferredVat => "B",
        }
    }

    /// The category a code stands for; the comparison is exact, as in the
    /// code list.
    ///
    /// # Errors
    ///
    /// [`ValueError::UnknownVatCategory`] for a code that is none of the ten.
    pub fn from_code(code: &str) -> Result<Self, ValueError> {
        Self::ALL
            .into_iter()
            .find(|category| category.code() == code)
            .ok_or_else(|| ValueError::UnknownVatCategory(code.to_owned()))
    }
}

/// An identifier the standard gives no scheme, such as the invoice number or
/// a VAT identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier(pub String);

/// An identifier that may name the scheme it is formed after, such as a
/// register number with the code of its register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalSchemeIdentifier {
    /// The identifier itself.
    pub content: String,
    /// The scheme it is formed after.
    pub scheme: Option<String>,
}

/// An identifier that must name the scheme it is formed after: the electronic
/// addresses and the item standard identifier (BR-62, BR-63, BR-64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredSchemeIdentifier {
    /// The identifier itself.
    pub content: String,
    /// The scheme it is formed after.
    pub scheme: String,
}

/// A classification of an item: a code, the scheme it is taken from, and
/// optionally the version of that scheme (BR-65).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemClassification {
    /// The code.
    pub content: String,
    /// The scheme, from UNTDID 7143.
    pub scheme: String,
    /// The version of the scheme.
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

    use super::{
        Amount, COUNTRY_CODE_LENGTH, CURRENCY_CODE_LENGTH, CountryCode, CurrencyCode, ValueError,
        VatCategory, is_capital_letters,
    };

    fn decimal(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap_or_else(|error| unreachable!("{text}: {error}"))
    }

    #[test]
    fn amounts_follow_their_table() {
        for (given, kept) in [
            ("10.25", Some("10.25")),
            ("10", Some("10")),
            ("-3.10", Some("-3.1")),
            ("10.250", Some("10.25")),
            ("10.00", Some("10")),
            ("10.255", None),
            ("0.001", None),
        ] {
            let amount = Amount::new(decimal(given)).ok();
            assert_eq!(
                amount.map(|amount| amount.value().to_string()),
                kept.map(str::to_owned),
                "{given}"
            );
        }
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
            assert_eq!(
                is_capital_letters(given, CURRENCY_CODE_LENGTH),
                accepted,
                "{given}"
            );
        }
    }

    #[test]
    fn codes_are_checked_for_their_length() {
        assert!(CurrencyCode::new("EUR").is_ok());
        assert!(CurrencyCode::new("CHF").is_ok());
        for code in ["eur", "EURO"] {
            assert!(
                matches!(CurrencyCode::new(code), Err(ValueError::CodeForm { .. })),
                "{code}"
            );
        }
        for code in ["VAT", "DEM"] {
            assert!(
                matches!(CurrencyCode::new(code), Err(ValueError::UnknownCurrency(_))),
                "{code}"
            );
        }
        assert!(CurrencyCode::new("DE").is_err());
        assert!(CountryCode::new("DE").is_ok());
        assert_eq!(
            CountryCode::new("DEU"),
            Err(ValueError::CodeForm {
                code: "DEU".to_owned(),
                length: COUNTRY_CODE_LENGTH
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
    fn the_vat_categories_are_those_of_the_code_list() {
        let codes: Vec<&str> = VatCategory::ALL
            .iter()
            .map(|category| category.code())
            .collect();
        assert_eq!(codes, ["S", "Z", "E", "AE", "K", "G", "O", "L", "M", "B"]);
    }

    #[test]
    fn unknown_or_differently_written_vat_codes_are_refused() {
        for code in ["s", "X", "", "AE ", "VAT"] {
            assert!(VatCategory::from_code(code).is_err(), "{code:?}");
        }
    }
}
