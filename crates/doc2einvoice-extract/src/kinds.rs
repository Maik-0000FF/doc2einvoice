// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The kinds of invoices by their codes (UNTDID 1001), as far as an invoice
//! may carry one (BR-CL-01).
//!
//! Some invoices print the code in brackets after their title, as
//! "Handelsrechnung (380)". Such a printed code is read. A plain title, as
//! "Rechnung" before the number, stands for a commercial invoice; that code
//! is worked out. Other titles do not tell the code: "Gutschrift" is printed
//! over credit notes and over self-billed invoices alike.

use crate::labels::{NUMBER_MARKERS, bracketed, normalized};

/// The codes a kind of invoice may have.
const CODES: &[&str] = &[
    "71", "80", "81", "82", "83", "84", "102", "130", "202", "203", "204", "211", "218", "219",
    "261", "262", "295", "296", "308", "325", "326", "331", "380", "381", "382", "383", "384",
    "385", "386", "387", "388", "389", "390", "393", "394", "395", "396", "420", "456", "457",
    "458", "471", "472", "473", "500", "501", "502", "503", "527", "532", "553", "575", "623",
    "633", "751", "780", "817", "870", "875", "876", "877", "935",
];

/// The code of a commercial invoice, the kind a plain title stands for.
const COMMERCIAL_INVOICE: &str = "380";

/// The titles of a commercial invoice in German, English and French, as
/// [`normalized`] gives them.
const INVOICE_TITLES: [&str; 3] = ["rechnung", "invoice", "facture"];

/// The kind the title of an invoice tells where no code is printed: a
/// commercial invoice, if the line starts with a plain word for invoice and
/// goes on with the number of the invoice or ends there. The code is not
/// printed and is marked as worked out where it is put into a draft.
///
/// | Line                                 | Kind  |
/// |--------------------------------------|-------|
/// | `Rechnung` `#` `RE-2018/507`         | `380` |
/// | `Rechnung` `Nr.` `17`                | `380` |
/// | `Invoice` `FA-2017-0010`             | `380` |
/// | `Facture` `FA-2017-0010`             | `380` |
/// | `Rechnung`                           | `380` |
/// | `Rechnung` `des` `Versicherers` `Nr.` `4711` | none (another kind of invoice) |
/// | `Rechnung` `vom` `16.09.2026`        | none (no number after the title) |
/// | `Rechnungskorrektur` `Nr.` `RK1`     | none (a correction) |
/// | `Gutschrift` `Nr.` `1`               | none (a credit note or a self-billed invoice) |
/// | `Avoir` `AV-2017-0005`               | none  |
/// | `Invoice` `date` `2026-01-01`        | none  |
/// | nothing                              | none  |
pub fn told_by_title(words: &[&str]) -> Option<&'static str> {
    let (title, rest) = words.split_first()?;
    let plain = INVOICE_TITLES.contains(&normalized(title).as_str());
    let number_follows = rest.first().is_none_or(|next| {
        NUMBER_MARKERS.contains(&normalized(next).as_str())
            || next.chars().any(|c| c.is_ascii_digit())
    });
    (plain && number_follows).then_some(COMMERCIAL_INVOICE)
}

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
    use super::{CODES, COMMERCIAL_INVOICE, printed, told_by_title};

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
    fn titles_follow_their_table() {
        let table: [(&[&str], bool); 12] = [
            (&["Rechnung", "#", "RE-2018/507"], true),
            (&["Rechnung", "Nr.", "17"], true),
            (&["Invoice", "FA-2017-0010"], true),
            (&["Facture", "FA-2017-0010"], true),
            (&["Rechnung"], true),
            (&["Rechnung", "des", "Versicherers", "Nr.", "4711"], false),
            (&["Rechnung", "vom", "16.09.2026"], false),
            (&["Rechnungskorrektur", "Nr.", "RK1"], false),
            (&["Gutschrift", "Nr.", "1"], false),
            (&["Avoir", "AV-2017-0005"], false),
            (&["Invoice", "date", "2026-01-01"], false),
            (&[], false),
        ];
        for (words, commercial) in table {
            let expected = commercial.then_some(COMMERCIAL_INVOICE);
            assert_eq!(told_by_title(words), expected, "{words:?}");
        }
        assert!(CODES.contains(&COMMERCIAL_INVOICE));
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
