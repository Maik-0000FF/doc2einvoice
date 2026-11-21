// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The namespaces of the documents this crate writes, for whoever reads them
//! back.

/// A UBL 2.1 invoice.
pub const UBL_INVOICE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
/// A UBL 2.1 credit note.
pub const UBL_CREDIT_NOTE: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
/// The aggregate components of UBL 2.1.
pub const UBL_AGGREGATE: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
/// The basic components of UBL 2.1.
pub const UBL_BASIC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
/// The Cross Industry Invoice D16B.
pub const CII_INVOICE: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
/// The reusable aggregate business information entities of CII D16B.
pub const CII_AGGREGATE: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
/// The unqualified data types of CII D16B.
pub const CII_UNQUALIFIED: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";
/// The qualified data types of CII D16B.
pub const CII_QUALIFIED: &str = "urn:un:unece:uncefact:data:standard:QualifiedDataType:100";
