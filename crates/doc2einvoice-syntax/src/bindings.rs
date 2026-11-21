// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where each business term and group of EN 16931 stands in the documents
//! this crate writes: an XPath for a UBL invoice, a Cross Industry Invoice and
//! a UBL credit note. Several places a reader may meet are joined with `|`.
//! The prefixes are those of [`crate::namespaces`]: `Invoice` and
//! `CreditNote` for the UBL documents, `cac`, `cbc`, `rsm`, `ram`, `udt` and
//! `qdt`.

/// The table, a header line and one line per business term or group, the
/// fields separated by tabs.
const TABLE: &str = include_str!("../data/bindings.tsv");

/// The place of one business term or group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    /// The identifier, such as `BT-27` or `BG-4`.
    pub id: &'static str,
    /// The path in a UBL invoice.
    pub ubl: &'static str,
    /// The path in a Cross Industry Invoice.
    pub cii: &'static str,
    /// The path in a UBL credit note.
    pub ubl_credit_note: &'static str,
}

/// The fields of a line of the table.
const FIELDS: usize = 4;

/// Every binding, in the order of the table: the business terms by number,
/// then the business groups. A test reads every line of the table.
#[must_use]
pub fn bindings() -> Vec<Binding> {
    TABLE
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&'static str> = line.split('\t').collect();
            match fields[..] {
                [id, ubl, cii, ubl_credit_note] if fields.len() == FIELDS => Some(Binding {
                    id,
                    ubl,
                    cii,
                    ubl_credit_note,
                }),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{TABLE, bindings};

    /// Business terms (164) and groups (32).
    const ROWS: usize = 196;

    #[test]
    fn every_line_of_the_table_is_a_binding() {
        assert_eq!(TABLE.lines().skip(1).count(), ROWS);
        assert_eq!(bindings().len(), ROWS);
    }

    #[test]
    fn a_binding_names_its_paths() {
        let seller = bindings().into_iter().find(|binding| binding.id == "BT-27");
        assert!(
            seller.is_some_and(|binding| binding.cii.ends_with("ram:SellerTradeParty/ram:Name"))
        );
    }
}
