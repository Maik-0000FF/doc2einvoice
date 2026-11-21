// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The business terms of EN 16931 as a list: the number, the name the
//! standard gives each, its semantic data type, and the business group it
//! belongs to. Labels for a form and the comparison of values read from a
//! page take them from here.

/// What the identifier of a business term starts with, as in `BT-27`.
const TERM_PREFIX: &str = "BT-";

/// What the identifier of a business group starts with, as in `BG-25`.
const GROUP_PREFIX: &str = "BG-";

/// The business groups are numbered from 1 to this.
const LAST_GROUP: u16 = 32;

/// The semantic data type of a business term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// A monetary amount, at most two decimal places.
    Amount,
    /// An attached file.
    BinaryObject,
    /// A code from a code list.
    Code,
    /// A calendar day.
    Date,
    /// A reference to a document.
    DocumentReference,
    /// An identifier, possibly after a scheme.
    Identifier,
    /// A percentage.
    Percentage,
    /// A number of units.
    Quantity,
    /// A written text.
    Text,
    /// The price of one unit.
    UnitPriceAmount,
}

/// A business term (BT) of EN 16931.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Term {
    /// The number, 27 for BT-27.
    pub number: u16,
    /// The name the standard gives it, such as "Seller name".
    pub name: &'static str,
    /// Its semantic data type.
    pub data_type: DataType,
    /// The number of the business group it belongs to directly, none for a
    /// term of the invoice itself.
    pub group: Option<u16>,
}

impl Term {
    /// The identifier as the standard writes it, such as `BT-27`.
    #[must_use]
    pub fn id(&self) -> String {
        format!("{TERM_PREFIX}{}", self.number)
    }
}

const fn term(number: u16, name: &'static str, data_type: DataType, group: Option<u16>) -> Term {
    Term {
        number,
        name,
        data_type,
        group,
    }
}

/// Every business term, by number. BT-4 is not assigned in the standard.
pub const TERMS: [Term; 164] = [
    term(1, "Invoice number", DataType::Identifier, None),
    term(2, "Invoice issue date", DataType::Date, None),
    term(3, "Invoice type code", DataType::Code, None),
    term(5, "Invoice currency code", DataType::Code, None),
    term(6, "VAT accounting currency code", DataType::Code, None),
    term(7, "Value added tax point date", DataType::Date, None),
    term(8, "Value added tax point date code", DataType::Code, None),
    term(9, "Payment due date", DataType::Date, None),
    term(10, "Buyer reference", DataType::Text, None),
    term(11, "Project reference", DataType::DocumentReference, None),
    term(12, "Contract reference", DataType::DocumentReference, None),
    term(
        13,
        "Purchase order reference",
        DataType::DocumentReference,
        None,
    ),
    term(
        14,
        "Sales order reference",
        DataType::DocumentReference,
        None,
    ),
    term(
        15,
        "Receiving advice reference",
        DataType::DocumentReference,
        None,
    ),
    term(
        16,
        "Despatch advice reference",
        DataType::DocumentReference,
        None,
    ),
    term(
        17,
        "Tender or lot reference",
        DataType::DocumentReference,
        None,
    ),
    term(18, "Invoiced object identifier", DataType::Identifier, None),
    term(19, "Buyer accounting reference", DataType::Text, None),
    term(20, "Payment terms", DataType::Text, None),
    term(21, "Invoice note subject code", DataType::Code, Some(1)),
    term(22, "Invoice note", DataType::Text, Some(1)),
    term(23, "Business process type", DataType::Text, Some(2)),
    term(
        24,
        "Specification identifier",
        DataType::Identifier,
        Some(2),
    ),
    term(
        25,
        "Preceding Invoice reference",
        DataType::DocumentReference,
        Some(3),
    ),
    term(26, "Preceding Invoice issue date", DataType::Date, Some(3)),
    term(27, "Seller name", DataType::Text, Some(4)),
    term(28, "Seller trading name", DataType::Text, Some(4)),
    term(29, "Seller identifier", DataType::Identifier, Some(4)),
    term(
        30,
        "Seller legal registration identifier",
        DataType::Identifier,
        Some(4),
    ),
    term(31, "Seller VAT identifier", DataType::Identifier, Some(4)),
    term(
        32,
        "Seller tax registration identifier",
        DataType::Identifier,
        Some(4),
    ),
    term(
        33,
        "Seller additional legal information",
        DataType::Text,
        Some(4),
    ),
    term(
        34,
        "Seller electronic address",
        DataType::Identifier,
        Some(4),
    ),
    term(35, "Seller address line 1", DataType::Text, Some(5)),
    term(36, "Seller address line 2", DataType::Text, Some(5)),
    term(37, "Seller city", DataType::Text, Some(5)),
    term(38, "Seller post code", DataType::Text, Some(5)),
    term(39, "Seller country subdivision", DataType::Text, Some(5)),
    term(40, "Seller country code", DataType::Code, Some(5)),
    term(41, "Seller contact point", DataType::Text, Some(6)),
    term(
        42,
        "Seller contact telephone number",
        DataType::Text,
        Some(6),
    ),
    term(43, "Seller contact email address", DataType::Text, Some(6)),
    term(44, "Buyer name", DataType::Text, Some(7)),
    term(45, "Buyer trading name", DataType::Text, Some(7)),
    term(46, "Buyer identifier", DataType::Identifier, Some(7)),
    term(
        47,
        "Buyer legal registration identifier",
        DataType::Identifier,
        Some(7),
    ),
    term(48, "Buyer VAT identifier", DataType::Identifier, Some(7)),
    term(
        49,
        "Buyer electronic address",
        DataType::Identifier,
        Some(7),
    ),
    term(50, "Buyer address line 1", DataType::Text, Some(8)),
    term(51, "Buyer address line 2", DataType::Text, Some(8)),
    term(52, "Buyer city", DataType::Text, Some(8)),
    term(53, "Buyer post code", DataType::Text, Some(8)),
    term(54, "Buyer country subdivision", DataType::Text, Some(8)),
    term(55, "Buyer country code", DataType::Code, Some(8)),
    term(56, "Buyer contact point", DataType::Text, Some(9)),
    term(
        57,
        "Buyer contact telephone number",
        DataType::Text,
        Some(9),
    ),
    term(58, "Buyer contact email address", DataType::Text, Some(9)),
    term(59, "Payee name", DataType::Text, Some(10)),
    term(60, "Payee identifier", DataType::Identifier, Some(10)),
    term(
        61,
        "Payee legal registration identifier",
        DataType::Identifier,
        Some(10),
    ),
    term(
        62,
        "Seller tax representative name",
        DataType::Text,
        Some(11),
    ),
    term(
        63,
        "Seller tax representative VAT identifier",
        DataType::Identifier,
        Some(11),
    ),
    term(
        64,
        "Tax representative address line 1",
        DataType::Text,
        Some(12),
    ),
    term(
        65,
        "Tax representative address line 2",
        DataType::Text,
        Some(12),
    ),
    term(66, "Tax representative city", DataType::Text, Some(12)),
    term(67, "Tax representative post code", DataType::Text, Some(12)),
    term(
        68,
        "Tax representative country subdivision",
        DataType::Text,
        Some(12),
    ),
    term(
        69,
        "Tax representative country code",
        DataType::Code,
        Some(12),
    ),
    term(70, "Deliver to party name", DataType::Text, Some(13)),
    term(
        71,
        "Deliver to location identifier",
        DataType::Identifier,
        Some(13),
    ),
    term(72, "Actual delivery date", DataType::Date, Some(13)),
    term(73, "Invoicing period start date", DataType::Date, Some(14)),
    term(74, "Invoicing period end date", DataType::Date, Some(14)),
    term(75, "Deliver to address line 1", DataType::Text, Some(15)),
    term(76, "Deliver to address line 2", DataType::Text, Some(15)),
    term(77, "Deliver to city", DataType::Text, Some(15)),
    term(78, "Deliver to post code", DataType::Text, Some(15)),
    term(
        79,
        "Deliver to country subdivision",
        DataType::Text,
        Some(15),
    ),
    term(80, "Deliver to country code", DataType::Code, Some(15)),
    term(81, "Payment means type code", DataType::Code, Some(16)),
    term(82, "Payment means text", DataType::Text, Some(16)),
    term(83, "Remittance information", DataType::Text, Some(16)),
    term(
        84,
        "Payment account identifier",
        DataType::Identifier,
        Some(17),
    ),
    term(85, "Payment account name", DataType::Text, Some(17)),
    term(
        86,
        "Payment service provider identifier",
        DataType::Identifier,
        Some(17),
    ),
    term(
        87,
        "Payment card primary account number",
        DataType::Text,
        Some(18),
    ),
    term(88, "Payment card holder name", DataType::Text, Some(18)),
    term(
        89,
        "Mandate reference identifier",
        DataType::Identifier,
        Some(19),
    ),
    term(
        90,
        "Bank assigned creditor identifier",
        DataType::Identifier,
        Some(19),
    ),
    term(
        91,
        "Debited account identifier",
        DataType::Identifier,
        Some(19),
    ),
    term(
        92,
        "Document level allowance amount",
        DataType::Amount,
        Some(20),
    ),
    term(
        93,
        "Document level allowance base amount",
        DataType::Amount,
        Some(20),
    ),
    term(
        94,
        "Document level allowance percentage",
        DataType::Percentage,
        Some(20),
    ),
    term(
        95,
        "Document level allowance VAT category code",
        DataType::Code,
        Some(20),
    ),
    term(
        96,
        "Document level allowance VAT rate",
        DataType::Percentage,
        Some(20),
    ),
    term(
        97,
        "Document level allowance reason",
        DataType::Text,
        Some(20),
    ),
    term(
        98,
        "Document level allowance reason code",
        DataType::Code,
        Some(20),
    ),
    term(
        99,
        "Document level charge amount",
        DataType::Amount,
        Some(21),
    ),
    term(
        100,
        "Document level charge base amount",
        DataType::Amount,
        Some(21),
    ),
    term(
        101,
        "Document level charge percentage",
        DataType::Percentage,
        Some(21),
    ),
    term(
        102,
        "Document level charge VAT category code",
        DataType::Code,
        Some(21),
    ),
    term(
        103,
        "Document level charge VAT rate",
        DataType::Percentage,
        Some(21),
    ),
    term(
        104,
        "Document level charge reason",
        DataType::Text,
        Some(21),
    ),
    term(
        105,
        "Document level charge reason code",
        DataType::Code,
        Some(21),
    ),
    term(
        106,
        "Sum of Invoice line net amount",
        DataType::Amount,
        Some(22),
    ),
    term(
        107,
        "Sum of allowances on document level",
        DataType::Amount,
        Some(22),
    ),
    term(
        108,
        "Sum of charges on document level",
        DataType::Amount,
        Some(22),
    ),
    term(
        109,
        "Invoice total amount without VAT",
        DataType::Amount,
        Some(22),
    ),
    term(110, "Invoice total VAT amount", DataType::Amount, Some(22)),
    term(
        111,
        "Invoice total VAT amount in accounting currency",
        DataType::Amount,
        Some(22),
    ),
    term(
        112,
        "Invoice total amount with VAT",
        DataType::Amount,
        Some(22),
    ),
    term(113, "Paid amount", DataType::Amount, Some(22)),
    term(114, "Rounding amount", DataType::Amount, Some(22)),
    term(115, "Amount due for payment", DataType::Amount, Some(22)),
    term(
        116,
        "VAT category taxable amount",
        DataType::Amount,
        Some(23),
    ),
    term(117, "VAT category tax amount", DataType::Amount, Some(23)),
    term(118, "VAT category code", DataType::Code, Some(23)),
    term(119, "VAT category rate", DataType::Percentage, Some(23)),
    term(120, "VAT exemption reason text", DataType::Text, Some(23)),
    term(121, "VAT exemption reason code", DataType::Code, Some(23)),
    term(
        122,
        "Supporting document reference",
        DataType::DocumentReference,
        Some(24),
    ),
    term(
        123,
        "Supporting document description",
        DataType::Text,
        Some(24),
    ),
    term(124, "External document location", DataType::Text, Some(24)),
    term(125, "Attached document", DataType::BinaryObject, Some(24)),
    term(
        126,
        "Invoice line identifier",
        DataType::Identifier,
        Some(25),
    ),
    term(127, "Invoice line note", DataType::Text, Some(25)),
    term(
        128,
        "Invoice line object identifier",
        DataType::Identifier,
        Some(25),
    ),
    term(129, "Invoiced quantity", DataType::Quantity, Some(25)),
    term(
        130,
        "Invoiced quantity unit of measure code",
        DataType::Code,
        Some(25),
    ),
    term(131, "Invoice line net amount", DataType::Amount, Some(25)),
    term(
        132,
        "Referenced purchase order line reference",
        DataType::DocumentReference,
        Some(25),
    ),
    term(
        133,
        "Invoice line Buyer accounting reference",
        DataType::Text,
        Some(25),
    ),
    term(
        134,
        "Invoice line period start date",
        DataType::Date,
        Some(26),
    ),
    term(
        135,
        "Invoice line period end date",
        DataType::Date,
        Some(26),
    ),
    term(
        136,
        "Invoice line allowance amount",
        DataType::Amount,
        Some(27),
    ),
    term(
        137,
        "Invoice line allowance base amount",
        DataType::Amount,
        Some(27),
    ),
    term(
        138,
        "Invoice line allowance percentage",
        DataType::Percentage,
        Some(27),
    ),
    term(
        139,
        "Invoice line allowance reason",
        DataType::Text,
        Some(27),
    ),
    term(
        140,
        "Invoice line allowance reason code",
        DataType::Code,
        Some(27),
    ),
    term(
        141,
        "Invoice line charge amount",
        DataType::Amount,
        Some(28),
    ),
    term(
        142,
        "Invoice line charge base amount",
        DataType::Amount,
        Some(28),
    ),
    term(
        143,
        "Invoice line charge percentage",
        DataType::Percentage,
        Some(28),
    ),
    term(144, "Invoice line charge reason", DataType::Text, Some(28)),
    term(
        145,
        "Invoice line charge reason code",
        DataType::Code,
        Some(28),
    ),
    term(146, "Item net price", DataType::UnitPriceAmount, Some(29)),
    term(
        147,
        "Item price discount",
        DataType::UnitPriceAmount,
        Some(29),
    ),
    term(148, "Item gross price", DataType::UnitPriceAmount, Some(29)),
    term(
        149,
        "Item price base quantity",
        DataType::Quantity,
        Some(29),
    ),
    term(
        150,
        "Item price base quantity unit of measure code",
        DataType::Code,
        Some(29),
    ),
    term(
        151,
        "Invoiced item VAT category code",
        DataType::Code,
        Some(30),
    ),
    term(
        152,
        "Invoiced item VAT rate",
        DataType::Percentage,
        Some(30),
    ),
    term(153, "Item name", DataType::Text, Some(31)),
    term(154, "Item description", DataType::Text, Some(31)),
    term(
        155,
        "Item Sellers identifier",
        DataType::Identifier,
        Some(31),
    ),
    term(
        156,
        "Item Buyers identifier",
        DataType::Identifier,
        Some(31),
    ),
    term(
        157,
        "Item standard identifier",
        DataType::Identifier,
        Some(31),
    ),
    term(
        158,
        "Item classification identifier",
        DataType::Identifier,
        Some(31),
    ),
    term(159, "Item country of origin", DataType::Code, Some(31)),
    term(160, "Item attribute name", DataType::Text, Some(32)),
    term(161, "Item attribute value", DataType::Text, Some(32)),
    term(162, "Seller address line 3", DataType::Text, Some(5)),
    term(163, "Buyer address line 3", DataType::Text, Some(8)),
    term(
        164,
        "Tax representative address line 3",
        DataType::Text,
        Some(12),
    ),
    term(165, "Deliver to address line 3", DataType::Text, Some(15)),
];

/// The business term with `number`, if the standard assigns it.
#[must_use]
pub fn term_by_number(number: u16) -> Option<&'static Term> {
    TERMS.iter().find(|term| term.number == number)
}

/// The business term an identifier such as `BT-27` names, if there is one.
///
/// | Identifier | Term   |
/// |------------|--------|
/// | `BT-27`    | BT-27  |
/// | `BT-4`     | none (the number is not assigned) |
/// | `BG-25`    | none (a group, not a term) |
/// | `BT-027`, `BT-+27`, `bt-27`, `BT-`, `27`, `BT-27 ` | none |
#[must_use]
pub fn term_by_id(id: &str) -> Option<&'static Term> {
    term_by_number(number_after(id, TERM_PREFIX)?)
}

/// The identifier of the business group `number`, as in `BG-25`.
#[must_use]
pub fn group_id(number: u16) -> String {
    format!("{GROUP_PREFIX}{number}")
}

/// The number of the business group an identifier such as `BG-25` names.
///
/// | Identifier | Number |
/// |------------|--------|
/// | `BG-25`    | 25     |
/// | `BG-32`    | 32     |
/// | `BG-0`, `BG-33` | none (no such group) |
/// | `BT-25`    | none (a term, not a group) |
/// | `BG-025`, `BG-+25`, `bg-25`, `BG-`, `BG-25 ` | none |
#[must_use]
pub fn group_number(id: &str) -> Option<u16> {
    number_after(id, GROUP_PREFIX).filter(|number| (1..=LAST_GROUP).contains(number))
}

/// The number written after `prefix` in `id`, in plain digits without a
/// leading zero.
fn number_after(id: &str, prefix: &str) -> Option<u16> {
    let digits = id.strip_prefix(prefix)?;
    let canonical = !digits.is_empty()
        && !digits.starts_with('0')
        && digits.bytes().all(|byte| byte.is_ascii_digit());
    if !canonical {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{TERMS, group_id, group_number, term_by_id, term_by_number};

    /// The highest number of a business term.
    const LAST_TERM: u16 = 165;
    /// BT-4 is not assigned.
    const UNASSIGNED: u16 = 4;

    #[test]
    fn every_assigned_number_has_one_term_in_order() {
        let numbers: Vec<u16> = TERMS.iter().map(|term| term.number).collect();
        let expected: Vec<u16> = (1..=LAST_TERM)
            .filter(|number| *number != UNASSIGNED)
            .collect();
        assert_eq!(numbers, expected);
    }

    #[test]
    fn a_term_is_found_by_its_number() {
        assert_eq!(
            term_by_number(27).map(|term| term.name),
            Some("Seller name")
        );
        assert_eq!(
            term_by_number(27).map(super::Term::id),
            Some("BT-27".to_owned())
        );
        assert!(term_by_number(UNASSIGNED).is_none());
    }

    #[test]
    fn a_group_is_named_by_its_identifier_only() {
        assert_eq!(group_id(25), "BG-25");
        assert_eq!(group_number("BG-25"), Some(25));
        assert_eq!(group_number("BG-32"), Some(32));
        for id in [
            "BG-0", "BG-33", "BT-25", "BG-025", "BG-+25", "bg-25", "BG-", "BG-25 ",
        ] {
            assert!(group_number(id).is_none(), "{id}");
        }
    }

    #[test]
    fn a_term_is_found_by_its_identifier_only() {
        assert_eq!(term_by_id("BT-27").map(|term| term.number), Some(27));
        for id in [
            "BT-4", "BG-25", "BT-027", "BT-+27", "bt-27", "BT-", "27", "BT-27 ",
        ] {
            assert!(term_by_id(id).is_none(), "{id}");
        }
    }
}
