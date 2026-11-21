// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether a value of the draft is the value the XML holds: both in the one
//! form of their data type, the XML first turned into it.

use std::str::FromStr;

use chrono::NaiveDate;
use doc2einvoice_model::DataType;
use rust_decimal::Decimal;

/// A date as a Cross Industry Invoice writes it (format 102).
const CII_DATE: &str = "%Y%m%d";

/// A date as the draft writes it.
const DRAFT_DATE: &str = "%Y-%m-%d";

/// The value of `xml` in the form of the draft, for `data_type`.
///
/// | Data type | XML        | Form         |
/// |-----------|------------|--------------|
/// | date      | `20261121` | `2026-11-21` |
/// | date      | `2026-11-21` (other format) | as given |
/// | amount    | `1234.50`  | `1234.5`     |
/// | amount    | `-3.10`    | `-3.1`       |
/// | text      | `Muster  GmbH` | `Muster GmbH` |
pub fn canonical(data_type: DataType, xml: &str) -> String {
    match data_type {
        DataType::Date => NaiveDate::parse_from_str(xml, CII_DATE).map_or_else(
            |_| xml.to_owned(),
            |date| date.format(DRAFT_DATE).to_string(),
        ),
        DataType::Amount
        | DataType::UnitPriceAmount
        | DataType::Quantity
        | DataType::Percentage => Decimal::from_str(xml)
            .map_or_else(|_| xml.to_owned(), |value| value.normalize().to_string()),
        DataType::Code
        | DataType::DocumentReference
        | DataType::Identifier
        | DataType::Text
        | DataType::BinaryObject => xml.split_whitespace().collect::<Vec<_>>().join(" "),
    }
}

/// Whether the draft value `found` is the value `expected` stands for.
///
/// | Data type | Draft          | Canonical XML  | Same |
/// |-----------|----------------|----------------|------|
/// | amount    | `1234.50`      | `1234.5`       | yes  |
/// | amount    | `1234.5`       | `1234.55`      | no   |
/// | text      | ` Muster GmbH` | `Muster GmbH`  | yes  |
/// | text      | `muster gmbh`  | `Muster GmbH`  | no   |
pub fn same(data_type: DataType, found: &str, expected: &str) -> bool {
    match data_type {
        DataType::Amount
        | DataType::UnitPriceAmount
        | DataType::Quantity
        | DataType::Percentage => {
            match (Decimal::from_str(found.trim()), Decimal::from_str(expected)) {
                (Ok(found), Ok(expected)) => found == expected,
                _ => false,
            }
        }
        _ => canonical(data_type, found) == expected,
    }
}

#[cfg(test)]
mod tests {
    use doc2einvoice_model::DataType;

    use super::{canonical, same};

    #[test]
    fn canonical_forms_follow_their_table() {
        assert_eq!(canonical(DataType::Date, "20261121"), "2026-11-21");
        assert_eq!(canonical(DataType::Date, "2026-11-21"), "2026-11-21");
        assert_eq!(canonical(DataType::Amount, "1234.50"), "1234.5");
        assert_eq!(canonical(DataType::Amount, "-3.10"), "-3.1");
        assert_eq!(canonical(DataType::Text, "Muster  GmbH"), "Muster GmbH");
    }

    #[test]
    fn comparisons_follow_their_table() {
        assert!(same(DataType::Amount, "1234.50", "1234.5"));
        assert!(!same(DataType::Amount, "1234.5", "1234.55"));
        assert!(same(DataType::Text, " Muster GmbH", "Muster GmbH"));
        assert!(!same(DataType::Text, "muster gmbh", "Muster GmbH"));
    }
}
