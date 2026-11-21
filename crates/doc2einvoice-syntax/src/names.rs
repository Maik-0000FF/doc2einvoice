// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The names and fixed values both syntaxes use, each written once. Each
//! syntax takes them into its own list of names.

// Attributes.
pub(crate) const CURRENCY_ID: &str = "currencyID";
pub(crate) const SCHEME_ID: &str = "schemeID";
pub(crate) const UNIT_CODE: &str = "unitCode";
pub(crate) const LIST_ID: &str = "listID";
pub(crate) const LIST_VERSION_ID: &str = "listVersionID";
pub(crate) const MIME_CODE: &str = "mimeCode";
pub(crate) const FILENAME: &str = "filename";

// Fixed values.
/// Value added tax, as both the tax scheme of UBL and the tax type of CII
/// (UNTDID 5153) name it.
pub(crate) const VAT: &str = "VAT";
/// The document type code (UNTDID 1001) of an invoiced object (BT-18,
/// BT-128).
pub(crate) const INVOICED_OBJECT_TYPE: &str = "130";
pub(crate) const TRUE: &str = "true";
pub(crate) const FALSE: &str = "false";
