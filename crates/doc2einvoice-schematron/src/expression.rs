// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Preparation of the expressions found in a rule file for the XPath engine.
//!
//! Every rewrite here keeps the meaning the standards define. They exist
//! because the engine either lacks a construct (user-defined functions, match
//! patterns) or rejects a form it should accept.
//!
//! Nothing here looks inside a string literal: what a rule file writes between
//! quotes is data, never code.

use std::collections::BTreeSet;

/// Prefix of the variables that hold user-defined functions.
const FUNCTION_VARIABLE_PREFIX: &str = "fn_";

/// Functions the engine rejects when they are called without an argument on a
/// node, paired with the explicit form the XPath specification defines them as.
const IMPLICIT_CONTEXT_CALLS: [(&str, &str); 2] = [
    ("normalize-space()", "normalize-space(string(.))"),
    ("string-length()", "string-length(string(.))"),
];

const PATH_ROOT: char = '/';
const DESCENDANT_PREFIX: &str = "//";
const UNION_OPERATOR: char = '|';
const GROUP_OPEN: char = '(';
const GROUP_CLOSE: char = ')';
const PREDICATE_OPEN: char = '[';
const PREDICATE_CLOSE: char = ']';
const VARIABLE_SIGIL: char = '$';
const QUOTES: [char; 2] = ['\'', '"'];
const COMMENT_OPEN: &str = "(:";
const COMMENT_CLOSE: &str = ":)";

/// Name of the variable that holds the user-defined function `call_name`
/// (for example `u:mod11`).
pub(crate) fn function_variable(call_name: &str) -> String {
    format!("{FUNCTION_VARIABLE_PREFIX}{}", call_name.replace(':', "_"))
}

/// Rewrites an expression so that the engine evaluates it as intended: calls of
/// user-defined functions become calls of the variables holding them, and the
/// argument-less calls the engine rejects get their explicit argument.
pub(crate) fn normalize(expression: &str, function_names: &[String]) -> String {
    map_code(expression, |code| {
        let mut result = code.to_owned();
        for (implicit, explicit) in IMPLICIT_CONTEXT_CALLS {
            result = result.replace(implicit, explicit);
        }
        for name in function_names {
            result = result.replace(
                &format!("{name}("),
                &format!("{VARIABLE_SIGIL}{}(", function_variable(name)),
            );
        }
        result
    })
}

/// The names of all variables an expression refers to with `$name`.
///
/// A name is read to its end, so `$total` is not found in `$totalAmount`.
pub(crate) fn variable_references(expression: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    map_code(expression, |code| {
        let mut rest = code;
        while let Some(sigil) = rest.find(VARIABLE_SIGIL) {
            let after = &rest[sigil + VARIABLE_SIGIL.len_utf8()..];
            let length = after
                .find(|character: char| !is_name_character(character))
                .unwrap_or(after.len());
            if length > 0 {
                names.insert(after[..length].to_owned());
            }
            rest = &after[length..];
        }
        code.to_owned()
    });
    names
}

/// The characters of a variable name as the rule files write them.
fn is_name_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.')
}

/// Applies `rewrite` to every part of an expression that is code and leaves
/// string literals and comments untouched.
///
/// | part of the expression            | treated as                                 |
/// |-----------------------------------|--------------------------------------------|
/// | `'text'`, `"text"`                | literal, copied as written                 |
/// | `'it''s'` (doubled quote)        | two literals in a row, copied              |
/// | `(: note :)`                      | comment, copied as written                 |
/// | `(: a (: nested :) b :)`          | one comment                                |
/// | `(: don't :)`                     | comment: the quote does not open a literal |
/// | `'(: not a comment :)'`          | literal                                    |
/// | a literal or comment never closed | copied as written up to the end            |
/// | everything else                   | code, handed to `rewrite`                  |
fn map_code(expression: &str, mut rewrite: impl FnMut(&str) -> String) -> String {
    let mut result = String::with_capacity(expression.len());
    let mut code_start = 0;
    let mut position = 0;

    while position < expression.len() {
        let rest = &expression[position..];
        match untouched_length(rest) {
            Some(length) => {
                result.push_str(&rewrite(&expression[code_start..position]));
                result.push_str(&rest[..length]);
                position += length;
                code_start = position;
            }
            None => position += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    result.push_str(&rewrite(&expression[code_start..]));
    result
}

/// The length of the string literal or comment `text` starts with, if it starts
/// with one. This is the one place that decides what is code and what is not;
/// everything in this module that reads an expression goes through it.
fn untouched_length(text: &str) -> Option<usize> {
    if text.starts_with(COMMENT_OPEN) {
        Some(comment_length(text))
    } else if text.starts_with(QUOTES) {
        Some(literal_length(text))
    } else {
        None
    }
}

/// The characters of an expression that are code, each with its position.
/// String literals and comments are left out.
fn code_characters(expression: &str) -> Vec<(usize, char)> {
    let mut characters = Vec::new();
    let mut position = 0;
    while position < expression.len() {
        let rest = &expression[position..];
        if let Some(length) = untouched_length(rest) {
            position += length;
            continue;
        }
        let Some(character) = rest.chars().next() else {
            break;
        };
        characters.push((position, character));
        position += character.len_utf8();
    }
    characters
}

/// The length of the string literal `text` starts with, including its quotes.
/// A literal that is never closed reaches to the end.
fn literal_length(text: &str) -> usize {
    let mut characters = text.char_indices();
    let Some((_, quote)) = characters.next() else {
        return 0;
    };
    characters
        .find(|(_, character)| *character == quote)
        .map_or(text.len(), |(position, _)| position + quote.len_utf8())
}

/// The length of the comment `text` starts with, including the comments nested
/// in it. A comment that is never closed reaches to the end.
fn comment_length(text: &str) -> usize {
    let mut depth = 0_usize;
    let mut position = 0;
    while position < text.len() {
        let rest = &text[position..];
        if rest.starts_with(COMMENT_OPEN) {
            depth += 1;
            position += COMMENT_OPEN.len();
        } else if rest.starts_with(COMMENT_CLOSE) {
            depth = depth.saturating_sub(1);
            position += COMMENT_CLOSE.len();
            if depth == 0 {
                return position;
            }
        } else {
            position += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    text.len()
}

/// Turns the match pattern of a rule context into an expression that selects
/// every node the pattern matches, evaluated from the document node.
///
/// A pattern matches a node wherever the node sits, so each alternative that
/// is not already anchored at the root is searched for in the whole document.
pub(crate) fn context_selection(pattern: &str) -> String {
    top_level_alternatives(pattern)
        .iter()
        .map(|alternative| alternative.trim())
        .map(|alternative| {
            if is_anchored(alternative) {
                alternative.to_owned()
            } else {
                format!("{DESCENDANT_PREFIX}{alternative}")
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Whether an alternative selects from the root by itself: an absolute path,
/// or a parenthesised union of such paths (possibly followed by predicates).
/// Searching the whole document for an anchored alternative would select the
/// same nodes once per node of the document.
fn is_anchored(alternative: &str) -> bool {
    if alternative.starts_with(PATH_ROOT) {
        return true;
    }
    leading_group(alternative).is_some_and(|group| {
        top_level_alternatives(group)
            .iter()
            .all(|inner| is_anchored(inner.trim()))
    })
}

/// The content of the parentheses an expression starts with, if it starts
/// with a parenthesised group.
fn leading_group(expression: &str) -> Option<&str> {
    if !expression.starts_with(GROUP_OPEN) {
        return None;
    }
    let mut nesting = 0_usize;
    for (position, character) in code_characters(expression) {
        match character {
            GROUP_OPEN => nesting += 1,
            GROUP_CLOSE => {
                nesting = nesting.saturating_sub(1);
                if nesting == 0 {
                    return Some(&expression[GROUP_OPEN.len_utf8()..position]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Splits a pattern at the union operators that are not nested in a predicate
/// or in parentheses. A union operator inside a string literal or a comment is
/// not one.
fn top_level_alternatives(pattern: &str) -> Vec<String> {
    let mut alternatives = Vec::new();
    let mut start = 0;
    let mut nesting = 0_usize;

    for (position, character) in code_characters(pattern) {
        match character {
            PREDICATE_OPEN | GROUP_OPEN => nesting += 1,
            PREDICATE_CLOSE | GROUP_CLOSE => nesting = nesting.saturating_sub(1),
            UNION_OPERATOR if nesting == 0 => {
                alternatives.push(pattern[start..position].to_owned());
                start = position + UNION_OPERATOR.len_utf8();
            }
            _ => {}
        }
    }
    alternatives.push(pattern[start..].to_owned());
    alternatives
}

#[cfg(test)]
mod tests {
    use super::{context_selection, function_variable, normalize, variable_references};

    // Input table for `context_selection`:
    //
    // | pattern                                  | selection                        |
    // |------------------------------------------|----------------------------------|
    // | relative name                            | searched in the whole document   |
    // | absolute path                            | kept                             |
    // | union of relative and absolute           | each alternative on its own      |
    // | `|` inside a predicate                   | not a split point                |
    // | `|` inside a string literal              | not a split point                |
    // | group of absolute paths, with predicate  | kept: already anchored           |
    // | group with a relative path               | searched in the whole document   |
    // | group of absolute paths, nested          | kept                             |
    // | `(` that is never closed                 | searched in the whole document   |
    // | `|` inside a comment                     | not a split point                |
    // | `)` inside a literal within a group      | does not close the group         |
    #[test]
    fn a_relative_pattern_is_searched_in_the_whole_document() {
        assert_eq!(context_selection("cac:InvoiceLine"), "//cac:InvoiceLine");
    }

    #[test]
    fn an_absolute_pattern_is_kept() {
        assert_eq!(context_selection("/ubl:Invoice"), "/ubl:Invoice");
    }

    #[test]
    fn a_union_is_split_into_its_alternatives() {
        assert_eq!(
            context_selection("/ubl:Invoice | cn:CreditNote"),
            "/ubl:Invoice | //cn:CreditNote"
        );
    }

    #[test]
    fn a_union_inside_a_predicate_is_not_split() {
        assert_eq!(context_selection("a[b | c]"), "//a[b | c]");
    }

    #[test]
    fn a_bar_inside_a_string_literal_is_not_split() {
        assert_eq!(context_selection("a[. = 'x|y']"), "//a[. = 'x|y']");
    }

    #[test]
    fn a_group_of_absolute_paths_is_already_anchored() {
        assert_eq!(context_selection("(/a | /b)[$c]"), "(/a | /b)[$c]");
        assert_eq!(context_selection("((/a | /b) | /c)"), "((/a | /b) | /c)");
    }

    #[test]
    fn a_group_with_a_relative_path_is_searched_in_the_whole_document() {
        assert_eq!(context_selection("(/a | b)[c]"), "//(/a | b)[c]");
        assert_eq!(context_selection("(a"), "//(a");
    }

    #[test]
    fn a_bar_inside_a_comment_is_not_split() {
        assert_eq!(context_selection("a (: b | c :)"), "//a (: b | c :)");
    }

    #[test]
    fn a_parenthesis_inside_a_literal_does_not_close_a_group() {
        assert_eq!(
            context_selection("(/a[. = ')'] | /b)[c]"),
            "(/a[. = ')'] | /b)[c]"
        );
    }

    // Input table for `normalize`:
    //
    // | expression                                   | result                         |
    // |----------------------------------------------|--------------------------------|
    // | call of a user-defined function              | call of its variable           |
    // | the function name inside a string literal    | untouched                      |
    // | argument-less `normalize-space()`            | explicit argument              |
    // | the same text inside a string literal        | untouched                      |
    // | the function called with an argument         | untouched                      |
    // | a literal that is never closed               | left as written                |
    // | the function name inside a comment           | untouched                      |
    // | a quote inside a comment                     | does not open a literal        |
    // | a doubled quote inside a literal             | stays inside the literal       |
    #[test]
    fn a_function_call_becomes_a_call_of_its_variable() {
        let functions = ["u:mod11".to_owned()];

        assert_eq!(normalize("u:mod11(.)", &functions), "$fn_u_mod11(.)");
        assert_eq!(function_variable("u:mod11"), "fn_u_mod11");
    }

    #[test]
    fn string_literals_are_never_rewritten() {
        let functions = ["u:mod11".to_owned()];

        assert_eq!(
            normalize(
                "u:mod11('u:mod11(') and . = \"string-length()\"",
                &functions
            ),
            "$fn_u_mod11('u:mod11(') and . = \"string-length()\""
        );
        assert_eq!(
            normalize("'never closed u:mod11(", &functions),
            "'never closed u:mod11("
        );
    }

    #[test]
    fn comments_are_never_rewritten_and_do_not_hide_what_follows() {
        let functions = ["u:mod11".to_owned()];

        assert_eq!(
            normalize("(: don't call u:mod11( here :) u:mod11(.)", &functions),
            "(: don't call u:mod11( here :) $fn_u_mod11(.)"
        );
        assert_eq!(
            normalize("(: a (: nested ' :) b :) string-length() = 2", &[]),
            "(: a (: nested ' :) b :) string-length(string(.)) = 2"
        );
        assert_eq!(
            normalize("'(: no comment :)' = u:mod11(.)", &functions),
            "'(: no comment :)' = $fn_u_mod11(.)"
        );
        assert_eq!(
            normalize("(: never closed u:mod11(", &functions),
            "(: never closed u:mod11("
        );
    }

    #[test]
    fn a_doubled_quote_stays_inside_its_literal() {
        let functions = ["u:mod11".to_owned()];

        assert_eq!(
            normalize("'it''s u:mod11(' = u:mod11(.)", &functions),
            "'it''s u:mod11(' = $fn_u_mod11(.)"
        );
    }

    #[test]
    fn argument_less_calls_get_their_explicit_argument() {
        assert_eq!(
            normalize("string-length(normalize-space()) = 2", &[]),
            "string-length(normalize-space(string(.))) = 2"
        );
        assert_eq!(
            normalize("string-length() = 2", &[]),
            "string-length(string(.)) = 2"
        );
    }

    #[test]
    fn a_function_with_arguments_is_left_alone() {
        assert_eq!(
            normalize("normalize-space(cbc:ID)", &[]),
            "normalize-space(cbc:ID)"
        );
    }

    // Input table for `variable_references`:
    //
    // | expression                          | names                    |
    // |-------------------------------------|--------------------------|
    // | no variable                         | none                     |
    // | one name, used twice                | once                     |
    // | a name that starts another name     | both, each in full       |
    // | names with `-`, `_`, `.`, digits    | read to their end        |
    // | `$name` inside a string literal     | not a reference          |
    // | `$name` inside a comment            | not a reference          |
    // | a lone `$`                          | not a reference          |
    #[test]
    fn names_are_read_to_their_end() {
        let names = variable_references("$total = 1 and $totalAmount > $total");

        assert_eq!(
            names.into_iter().collect::<Vec<_>>(),
            ["total", "totalAmount"]
        );
    }

    #[test]
    fn names_may_contain_the_usual_name_characters() {
        let names = variable_references("matches(., $XR-IBAN-REGEX) or $a_1.b(2)");

        assert_eq!(
            names.into_iter().collect::<Vec<_>>(),
            ["XR-IBAN-REGEX", "a_1.b"]
        );
    }

    #[test]
    fn what_is_not_a_reference_is_not_found() {
        assert!(variable_references("count(a) = 1").is_empty());
        assert!(variable_references("matches(., '^\\$total$') or . = \"$x\"").is_empty());
        assert!(variable_references("a $ b").is_empty());
        assert!(variable_references("(: uses $x, doesn't it :) 1").is_empty());
    }
}
