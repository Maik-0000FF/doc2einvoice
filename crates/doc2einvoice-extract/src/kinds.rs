// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The kinds of invoices by their codes (UNTDID 1001), as far as an invoice
//! may carry one (BR-CL-01).
//!
//! Some invoices print the code in brackets after their title, as
//! "Handelsrechnung (380)". Only such a printed code is read. A title alone
//! does not tell the code: "Gutschrift" is printed over credit notes and
//! over self-billed invoices alike.

use crate::labels::bracketed;

/// The codes a kind of invoice may have.
const CODES: &[&str] = &[
    "71", "80", "81", "82", "83", "84", "102", "130", "202", "203", "204", "211", "218", "219",
    "261", "262", "295", "296", "308", "325", "326", "331", "380", "381", "382", "383", "384",
    "385", "386", "387", "388", "389", "390", "393", "394", "395", "396", "420", "456", "457",
    "458", "471", "472", "473", "500", "501", "502", "503", "527", "532", "553", "575", "623",
    "633", "751", "780", "817", "870", "875", "876", "877", "935",
];

/// The code of a kind of invoice a word prints in brackets.
///
/// | Word            | Code  |
/// |-----------------|-------|
/// | `(380)`         | `380` |
/// | `(384)`, `(71)` | `384`, `71` |
/// | `(555)`, `(12)` | none (no code of a kind of invoice) |
/// | `380`, `(380`   | none (not in brackets) |
/// | `(S)`, `()`     | none  |
pub fn printed(word: &str) -> Option<&'static str> {
    let held = bracketed(word)?;
    CODES.iter().copied().find(|code| *code == held)
}

#[cfg(test)]
mod tests {
    use super::{CODES, printed};

    /// The rules the invoices are validated with, which hold the list of
    /// the codes a kind of invoice may have (BR-CL-01).
    const RULES: &str =
        include_str!("../../../rulesets/en16931/EN16931-CII-validation-preprocessed.sch");

    /// The rule that lists the codes, and what its list stands between.
    const KINDS_RULE: &str = "id=\"BR-CL-01\"";
    const LIST_START: &str = "contains(' ";
    const LIST_END: &str = " '";

    #[test]
    fn printed_codes_follow_their_table() {
        let table = [
            ("(380)", Some("380")),
            ("(384)", Some("384")),
            ("(71)", Some("71")),
            ("(555)", None),
            ("(12)", None),
            ("380", None),
            ("(380", None),
            ("(S)", None),
            ("()", None),
        ];
        for (word, expected) in table {
            assert_eq!(printed(word), expected, "{word}");
        }
    }

    #[test]
    fn the_codes_are_those_the_rules_list() {
        let rule = RULES
            .lines()
            .find(|line| line.contains(KINDS_RULE))
            .unwrap_or_default();
        let listed: Vec<&str> = rule
            .split_once(LIST_START)
            .and_then(|(_, rest)| rest.split_once(LIST_END))
            .map(|(list, _)| list.split(' ').collect())
            .unwrap_or_default();
        assert!(!listed.is_empty(), "the list of the rule");
        assert_eq!(CODES, listed.as_slice());
    }
}
