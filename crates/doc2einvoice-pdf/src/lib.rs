// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading the visible text of a PDF.
//!
//! A PDF does not contain words or lines. It contains characters, each drawn
//! at its own place, in whatever order the program that wrote the file chose.
//! This crate reads those characters and orders them by where they stand.
//!
//! The layers:
//!
//! - [`read`]: the characters of every page, each a [`Glyph`] with its place,
//!   as a [`Page`].
//! - [`Limits`]: how large a PDF may be to be read at all.
//! - [`PageLayout`]: the characters of a page ordered into [`Word`]s,
//!   [`Line`]s and [`Block`]s.
//!
//! The file is only read. Nothing here changes a PDF or writes one.
//!
//! Places are given the way a page is displayed: the origin is the top left
//! corner of the visible page, `x` runs to the right and `y` downwards, both
//! in points (1/72 inch). A page that the file marks as rotated is read the
//! way it is shown.
//!
//! Which PDF library does the reading is a detail of [`read`]. Everything else
//! works on [`Glyph`]s, so another reader can take its place without touching
//! the layout.
//!
//! ```
//! use doc2einvoice_pdf::{PdfError, read};
//!
//! // Text that is no PDF is refused.
//! assert!(matches!(read(b"not a PDF"), Err(PdfError::Invalid)));
//! ```

mod error;
mod glyph;
mod layout;
mod reader;

pub use error::PdfError;
pub use glyph::{Glyph, Page};
pub use layout::{Block, Line, PageLayout, Word};
pub use reader::{Limits, read, read_within};
