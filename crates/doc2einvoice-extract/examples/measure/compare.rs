// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Whether a value of the draft is the value the XML holds. Only the XML is
//! turned into the form of the draft; the draft is taken as it stands, so a
//! value in another form counts as wrong.

use std::str::FromStr;

use chrono::NaiveDate;
use doc2einvoice_model::{DATE_FORMAT, DataType};
use iban::{Iban, IbanLike};
use rust_decimal::Decimal;

/// A date as a Cross Industry Invoice writes it (format 102).
const CII_DATE: &str = "%Y%m%d";

/// The value `xml` holds, already free of surrounding and repeated white
/// space, in the form of the draft for `data_type`.
///
/// | Data type | XML            | Form           |
/// |-----------|----------------|----------------|
/// | date      | `20261121`     | `2026-11-21`   |
/// | date      | `2026-11-21` (another format) | as given |
/// | amount    | `1234.50`      | `1234.5`       |
/// | amount    | `-3.10`        | `-3.1`         |
/// | text      | `Muster GmbH`  | `Muster GmbH`  |
/// | identifier | `DE02 1203 0000 0000 2020 51` | `DE02120300000000202051` (an IBAN, in its electronic form) |
/// | identifier | `210111 mit LV` | `210111 mit LV` |
pub fn canonical(data_type: DataType, xml: &str) -> String {
    match data_type {
        DataType::Date => NaiveDate::parse_from_str(xml, CII_DATE).map_or_else(
            |_| xml.to_owned(),
            |date| date.format(DATE_FORMAT).to_string(),
        ),
        DataType::Amount
        | DataType::UnitPriceAmount
        | DataType::Quantity
        | DataType::Percentage => Decimal::from_str(xml)
            .map_or_else(|_| xml.to_owned(), |value| value.normalize().to_string()),
        DataType::Identifier => xml
            .parse::<Iban>()
            .map_or_else(|_| xml.to_owned(), |iban| iban.electronic_str().to_owned()),
        DataType::Code | DataType::DocumentReference | DataType::Text | DataType::BinaryObject => {
            xml.to_owned()
        }
    }
}

/// Whether the draft value `found` is the value `expected`, the canonical
/// form of the XML, stands for. Numbers are compared as numbers, everything
/// else as written.
///
/// | Data type | Draft          | Canonical XML  | Same |
/// |-----------|----------------|----------------|------|
/// | amount    | `1234.5`       | `1234.5`       | yes  |
/// | amount    | `1234.50`      | `1234.5`       | yes  |
/// | amount    | `1234.5`       | `1234.55`      | no   |
/// | amount    | ` 1234.5`      | `1234.5`       | no   |
/// | amount    | `1.234,50`     | `1234.5`       | no   |
/// | date      | `2026-11-21`   | `2026-11-21`   | yes  |
/// | date      | `2026-11-21 `  | `2026-11-21`   | no   |
/// | date      | `20261121`     | `2026-11-21`   | no   |
/// | text      | `Muster GmbH`  | `Muster GmbH`  | yes  |
/// | text      | ` Muster GmbH` | `Muster GmbH`  | no   |
/// | text      | `muster gmbh`  | `Muster GmbH`  | no   |
pub fn same(data_type: DataType, found: &str, expected: &str) -> bool {
    match data_type {
        DataType::Amount
        | DataType::UnitPriceAmount
        | DataType::Quantity
        | DataType::Percentage => match (Decimal::from_str(found), Decimal::from_str(expected)) {
            (Ok(found), Ok(expected)) => found == expected,
            _ => false,
        },
        DataType::Date
        | DataType::Code
        | DataType::DocumentReference
        | DataType::Identifier
        | DataType::Text
        | DataType::BinaryObject => found == expected,
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
        assert_eq!(canonical(DataType::Text, "Muster GmbH"), "Muster GmbH");
        assert_eq!(
            canonical(DataType::Identifier, "DE02 1203 0000 0000 2020 51"),
            "DE02120300000000202051"
        );
        assert_eq!(
            canonical(DataType::Identifier, "210111 mit LV"),
            "210111 mit LV"
        );
    }

    #[test]
    fn comparisons_follow_their_table() {
        let rows = [
            (DataType::Amount, "1234.5", "1234.5", true),
            (DataType::Amount, "1234.50", "1234.5", true),
            (DataType::Amount, "1234.5", "1234.55", false),
            (DataType::Amount, " 1234.5", "1234.5", false),
            (DataType::Amount, "1.234,50", "1234.5", false),
            (DataType::Date, "2026-11-21", "2026-11-21", true),
            (DataType::Date, "2026-11-21 ", "2026-11-21", false),
            (DataType::Date, "20261121", "2026-11-21", false),
            (DataType::Text, "Muster GmbH", "Muster GmbH", true),
            (DataType::Text, " Muster GmbH", "Muster GmbH", false),
            (DataType::Text, "muster gmbh", "Muster GmbH", false),
        ];
        for (data_type, found, expected, result) in rows {
            assert_eq!(same(data_type, found, expected), result, "{found:?}");
        }
    }
}
