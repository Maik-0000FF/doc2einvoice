// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The list of the libraries a package contains, as the generator writes it
//! and the interface reads it.
//!
//! The application itself only carries the file from the one to the other, so
//! its form is defined here once; the type the interface reads it as is
//! derived from these.

use serde::Serialize;
use ts_rs::TS;

/// Every library a package of the application contains, with the texts they
/// ask to be passed on: their licences, and the notices and statements of
/// copyright their authors ship beside them.
///
/// Many libraries share a text word for word, so each text is held once and
/// the libraries refer to it by its place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryNotices {
    /// The texts, each once and word for word.
    pub texts: Vec<String>,
    /// The libraries, by name.
    pub libraries: Vec<Library>,
}

/// A library the package contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Library {
    /// The name its author publishes it under.
    pub name: String,
    /// The version contained.
    pub version: String,
    /// The licence its author states, as an SPDX expression where it is one.
    pub license: String,
    /// The places of its texts in [`LibraryNotices::texts`].
    pub texts: Vec<usize>,
    /// What was changed from the published library, if anything.
    pub changes: Option<String>,
}
