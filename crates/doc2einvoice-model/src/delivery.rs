// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Where and when the invoiced goods and services are delivered (BG-13), and
//! the period a document or a line covers.

use crate::party::PostalAddress;
use crate::values::{Date, OptionalSchemeIdentifier, Text};

/// A period of days. The invoice (BG-14) and each line (BG-26) have one of
/// this same form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    /// The first day (BT-73, BT-134).
    pub start: Option<Date>,
    /// The last day (BT-74, BT-135).
    pub end: Option<Date>,
}

/// Delivery information (BG-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryInformation {
    /// The party delivered to, where it is not the buyer (BT-70).
    pub party_name: Option<Text>,
    /// An identifier of the place of delivery (BT-71).
    pub location: Option<OptionalSchemeIdentifier>,
    /// The day the delivery actually took place (BT-72).
    pub actual_date: Option<Date>,
    /// The invoicing period (BG-14).
    pub invoicing_period: Option<Period>,
    /// The address delivered to, where it is not the address of the buyer
    /// (BG-15).
    pub address: Option<PostalAddress>,
}
