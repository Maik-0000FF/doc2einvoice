// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The one way this crate writes XML: elements with attributes and text,
//! nested by closures, escaped by the XML library.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};

/// The version and encoding every document is declared with.
const XML_VERSION: &str = "1.0";
const XML_ENCODING: &str = "UTF-8";

/// An attribute: its name and its value.
pub(crate) type Attribute<'a> = (&'a str, &'a str);

/// The attribute `name` with `value`, if there is a value.
pub(crate) fn optional_attribute<'a>(name: &'a str, value: Option<&'a str>) -> Vec<Attribute<'a>> {
    value.map(|value| (name, value)).into_iter().collect()
}

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

    /// An element whose content `body` writes, if that content is not
    /// empty. No element is ever written empty: a value that is blank is a
    /// value that is not there.
    ///
    /// | Content `body` writes        | Written             |
    /// |------------------------------|---------------------|
    /// | `<b>1</b>`                   | `<a><b>1</b></a>`   |
    /// | nothing, or only blank texts | nothing             |
    pub(crate) fn element(
        &mut self,
        name: &str,
        attributes: &[Attribute<'_>],
        body: impl FnOnce(&mut Self) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let content = Self::content(body)?;
        if content.is_empty() {
            return Ok(());
        }
        self.wrap(name, attributes, &content)
    }

    /// An element the XML Schema requires, written even if it is empty.
    pub(crate) fn required_element(
        &mut self,
        name: &str,
        attributes: &[Attribute<'_>],
        body: impl FnOnce(&mut Self) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        let content = Self::content(body)?;
        self.wrap(name, attributes, &content)
    }

    /// What `body` writes, as XML not yet placed in the document.
    fn content(body: impl FnOnce(&mut Self) -> std::io::Result<()>) -> std::io::Result<Vec<u8>> {
        let mut inner = Self {
            writer: Writer::new(Vec::new()),
        };
        body(&mut inner)?;
        Ok(inner.finish())
    }

    /// The element `name` around `content`, which is written XML already.
    fn wrap(
        &mut self,
        name: &str,
        attributes: &[Attribute<'_>],
        content: &[u8],
    ) -> std::io::Result<()> {
        let start = BytesStart::new(name).with_attributes(attributes.iter().copied());
        let end = start.to_end().into_owned();
        self.writer.write_event(Event::Start(start))?;
        self.writer.get_mut().extend_from_slice(content);
        self.writer.write_event(Event::End(end))
    }

    /// An element that holds only `text`, if the text is not blank.
    ///
    /// | Text                       | Written        |
    /// |----------------------------|----------------|
    /// | `a`                        | `<x>a</x>`     |
    /// | ` a `                      | `<x> a </x>`   |
    /// | empty                      | nothing        |
    /// | only spaces or line breaks | nothing        |
    pub(crate) fn text(
        &mut self,
        name: &str,
        attributes: &[Attribute<'_>],
        text: &str,
    ) -> std::io::Result<()> {
        if text.trim().is_empty() {
            return Ok(());
        }
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

    fn written_xml(build: impl FnOnce(&mut Xml) -> std::io::Result<()>) -> String {
        let mut xml = Xml::new().unwrap_or_else(|error| unreachable!("{error}"));
        build(&mut xml).unwrap_or_else(|error| unreachable!("{error}"));
        String::from_utf8(xml.finish()).unwrap_or_else(|error| unreachable!("{error}"))
    }

    #[test]
    fn text_and_attributes_are_escaped() {
        let document = written_xml(|xml| xml.text("a", &[("b", "x\"<&")], "1 < 2 & 3"));
        assert!(
            document.ends_with(r#"<a b="x&quot;&lt;&amp;">1 &lt; 2 &amp; 3</a>"#),
            "{document}"
        );
    }

    #[test]
    fn blank_texts_follow_their_table() {
        for (text, written) in [
            ("a", "<x>a</x>"),
            (" a ", "<x> a </x>"),
            ("", ""),
            ("  \n\t", ""),
        ] {
            let document = written_xml(|xml| xml.text("x", &[], text));
            assert!(
                document.ends_with(&format!("?>{written}")),
                "{text:?}: {document}"
            );
        }
    }

    #[test]
    fn an_element_with_only_blank_content_is_not_written() {
        let document =
            written_xml(|xml| xml.element("a", &[("b", "c")], |xml| xml.text("x", &[], " ")));
        assert!(document.ends_with("?>"), "{document}");
    }

    #[test]
    fn a_required_element_is_written_even_if_empty() {
        let document = written_xml(|xml| xml.required_element("a", &[], |_| Ok(())));
        assert!(document.ends_with("?><a></a>"), "{document}");
    }

    #[test]
    fn an_optional_attribute_is_there_only_with_a_value() {
        assert_eq!(super::optional_attribute("a", Some("b")), [("a", "b")]);
        assert!(super::optional_attribute("a", None).is_empty());
    }

    #[test]
    fn elements_nest_and_close_in_order() {
        let document = written_xml(|xml| {
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
