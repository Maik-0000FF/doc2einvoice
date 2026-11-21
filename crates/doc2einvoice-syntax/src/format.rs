// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! How a single value is written: numbers, dates and attached files. Both
//! syntaxes take them from here.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use doc2einvoice_model::Date;
use rust_decimal::Decimal;

/// A date as UBL writes it (xs:date).
const UBL_DATE_PATTERN: &str = "%Y-%m-%d";

/// A date as CII writes it, in the date format named by [`CII_DATE_FORMAT`].
const CII_DATE_PATTERN: &str = "%Y%m%d";

/// The code of UNTDID 2379 for a date written as CCYYMMDD.
pub(crate) const CII_DATE_FORMAT: &str = "102";

/// A number as both syntaxes write it: digits, a point before the decimal
/// places, a minus sign for a negative number, no grouping.
///
/// | Number  | Written |
/// |---------|---------|
/// | 10.25   | 10.25   |
/// | -3.1    | -3.1    |
/// | 0       | 0       |
/// | 1.5000  | 1.5000  (places are kept as given) |
pub(crate) fn number(value: Decimal) -> String {
    value.to_string()
}

/// A date as UBL writes it.
///
/// | Date             | Written    |
/// |------------------|------------|
/// | 21 November 2026 | 2026-11-21 |
/// | 1 January 2027   | 2027-01-01 |
pub(crate) fn ubl_date(date: Date) -> String {
    date.format(UBL_DATE_PATTERN).to_string()
}

/// A date as CII writes it.
///
/// | Date             | Written  |
/// |------------------|----------|
/// | 21 November 2026 | 20261121 |
/// | 1 January 2027   | 20270101 |
pub(crate) fn cii_date(date: Date) -> String {
    date.format(CII_DATE_PATTERN).to_string()
}

/// The content of an attached file, as Base64 with padding.
pub(crate) fn attachment(content: &[u8]) -> String {
    BASE64.encode(content)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use doc2einvoice_model::Date;
    use rust_decimal::Decimal;

    use super::{attachment, cii_date, number, ubl_date};

    fn date(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd_opt(year, month, day).unwrap_or_else(|| unreachable!("{year}-{month}-{day}"))
    }

    #[test]
    fn numbers_follow_their_table() {
        for (given, written) in [
            ("10.25", "10.25"),
            ("-3.1", "-3.1"),
            ("0", "0"),
            ("1.5000", "1.5000"),
        ] {
            let value = Decimal::from_str(given).unwrap_or_else(|error| unreachable!("{error}"));
            assert_eq!(number(value), written);
        }
    }

    #[test]
    fn dates_follow_their_tables() {
        assert_eq!(ubl_date(date(2026, 11, 21)), "2026-11-21");
        assert_eq!(ubl_date(date(2027, 1, 1)), "2027-01-01");
        assert_eq!(cii_date(date(2026, 11, 21)), "20261121");
        assert_eq!(cii_date(date(2027, 1, 1)), "20270101");
    }

    #[test]
    fn attachments_are_base64_with_padding() {
        assert_eq!(attachment(b"ab"), "YWI=");
        assert_eq!(attachment(b""), "");
    }
}
