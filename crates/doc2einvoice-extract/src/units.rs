// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The units of measure invoices print after a quantity, and the codes the
//! standard names them with (UN/ECE Recommendation 20).
//!
//! A page prints a word, as "Stk" or "kg"; an invoice carries a code, as
//! `H87` or `KGM`. The code is looked up, never printed, and is therefore
//! marked as worked out where it is put into a draft. A word that is not in
//! the table has no code: none is guessed.

use crate::labels::{LABEL_ENDINGS, normalized};

/// The words a unit is printed with, in German, English and French, by the
/// code of the unit, written as [`normalized`] gives them: in whichever
/// case they are printed.
///
/// A piece is `H87`, as the German administration recommends for "Stück";
/// what is counted as units without being pieces is `C62`, "one".
const WORDS: &[(&str, &[&str])] = &[
    (
        "H87",
        &[
            "stk", "stck", "stück", "st", "pc", "pcs", "piece", "pieces", "pièce", "pièces",
        ],
    ),
    (
        "C62",
        &[
            "unit",
            "units",
            "unit(s)",
            "einheit",
            "einheiten",
            "unité",
            "unités",
            "unité(s)",
        ],
    ),
    ("KGM", &["kg"]),
    (
        "LTR",
        &["ltr", "liter", "liter(s)", "litre", "litres", "litre(s)"],
    ),
    ("MLT", &["ml"]),
    ("CMT", &["cm"]),
    ("MMT", &["mm"]),
    ("KMT", &["km"]),
    ("MTK", &["m²", "m2", "qm"]),
    ("MTQ", &["m³", "m3", "cbm"]),
    (
        "HUR",
        &[
            "std", "stunde", "stunden", "hour", "hours", "heure", "heures",
        ],
    ),
    ("MIN", &["min"]),
    (
        "DAY",
        &["tag", "tage", "tag(e)", "day", "days", "jour", "jours"],
    ),
    (
        "WEE",
        &["woche", "wochen", "week", "weeks", "semaine", "semaines"],
    ),
    ("MON", &["monat", "monate", "month", "months", "mois"]),
    ("ANN", &["jahr", "jahre", "year", "years"]),
    ("KWH", &["kwh"]),
];

/// The symbols of one letter a unit is printed with, by the code of the
/// unit. They count only as they are written: `m` is a metre, `M` may be a
/// size. The litre is written both ways.
const SYMBOLS: &[(&str, &[&str])] = &[
    ("MTR", &["m"]),
    ("GRM", &["g"]),
    ("TNE", &["t"]),
    ("HUR", &["h"]),
    ("LTR", &["l", "L"]),
];

/// The code of the unit a printed word stands for.
///
/// | Word                         | Code  |
/// |------------------------------|-------|
/// | `Stk`, `Stk.`, `ST`, `Stück` | `H87` |
/// | `Unit(s)`, `Einheiten`       | `C62` |
/// | `kg`, `KG`                   | `KGM` |
/// | `m²`, `qm`                   | `MTK` |
/// | `Liter(s)`, `l`, `L`         | `LTR` |
/// | `Std`, `Std.`, `h`           | `HUR` |
/// | `m`, `m.`                    | `MTR` |
/// | `Tag(e)`, `Tage`             | `DAY` |
/// | `Monat`                      | `MON` |
/// | `kWh`, `KWH`                 | `KWH` |
/// | `M`, `G`, `T`, `H`           | none (a size or a mark, not the symbol of a unit) |
/// | `an`, `ans`                  | none (too common a word for the year it may mean) |
/// | `Paletten`, `Pauschal`       | none (not in the table) |
/// | `19%`, `100,00`, nothing     | none  |
pub fn code(word: &str) -> Option<&'static str> {
    let as_written = word.trim_end_matches(LABEL_ENDINGS);
    let any_case = normalized(word);
    let among = |table: &'static [(&'static str, &'static [&'static str])], word: &str| {
        table
            .iter()
            .find(|(_, words)| words.contains(&word))
            .map(|(code, _)| *code)
    };
    among(SYMBOLS, as_written).or_else(|| among(WORDS, &any_case))
}

#[cfg(test)]
mod tests {
    use doc2einvoice_model::UNIT_CODES;

    use super::{SYMBOLS, WORDS, code};

    #[test]
    fn units_follow_their_table() {
        let table = [
            ("Stk", Some("H87")),
            ("Stk.", Some("H87")),
            ("ST", Some("H87")),
            ("Stück", Some("H87")),
            ("Unit(s)", Some("C62")),
            ("Einheiten", Some("C62")),
            ("kg", Some("KGM")),
            ("KG", Some("KGM")),
            ("m²", Some("MTK")),
            ("qm", Some("MTK")),
            ("Liter(s)", Some("LTR")),
            ("l", Some("LTR")),
            ("L", Some("LTR")),
            ("Std", Some("HUR")),
            ("Std.", Some("HUR")),
            ("h", Some("HUR")),
            ("m", Some("MTR")),
            ("m.", Some("MTR")),
            ("Tag(e)", Some("DAY")),
            ("Tage", Some("DAY")),
            ("Monat", Some("MON")),
            ("kWh", Some("KWH")),
            ("KWH", Some("KWH")),
            ("M", None),
            ("G", None),
            ("T", None),
            ("H", None),
            ("an", None),
            ("ans", None),
            ("Paletten", None),
            ("Pauschal", None),
            ("19%", None),
            ("100,00", None),
            ("", None),
        ];
        for (word, expected) in table {
            assert_eq!(code(word), expected, "{word}");
        }
    }

    #[test]
    fn every_code_is_one_the_standard_allows() {
        for (code, _) in WORDS.iter().chain(SYMBOLS) {
            assert!(UNIT_CODES.contains(code), "{code}");
        }
    }

    #[test]
    fn no_word_stands_for_two_units() {
        let words: Vec<&str> = WORDS
            .iter()
            .chain(SYMBOLS)
            .flat_map(|(_, words)| words.iter().copied())
            .collect();
        for word in &words {
            let count = words.iter().filter(|other| *other == word).count();
            assert_eq!(count, 1, "{word}");
        }
    }
}
