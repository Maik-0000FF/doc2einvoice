// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Writing an invoice of the EN 16931 model in the two syntaxes the standard
//! names: UBL 2.1 and UN/CEFACT Cross Industry Invoice D16B.
//!
//! Each business term is written at the element of the syntax that holds it,
//! in the order the XML Schema of the syntax prescribes. A fixed code word
//! the syntax requires and no business term holds is written as the
//! published examples of the standard write it; an invoice the syntax cannot
//! express without inventing a value about it is refused with a [`Gap`].
//! Nothing is written empty.

pub mod bindings;
mod cii;
mod error;
mod format;
mod names;
pub mod namespaces;
mod shared;
mod ubl;
mod xml;

pub use cii::write as write_cii;
pub use error::{Gap, Syntax, WriteError};
pub use ubl::{UblDocument, write as write_ubl};
