// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The invoice a PDF carries as an embedded Cross Industry Invoice, as
//! ZUGFeRD and Factur-X embed it.

use lopdf::{Document, Object};

/// The local name of the root element of a Cross Industry Invoice.
const CII_ROOT: &str = "CrossIndustryInvoice";

/// The keys of a file specification that lead to the embedded file: the
/// embedded files, and in them the file under its name, or under its
/// Unicode name if only that is given.
const EMBEDDED_FILES: &[u8] = b"EF";
const FILE_KEYS: [&[u8]; 2] = [b"F", b"UF"];

/// A byte order mark some producers put in front of the XML.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// The first embedded file of `pdf` that is a Cross Industry Invoice.
///
/// | PDF                                         | Result          |
/// |---------------------------------------------|-----------------|
/// | embeds a CII under `/F`                     | `Ok(Some(xml))` |
/// | embeds a CII under `/UF` only               | `Ok(Some(xml))` |
/// | embeds other files only, or none            | `Ok(None)`      |
/// | embeds a CII that is not UTF-8              | `Ok(None)`      |
/// | cannot be loaded (broken, encrypted, no PDF) | `Err`          |
pub fn invoice(pdf: &[u8]) -> Result<Option<String>, lopdf::Error> {
    let document = Document::load_mem(pdf)?;
    Ok(document
        .objects
        .values()
        .find_map(|object| cii_of(&document, object)))
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
    let file = FILE_KEYS.iter().find_map(|key| files.get(key).ok())?;
    let (_, Object::Stream(stream)) = document.dereference(file).ok()? else {
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

#[cfg(test)]
mod tests {
    use lopdf::{Document, Object, Stream, dictionary};

    use super::{CII_ROOT, invoice};

    /// The smallest Cross Industry Invoice the check accepts.
    const CII: &str = "<rsm:CrossIndustryInvoice xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\"/>";

    /// A PDF that embeds `content` under the key `key` of the embedded files.
    fn pdf(key: &str, content: &str) -> Vec<u8> {
        let mut document = Document::with_version("1.7");
        let file = document.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
        let specification = document.add_object(dictionary! {
            "Type" => "Filespec",
            "EF" => dictionary! { key => file },
        });
        let pages = document.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => Vec::<Object>::new(),
            "Count" => 0,
        });
        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages,
            "AF" => vec![Object::Reference(specification)],
        });
        document.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        document
            .save_to(&mut bytes)
            .unwrap_or_else(|error| unreachable!("{error}"));
        bytes
    }

    fn found(pdf: &[u8]) -> Option<String> {
        invoice(pdf).unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn an_invoice_is_found_under_either_name() {
        for key in ["F", "UF"] {
            let xml = found(&pdf(key, CII));
            assert!(xml.is_some_and(|xml| xml.contains(CII_ROOT)), "{key}");
        }
    }

    #[test]
    fn other_files_are_no_invoice() {
        assert_eq!(found(&pdf("F", "<other/>")), None);
        assert_eq!(found(&pdf("F", "not xml")), None);
    }

    #[test]
    fn what_is_no_pdf_cannot_be_loaded() {
        assert!(invoice(b"no pdf").is_err());
    }
}
