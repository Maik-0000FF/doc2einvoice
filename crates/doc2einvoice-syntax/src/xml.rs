// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The one way this crate writes XML: elements with attributes and text,
//! nested by closures, escaped by the XML library.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesText, Event};

/// The version and encoding every document is declared with.
const XML_VERSION: &str = "1.0";
const XML_ENCODING: &str = "UTF-8";

/// An attribute: its name and its value.
pub(crate) type Attribute<'a> = (&'a str, &'a str);

/// A document being written.
pub(crate) struct Xml {
    writer: Writer<Vec<u8>>,
}

impl Xml {
    /// A document that starts with its XML declaration.
    pub(crate) fn new() -> std::io::Result<Self> {
        let mut writer = Writer::new(Vec::new());
        writer.write_event(Event::Decl(BytesDecl::new(
            XML_VERSION,
            Some(XML_ENCODING),
            None,
        )))?;
        Ok(Self { writer })
    }

    /// An element whose content `body` writes.
    pub(crate) fn element(
        &mut self,
        name: &str,
        attributes: &[Attribute<'_>],
        body: impl FnOnce(&mut Self) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let start =
            quick_xml::events::BytesStart::new(name).with_attributes(attributes.iter().copied());
        let end = start.to_end().into_owned();
        self.writer.write_event(Event::Start(start))?;
        body(self)?;
        self.writer.write_event(Event::End(end))
    }

    /// An element that holds only `text`.
    pub(crate) fn text(
        &mut self,
        name: &str,
        attributes: &[Attribute<'_>],
        text: &str,
    ) -> std::io::Result<()> {
        self.element(name, attributes, |xml| {
            xml.writer.write_event(Event::Text(BytesText::new(text)))
        })
    }

    /// An element that holds `text`, if there is any.
    pub(crate) fn optional_text(&mut self, name: &str, text: Option<&str>) -> std::io::Result<()> {
        match text {
            Some(text) => self.text(name, &[], text),
            None => Ok(()),
        }
    }

    /// The document as UTF-8.
    pub(crate) fn finish(self) -> Vec<u8> {
        self.writer.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::Xml;

    fn written(build: impl FnOnce(&mut Xml) -> std::io::Result<()>) -> String {
        let mut xml = Xml::new().unwrap_or_else(|error| unreachable!("{error}"));
        build(&mut xml).unwrap_or_else(|error| unreachable!("{error}"));
        String::from_utf8(xml.finish()).unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn text_and_attributes_are_escaped() {
        let document = written(|xml| xml.text("a", &[("b", "x\"<&")], "1 < 2 & 3"));
        assert!(
            document.ends_with(r#"<a b="x&quot;&lt;&amp;">1 &lt; 2 &amp; 3</a>"#),
            "{document}"
        );
    }

    #[test]
    fn elements_nest_and_close_in_order() {
        let document = written(|xml| {
            xml.element("a", &[], |xml| {
                xml.optional_text("b", Some("1"))?;
                xml.optional_text("c", None)
            })
        });
        assert!(
            document.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#),
            "{document}"
        );
        assert!(document.ends_with("<a><b>1</b></a>"), "{document}");
    }
}
