// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Resolution of `include` elements.
//!
//! The interpreter never opens a file itself. The caller hands in the text of
//! every file a rule file may include, so that rule files can be compiled from
//! data embedded in the program.
//!
//! Includes are found in the parsed document, not in its text: an `include`
//! inside a comment is not one, and how the element is written (prefix, line
//! breaks, order of attributes) does not matter.

use roxmltree::Document;

use crate::error::SchemaError;
use crate::schema::SCHEMATRON_NAMESPACE;

const ELEMENT_INCLUDE: &str = "include";
const ATTRIBUTE_HREF: &str = "href";

/// Supplies the text of included files.
pub trait IncludeResolver {
    /// Returns the text of the file an `include` refers to, or `None` if the
    /// reference is unknown.
    fn resolve(&self, href: &str) -> Option<&str>;
}

/// The resolver for rule files that include nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoIncludes;

impl IncludeResolver for NoIncludes {
    fn resolve(&self, _href: &str) -> Option<&str> {
        None
    }
}

/// Resolves includes from a fixed list of pairs of `href` and file text.
#[derive(Debug, Clone, Copy)]
pub struct IncludedFiles<'a>(pub &'a [(&'a str, &'a str)]);

impl IncludeResolver for IncludedFiles<'_> {
    fn resolve(&self, href: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(known, _)| *known == href)
            .map(|(_, text)| *text)
    }
}

/// Replaces every `include` element by the document element of the file it
/// names, so that the rules form a single document. Included files may include
/// further files.
pub(crate) fn expand(source: &str, resolver: &dyn IncludeResolver) -> Result<String, SchemaError> {
    expand_within(source, resolver, &mut Vec::new())
}

/// `including` holds the files that are being expanded around this one, to
/// recognise a file that includes itself, directly or through other files.
fn expand_within(
    source: &str,
    resolver: &dyn IncludeResolver,
    including: &mut Vec<String>,
) -> Result<String, SchemaError> {
    let document = Document::parse(source)?;
    let includes: Vec<_> = document
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == ELEMENT_INCLUDE
                && node.tag_name().namespace() == Some(SCHEMATRON_NAMESPACE)
        })
        .collect();

    let mut expanded = source.to_owned();
    // From the last include to the first, so that replacing one does not move
    // the positions of those still to be replaced.
    for include in includes.iter().rev() {
        let href = include
            .attribute(ATTRIBUTE_HREF)
            .ok_or(SchemaError::MissingAttribute {
                element: ELEMENT_INCLUDE,
                attribute: ATTRIBUTE_HREF,
            })?;
        if including.iter().any(|outer| outer == href) {
            return Err(SchemaError::IncludeCycle {
                href: href.to_owned(),
            });
        }
        let text = resolver
            .resolve(href)
            .ok_or_else(|| SchemaError::UnresolvedInclude {
                href: href.to_owned(),
            })?;

        including.push(href.to_owned());
        let nested = expand_within(text, resolver, including)?;
        including.pop();

        // Only the document element is inserted: an XML declaration or a
        // comment before it has no place in the middle of another document.
        let root = Document::parse(&nested)?.root_element().range();
        expanded.replace_range(include.range(), &nested[root]);
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::{IncludedFiles, NoIncludes, expand};
    use crate::error::SchemaError;

    const SCH: &str = "http://purl.oclc.org/dsdl/schematron";

    fn schema(body: &str) -> String {
        format!("<schema xmlns=\"{SCH}\" xmlns:sch=\"{SCH}\">{body}</schema>")
    }

    // Input table for `expand`:
    //
    // | rule file                                         | result                          |
    // |---------------------------------------------------|---------------------------------|
    // | no include                                        | unchanged                       |
    // | one include, file with XML declaration and comment| only its document element       |
    // | `href` after a line break, other attributes first | expanded                        |
    // | include written with a prefix (`sch:include`)     | expanded                        |
    // | the word include inside a comment                 | untouched                       |
    // | an element `include` in another namespace         | untouched                       |
    // | two includes                                      | both expanded, in place         |
    // | included file includes a further file             | expanded as well                |
    // | unknown file                                      | UnresolvedInclude               |
    // | include without href                              | MissingAttribute                |
    // | file includes itself                              | IncludeCycle                    |
    // | two files include each other                      | IncludeCycle                    |
    // | rule file that is not well-formed                 | Xml                             |
    #[test]
    fn a_rule_file_without_includes_is_unchanged() -> Result<(), SchemaError> {
        let rules = schema("<pattern/>");

        assert_eq!(expand(&rules, &NoIncludes)?, rules);
        Ok(())
    }

    #[test]
    fn an_include_is_replaced_by_the_document_element_of_the_file() -> Result<(), SchemaError> {
        let files = [(
            "common.sch",
            "<?xml version=\"1.0\"?>\n<!-- shared -->\n<pattern xmlns=\"http://purl.oclc.org/dsdl/schematron\"/>",
        )];

        let expanded = expand(
            &schema("<include href=\"common.sch\"/>"),
            &IncludedFiles(&files),
        )?;

        assert_eq!(
            expanded,
            schema("<pattern xmlns=\"http://purl.oclc.org/dsdl/schematron\"/>")
        );
        Ok(())
    }

    #[test]
    fn how_the_include_is_written_does_not_matter() -> Result<(), SchemaError> {
        let files = [("a.sch", "<a/>")];

        for include in [
            "<include\n   href=\"a.sch\"/>",
            "<include id=\"x\" href='a.sch'></include>",
            "<sch:include href=\"a.sch\"/>",
        ] {
            assert_eq!(
                expand(&schema(include), &IncludedFiles(&files))?,
                schema("<a/>"),
                "`{include}` must be expanded"
            );
        }
        Ok(())
    }

    #[test]
    fn what_only_looks_like_an_include_is_untouched() -> Result<(), SchemaError> {
        for body in [
            "<!-- <include href=\"a.sch\"/> -->",
            "<include xmlns=\"urn:other\" href=\"a.sch\"/>",
        ] {
            let rules = schema(body);

            assert_eq!(expand(&rules, &NoIncludes)?, rules);
        }
        Ok(())
    }

    #[test]
    fn several_and_nested_includes_are_expanded() -> Result<(), SchemaError> {
        let files = [
            ("a.sch", "<a/>"),
            (
                "b.sch",
                "<b xmlns:sch=\"http://purl.oclc.org/dsdl/schematron\"><sch:include href=\"a.sch\"/></b>",
            ),
        ];

        let expanded = expand(
            &schema("<include href=\"a.sch\"/><x/><include href=\"b.sch\"/>"),
            &IncludedFiles(&files),
        )?;

        assert_eq!(
            expanded,
            schema("<a/><x/><b xmlns:sch=\"http://purl.oclc.org/dsdl/schematron\"><a/></b>")
        );
        Ok(())
    }

    #[test]
    fn an_unknown_file_is_refused() {
        let result = expand(&schema("<include href=\"missing.sch\"/>"), &NoIncludes);

        assert!(matches!(
            result,
            Err(SchemaError::UnresolvedInclude { href }) if href == "missing.sch"
        ));
    }

    #[test]
    fn an_include_without_href_is_refused() {
        let result = expand(&schema("<include/>"), &NoIncludes);

        assert!(matches!(
            result,
            Err(SchemaError::MissingAttribute {
                element: "include",
                attribute: "href"
            })
        ));
    }

    #[test]
    fn a_file_that_includes_itself_is_refused() {
        let own = schema("<include href=\"self.sch\"/>");
        let first = schema("<include href=\"second.sch\"/>");
        let second = schema("<include href=\"first.sch\"/>");
        let files = [
            ("self.sch", own.as_str()),
            ("first.sch", first.as_str()),
            ("second.sch", second.as_str()),
        ];

        for start in ["self.sch", "first.sch"] {
            let result = expand(
                &schema(&format!("<include href=\"{start}\"/>")),
                &IncludedFiles(&files),
            );

            assert!(
                matches!(result, Err(SchemaError::IncludeCycle { .. })),
                "`{start}` must be refused"
            );
        }
    }

    #[test]
    fn a_rule_file_that_is_not_well_formed_is_refused() {
        assert!(matches!(
            expand("<schema>", &NoIncludes),
            Err(SchemaError::Xml(_))
        ));
    }
}
