// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading a document into a tree, after making sure the tree is not too deep.
//!
//! Everything that works on a document walks its tree, and much of that code
//! calls itself once per level. A document nested tens of thousands of levels
//! deep exhausts the stack of the thread, which ends the whole program; no
//! error can be returned from there. Such a document is therefore refused
//! before anything builds a tree from it.

use xmlparser::{ElementEnd, Token, Tokenizer};

use crate::error::ValidationError;

/// The deepest nesting of elements a document may have.
///
/// An invoice in UBL or CII is nested about a dozen levels deep. The limit
/// leaves room for any real document and stays far below the depth at which
/// the stack of a thread of the usual size is exhausted, which lies in the
/// thousands.
pub const MAX_NESTING_DEPTH: usize = 256;

/// Reads a document into a tree.
///
/// # Errors
///
/// Returns [`ValidationError::TooDeeplyNested`] if the elements of the
/// document are nested deeper than [`MAX_NESTING_DEPTH`], and
/// [`ValidationError::Malformed`] if it is not well-formed XML.
pub(crate) fn parse(document: &str) -> Result<roxmltree::Document<'_>, ValidationError> {
    refuse_deep_nesting(document)?;
    roxmltree::Document::parse(document)
        .map_err(|error| ValidationError::Malformed(error.to_string()))
}

/// Refuses a document whose elements are nested deeper than the limit.
///
/// | Document                                          | Result               |
/// |---------------------------------------------------|----------------------|
/// | elements nested up to the limit                   | accepted             |
/// | elements nested one level deeper                  | refused              |
/// | many elements next to each other, none deep       | accepted             |
/// | an empty element (`<a/>`) at the limit            | accepted: one level  |
/// | text that looks like tags in a comment, in CDATA  | accepted: it is text |
/// | or in an attribute value                          |                      |
/// | no text at all                                    | accepted             |
/// | not well-formed, and shallow up to the mistake    | accepted: the parser |
/// |                                                   | reports the mistake  |
/// | not well-formed, and too deep before the mistake  | refused              |
/// | more end tags than start tags                     | accepted: the parser |
/// |                                                   | reports the mistake  |
///
/// The text is only cut into its parts here, one after the other; no tree is
/// built and nothing calls itself, so the depth of the document cannot harm
/// this check. Whether the document is well-formed is not decided here.
fn refuse_deep_nesting(document: &str) -> Result<(), ValidationError> {
    let mut depth: usize = 0;

    for token in Tokenizer::from(document) {
        match token {
            Ok(Token::ElementStart { .. }) => {
                depth += 1;
                if depth > MAX_NESTING_DEPTH {
                    return Err(ValidationError::TooDeeplyNested {
                        limit: MAX_NESTING_DEPTH,
                    });
                }
            }
            Ok(Token::ElementEnd {
                end: ElementEnd::Close(..) | ElementEnd::Empty,
                ..
            }) => depth = depth.saturating_sub(1),
            Ok(_) => {}
            // What follows a mistake is the business of the parser.
            Err(_) => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MAX_NESTING_DEPTH, parse, refuse_deep_nesting};
    use crate::error::ValidationError;

    fn nested(levels: usize) -> String {
        format!("{}{}", "<a>".repeat(levels), "</a>".repeat(levels))
    }

    fn is_too_deep(result: &Result<(), ValidationError>) -> bool {
        matches!(
            result,
            Err(ValidationError::TooDeeplyNested {
                limit: MAX_NESTING_DEPTH
            })
        )
    }

    #[test]
    fn the_limit_itself_is_accepted_and_one_level_more_is_not() {
        assert!(refuse_deep_nesting(&nested(MAX_NESTING_DEPTH)).is_ok());
        assert!(is_too_deep(&refuse_deep_nesting(&nested(
            MAX_NESTING_DEPTH + 1
        ))));
    }

    #[test]
    fn elements_next_to_each_other_are_not_deep() {
        let wide = format!("<a>{}</a>", "<b/><c></c>".repeat(MAX_NESTING_DEPTH * 4));

        assert!(refuse_deep_nesting(&wide).is_ok());
    }

    #[test]
    fn an_empty_element_counts_as_one_level() {
        let at_the_limit = format!(
            "{}<b/>{}",
            "<a>".repeat(MAX_NESTING_DEPTH - 1),
            "</a>".repeat(MAX_NESTING_DEPTH - 1)
        );
        let beyond = format!(
            "{}<b/>{}",
            "<a>".repeat(MAX_NESTING_DEPTH),
            "</a>".repeat(MAX_NESTING_DEPTH)
        );

        assert!(refuse_deep_nesting(&at_the_limit).is_ok());
        assert!(is_too_deep(&refuse_deep_nesting(&beyond)));
    }

    #[test]
    fn what_only_looks_like_tags_is_text() {
        let tags = "<a>".repeat(MAX_NESTING_DEPTH * 2);

        for document in [
            format!("<r><!-- {tags} --></r>"),
            format!("<r><![CDATA[{tags}]]></r>"),
            format!(
                "<r note=\"{}\"/>",
                "&lt;a&gt;".repeat(MAX_NESTING_DEPTH * 2)
            ),
            format!("<r note='{}'/>", ">".repeat(MAX_NESTING_DEPTH * 2)),
        ] {
            assert!(refuse_deep_nesting(&document).is_ok(), "{document:.40}");
        }
    }

    #[test]
    fn what_is_not_well_formed_is_left_to_the_parser_unless_it_is_too_deep_first() {
        for document in ["", "<a><b></a>", "</a></a></a>", "<a", "not XML"] {
            assert!(refuse_deep_nesting(document).is_ok(), "{document}");
        }
        let deep_then_broken = format!("{}<", "<a>".repeat(MAX_NESTING_DEPTH + 1));

        assert!(is_too_deep(&refuse_deep_nesting(&deep_then_broken)));
    }

    #[test]
    fn a_document_far_deeper_than_any_stack_allows_is_refused_and_does_not_crash() {
        /// Deep enough to exhaust the stack of a thread if a tree were built.
        const LEVELS: usize = 200_000;

        assert!(matches!(
            parse(&nested(LEVELS)),
            Err(ValidationError::TooDeeplyNested { .. })
        ));
    }

    #[test]
    fn a_document_at_the_limit_is_read_into_a_tree() -> Result<(), ValidationError> {
        let document = nested(MAX_NESTING_DEPTH);

        assert_eq!(
            parse(&document)?.descendants().count(),
            MAX_NESTING_DEPTH + 1
        );
        Ok(())
    }
}
