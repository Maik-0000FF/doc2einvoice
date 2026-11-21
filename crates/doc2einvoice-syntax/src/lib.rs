// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing an invoice of the EN 16931 model in the two syntaxes the standard
//! names: UBL 2.1 and UN/CEFACT Cross Industry Invoice D16B.
//!
//! Each business term is written at the element of the syntax that holds it,
//! in the order the XML Schema of the syntax prescribes. An invoice the
//! syntax cannot express without inventing a value is refused with a
//! [`Gap`].

mod cii;
mod error;
mod format;
mod ubl;
mod xml;

pub use cii::write as write_cii;
pub use error::{Gap, Syntax, WriteError};
pub use ubl::{UblDocument, write as write_ubl};
