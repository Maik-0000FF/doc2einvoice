// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding the business terms of an invoice in the text of its pages.
//!
//! The result is a [`Draft`]: each value found, in the form its data type is
//! written in, with the place on the page it was found at. Nothing is
//! guessed: a value that is not found is not in the draft, and the user
//! confirms or adds what the draft holds.
//!
//! How well the extraction finds the values is measured against invoices
//! that carry their data as XML (`examples/measure`).

mod address;
mod breakdown;
mod kinds;
mod labels;
mod lines;
mod page;
mod postcodes;
mod rules;
mod table;
mod units;
mod values;

use doc2einvoice_model::Draft;
use doc2einvoice_pdf::PageLayout;

/// Finds the business terms of an invoice on its pages.
#[must_use]
pub fn extract(pages: &[PageLayout]) -> Draft {
    rules::draft(pages)
}
