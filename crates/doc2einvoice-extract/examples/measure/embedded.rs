// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The invoice a PDF carries as an embedded Cross Industry Invoice, as
//! ZUGFeRD and Factur-X embed it.

use lopdf::{Document, Object};

/// The local name of the root element of a Cross Industry Invoice.
const CII_ROOT: &str = "CrossIndustryInvoice";

/// The keys of a file specification that lead to the embedded file.
const EMBEDDED_FILES: &[u8] = b"EF";
const FILE: &[u8] = b"F";

/// A byte order mark some producers put in front of the XML.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// The first embedded file of `pdf` that is a Cross Industry Invoice, if any.
pub fn invoice(pdf: &[u8]) -> Option<String> {
    let document = Document::load_mem(pdf).ok()?;
    document
        .objects
        .values()
        .find_map(|object| cii_of(&document, object))
}

/// The Cross Industry Invoice a file specification embeds, if it is one.
fn cii_of(document: &Document, object: &Object) -> Option<String> {
    let Object::Dictionary(specification) = object else {
        return None;
    };
    let (_, Object::Dictionary(files)) = document
        .dereference(specification.get(EMBEDDED_FILES).ok()?)
        .ok()?
    else {
        return None;
    };
    let (_, Object::Stream(stream)) = document.dereference(files.get(FILE).ok()?).ok()? else {
        return None;
    };
    let bytes = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());
    let text = String::from_utf8(bytes).ok()?;
    let text = text.trim_start_matches(BYTE_ORDER_MARK).to_owned();
    let is_cii = roxmltree::Document::parse(&text)
        .is_ok_and(|xml| xml.root_element().tag_name().name() == CII_ROOT);
    is_cii.then_some(text)
}
