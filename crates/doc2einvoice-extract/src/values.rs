// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading one value from the words printed after a label, in the form the
//! draft writes it. Each reader takes the words in reading order and tells
//! how many of them the value used, so that its place can be given.
//!
//! A reader refuses what it cannot read without guessing: a date whose day
//! and month could be swapped, or a number whose separator could group
//! thousands or mark decimals.

use std::str::FromStr;

use chrono::NaiveDate;
use doc2einvoice_model::DATE_FORMAT;
use rust_decimal::Decimal;

/// A value read from words: its text in the form of the draft, and how many
/// of the words, from the first on, it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    pub text: String,
    /// The words skipped before the value.
    pub skipped: usize,
    /// The words the value was read from, after the skipped ones.
    pub used: usize,
}

/// Punctuation that may follow a value without belonging to it.
const TRAILING: &[char] = &[',', ';', ':'];

/// How a date is printed: the character between its parts, whether the
/// year comes first, and the chrono format for a year of two and of four
/// digits.
struct DateForm {
    separator: char,
    year_first: bool,
    short_year: Option<&'static str>,
    long_year: &'static str,
    /// Whether day and month could also be read the other way round.
    may_swap: bool,
}

/// The forms dates are read in: day, month and year with dots; the ISO
/// form; day, month and year with slashes, which is also how the month
/// first is printed.
const DATE_FORMS: [DateForm; 3] = [
    DateForm {
        separator: '.',
        year_first: false,
        short_year: Some("%d.%m.%y"),
        long_year: "%d.%m.%Y",
        may_swap: false,
    },
    DateForm {
        separator: '-',
        year_first: true,
        short_year: None,
        long_year: "%Y-%m-%d",
        may_swap: false,
    },
    DateForm {
        separator: '/',
        year_first: false,
        short_year: None,
        long_year: "%d/%m/%Y",
        may_swap: true,
    },
];

/// How many digits a year is printed with.
const SHORT_YEAR: usize = 2;
const LONG_YEAR: usize = 4;

/// How many digits a day or month is printed with at most.
const DAY_DIGITS: usize = 2;

/// The highest number of a month; a day above it cannot be a month.
const MONTHS: u32 = 12;

/// The signs a number may carry for minus: the hyphen and the minus sign.
const MINUS_SIGNS: [char; 2] = ['-', '\u{2212}'];

/// The separators of a printed number.
const SEPARATORS: [char; 2] = ['.', ','];

/// How many digits a group of thousands has.
const GROUP_DIGITS: usize = 3;

/// Currency marks that name exactly one currency.
const CURRENCY_SYMBOLS: [(&str, &str); 2] = [("€", "EUR"), ("£", "GBP")];

/// How many letters a currency code has.
const CODE_LETTERS: usize = 3;

/// Words that say a number follows, compared in lower case without a point
/// or colon at the end, as "Nr." in "Bau (877) Nr. 210113".
const NUMBER_MARKERS: [&str; 5] = ["nr", "no", "#", "n°", "nummer"];

/// How many words after a label a number marker may stand, not counting
/// words in brackets.
const MARKER_REACH: usize = 3;

/// Characters a number marker may end in.
const MARKER_ENDINGS: [char; 2] = ['.', ':'];

/// The identifier the words stand for: the first word, or the word after a
/// number marker among the first few, if it holds a digit and is no date.
///
/// | Words                              | Read                          |
/// |------------------------------------|-------------------------------|
/// | `RE-2026-17`                       | `RE-2026-17`                  |
/// | `471102,` `vom`                    | `471102`                      |
/// | `RE-20190610/507` `issued`         | `RE-20190610/507`             |
/// | `Bau` `(877)` `Nr.` `210113`       | `210113`, three words skipped |
/// | `des` `Versicherers` `Nr.` `4711`  | `4711`, three words skipped   |
/// | `des` `Versicherers` `(575)` `Nr.` `0012`  | `0012` (the code in brackets not counted) |
/// | `(Selbst` `ausgestellte` `Rechnung)` `(389)` `Nr.` `4711` | `4711` |
/// | `a` `b` `c` `Nr.` `4711`           | none (the marker too far)     |
/// | `Nr.` `Kunde:`                     | none (no digit after it)      |
/// | `des` `Versicherers`               | none (no digit)               |
/// | `gemäß` `Bestellung` `vom`         | none                          |
/// | `16.09.2026`                       | none (a date)                 |
/// | `(380)`                            | none (a code in brackets)     |
/// | `Nr.` `16.09.2026`                 | none (a date after the marker) |
/// | nothing                            | none                          |
pub fn identifier(words: &[&str]) -> Option<Read> {
    if let Some(text) = identifier_word(words.first()?) {
        return Some(Read {
            text,
            skipped: 0,
            used: 1,
        });
    }
    let skipped = after_marker(words)?;
    let text = identifier_word(words.get(skipped)?)?;
    Some(Read {
        text,
        skipped,
        used: 1,
    })
}

/// The word after a number marker that stands within the first few words,
/// not counting words in brackets.
fn after_marker(words: &[&str]) -> Option<usize> {
    let mut depth = 0_usize;
    let mut counted = 0;
    for (index, word) in words.iter().enumerate() {
        let opens = word.starts_with('(');
        let inside = depth > 0 || opens;
        if opens {
            depth += 1;
        }
        if word.ends_with(')') {
            depth = depth.saturating_sub(1);
        }
        if inside {
            continue;
        }
        let normal = word.trim_end_matches(MARKER_ENDINGS).to_lowercase();
        if NUMBER_MARKERS.contains(&normal.as_str()) {
            return Some(index + 1);
        }
        counted += 1;
        if counted == MARKER_REACH {
            return None;
        }
    }
    None
}

/// The word as an identifier, if it holds a digit and is neither a date nor
/// a code in brackets.
fn identifier_word(word: &str) -> Option<String> {
    let word = word.trim_end_matches(TRAILING);
    let bracketed = word.starts_with('(') && word.ends_with(')');
    let usable = !bracketed && word.chars().any(|c| c.is_ascii_digit()) && date(&[word]).is_none();
    usable.then(|| word.to_owned())
}

/// The date the first word stands for.
///
/// | Words           | Read         |
/// |-----------------|--------------|
/// | `16.09.2026`    | `2026-09-16` |
/// | `6.9.2026`      | `2026-09-06` |
/// | `16.09.26`      | `2026-09-16` |
/// | `2026-09-16`    | `2026-09-16` |
/// | `16/11/2017`    | `2017-11-16` |
/// | `05/08/2026`    | none (day and month could be swapped) |
/// | `05/05/2026`    | `2026-05-05` (both readings agree) |
/// | `16.09.2026,`   | `2026-09-16` |
/// | `31.02.2026`    | none (no such day) |
/// | `16.09.026`, `16.09.20266`, `016.09.2026`, `26-09-16` | none |
/// | `16.09.2026.`, `16-09-2026`, `16.09` | none |
/// | `2.7`, `Datum`  | none         |
pub fn date(words: &[&str]) -> Option<Read> {
    let word = words.first()?.trim_end_matches(TRAILING);
    let parsed = DATE_FORMS.iter().find_map(|form| dated(word, form))?;
    Some(Read {
        text: parsed.format(DATE_FORMAT).to_string(),
        skipped: 0,
        used: 1,
    })
}

/// The date `word` stands for in `form`, if it has the shape of the form.
fn dated(word: &str, form: &DateForm) -> Option<NaiveDate> {
    let parts: Vec<&str> = word.split(form.separator).collect();
    let [first, second, third] = parts.as_slice() else {
        return None;
    };
    let (year, day, month) = if form.year_first {
        (*first, *third, *second)
    } else {
        (*third, *first, *second)
    };
    let digits = |part: &str, most: usize| {
        !part.is_empty() && part.len() <= most && part.chars().all(|c| c.is_ascii_digit())
    };
    let year_digits = year.chars().all(|c| c.is_ascii_digit());
    let format = match year.len() {
        SHORT_YEAR if year_digits => form.short_year?,
        LONG_YEAR if year_digits => form.long_year,
        _ => return None,
    };
    if !digits(day, DAY_DIGITS) || !digits(month, DAY_DIGITS) {
        return None;
    }
    let parsed = NaiveDate::parse_from_str(word, format).ok()?;
    let (day, month): (u32, u32) = (day.parse().ok()?, month.parse().ok()?);
    let unambiguous = !form.may_swap || day > MONTHS || day == month;
    unambiguous.then_some(parsed)
}

/// The amount the first word after any currency marks stands for.
///
/// | Words                     | Read                       |
/// |---------------------------|----------------------------|
/// | `571,04` `€`              | `571.04`                   |
/// | `1.234,50`                | `1234.5`                   |
/// | `1,234.50`                | `1234.5`                   |
/// | `12.345.678,90`           | `12345678.9`               |
/// | `1.234.567`               | `1234567`                  |
/// | `1234,567`                | `1234.567` (no group has four digits) |
/// | `EUR` `-1,12`             | `-1.12`, one word skipped  |
/// | `€` `571.04`              | `571.04`, one word skipped |
/// | `571,04€`, `€571.04`      | `571.04`                   |
/// | `−7,67` (minus sign)      | `-7.67`                    |
/// | `218,48-`                 | `-218.48`                  |
/// | `0,025`                   | `0.025`                    |
/// | `100`                     | `100`                      |
/// | `1.234`, `1,234`          | none (thousands or decimals) |
/// | `1.23.4`, `1,2,3`, `1.234.5` | none (groups out of place) |
/// | `1,234,5.0`, `1.0,5`      | none                       |
/// | `12,5%`                   | none (a rate)              |
/// | `1` `234,50` (two words)  | `1` (a space may part columns as well as groups) |
/// | `in` `EUR`                | none                       |
pub fn amount(words: &[&str]) -> Option<Read> {
    let skipped = words
        .iter()
        .take_while(|word| currency_mark(word).is_some())
        .count();
    let word = words.get(skipped)?.trim_end_matches(TRAILING);
    let value = number(strip_currency(word))?;
    Some(Read {
        text: value.normalize().to_string(),
        skipped,
        used: 1,
    })
}

/// The word without a currency symbol at its start or end.
fn strip_currency(word: &str) -> &str {
    CURRENCY_SYMBOLS.iter().fold(word, |text, (symbol, _)| {
        text.trim_start_matches(symbol).trim_end_matches(symbol)
    })
}

/// The number `text` stands for, a sign in front or behind.
fn number(text: &str) -> Option<Decimal> {
    let (negative, digits) = match (
        text.strip_prefix(MINUS_SIGNS),
        text.strip_suffix(MINUS_SIGNS),
    ) {
        (Some(rest), _) | (None, Some(rest)) => (true, rest),
        (None, None) => (false, text),
    };
    let value = Decimal::from_str(&plain_digits(digits)?).ok()?;
    Some(if negative { -value } else { value })
}

/// The digits with a point for decimals and no separators of thousands, by
/// the separators `digits` holds:
///
/// - none: the digits;
/// - both: the last one marks decimals, the other groups thousands;
/// - one, more than once: it groups thousands;
/// - one, once: it marks decimals, unless three digits follow it and the
///   digits before it could be a first group of thousands, which leaves it
///   open.
fn plain_digits(digits: &str) -> Option<String> {
    if !digits
        .chars()
        .all(|c| c.is_ascii_digit() || SEPARATORS.contains(&c))
    {
        return None;
    }
    let Some(last) = digits.rfind(SEPARATORS) else {
        return (!digits.is_empty()).then(|| digits.to_owned());
    };
    let mark = digits[last..].chars().next()?;
    let (whole, fraction) = (&digits[..last], &digits[last + mark.len_utf8()..]);
    let other = SEPARATORS
        .into_iter()
        .find(|separator| *separator != mark)?;
    if whole.contains(mark) && !whole.contains(other) {
        return ungrouped(digits, mark);
    }
    let open = !whole.contains(other)
        && fraction.len() == GROUP_DIGITS
        && whole.len() <= GROUP_DIGITS
        && whole != "0";
    if open || fraction.is_empty() || whole.contains(mark) {
        return None;
    }
    Some(format!("{}.{fraction}", ungrouped(whole, other)?))
}

/// `text` without its separators of thousands, if each group after the
/// first has three digits and the first one to three.
fn ungrouped(text: &str, separator: char) -> Option<String> {
    let mut groups = text.split(separator);
    let first = groups.next()?;
    let grouped = text.contains(separator);
    let first_fits = !first.is_empty() && (!grouped || first.len() <= GROUP_DIGITS);
    let rest_fit = groups.all(|group| group.len() == GROUP_DIGITS);
    (first_fits && rest_fit && text.chars().all(|c| c.is_ascii_digit() || c == separator))
        .then(|| text.replace(separator, ""))
}

/// The currency a word marks: a symbol that names one currency, or a code of
/// three capital letters.
///
/// | Word   | Currency |
/// |--------|----------|
/// | `€`    | `EUR`    |
/// | `£`    | `GBP`    |
/// | `EUR`  | `EUR`    |
/// | `CHF`  | `CHF`    |
/// | `$`    | none (several currencies use it) |
/// | `Eur`, `EURO`, `GLN` | `GLN` is three capitals: a code; the others none |
pub fn currency_mark(word: &str) -> Option<String> {
    if let Some((_, code)) = CURRENCY_SYMBOLS.iter().find(|(symbol, _)| *symbol == word) {
        return Some((*code).to_owned());
    }
    let is_code = word.len() == CODE_LETTERS && word.chars().all(|c| c.is_ascii_uppercase());
    is_code.then(|| word.to_owned())
}

/// Whether a word is an amount printed with decimals, as currency marks
/// stand beside.
///
/// | Word      | Decimals |
/// |-----------|----------|
/// | `571,04`  | yes      |
/// | `€571.04` | yes      |
/// | `372876`  | no       |
/// | `1.234`   | no (not an amount) |
pub fn has_decimals(word: &str) -> bool {
    word.contains(SEPARATORS) && amount(&[word]).is_some()
}

/// The currency of a symbol joined to an amount with decimals, as in
/// `€571.04` or `571,04€`.
///
/// | Word       | Currency |
/// |------------|----------|
/// | `€571.04`  | `EUR`    |
/// | `571,04€`  | `EUR`    |
/// | `€`        | none (no amount) |
/// | `€100`     | none (no decimals) |
pub fn joined_symbol(word: &str) -> Option<String> {
    let (_, code) = CURRENCY_SYMBOLS.iter().find(|(symbol, _)| {
        word != *symbol && (word.starts_with(symbol) || word.ends_with(symbol))
    })?;
    has_decimals(word).then(|| (*code).to_owned())
}

/// The currency the first word names.
///
/// | Words  | Read  |
/// |--------|-------|
/// | `EUR`  | `EUR` |
/// | `€`    | `EUR` |
/// | `Euro` | none  |
pub fn currency(words: &[&str]) -> Option<Read> {
    let code = currency_mark(words.first()?.trim_end_matches(TRAILING))?;
    Some(Read {
        text: code,
        skipped: 0,
        used: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::{amount, currency, currency_mark, date, has_decimals, identifier, joined_symbol};

    fn text(read: Option<super::Read>) -> Option<String> {
        read.map(|read| read.text)
    }

    #[test]
    fn identifiers_follow_their_table() {
        let rows: [Row; 15] = [
            (&["RE-2026-17"], Some(("RE-2026-17", 0))),
            (&["471102,", "vom"], Some(("471102", 0))),
            (&["RE-20190610/507", "issued"], Some(("RE-20190610/507", 0))),
            (&["Bau", "(877)", "Nr.", "210113"], Some(("210113", 3))),
            (&["des", "Versicherers", "Nr.", "4711"], Some(("4711", 3))),
            (
                &["des", "Versicherers", "(575)", "Nr.", "0012"],
                Some(("0012", 4)),
            ),
            (
                &[
                    "(Selbst",
                    "ausgestellte",
                    "Rechnung)",
                    "(389)",
                    "Nr.",
                    "4711",
                ],
                Some(("4711", 5)),
            ),
            (&["a", "b", "c", "Nr.", "4711"], None),
            (&["Nr.", "Kunde:"], None),
            (&["des", "Versicherers"], None),
            (&["gemäß", "Bestellung", "vom"], None),
            (&["16.09.2026"], None),
            (&["(380)"], None),
            (&[], None),
            (&["Nr.", "16.09.2026"], None),
        ];
        for (words, expected) in rows {
            let read = identifier(words).map(|read| (read.text, read.skipped));
            let expected = expected.map(|(text, skipped)| (text.to_owned(), skipped));
            assert_eq!(read, expected, "{words:?}");
        }
    }

    #[test]
    fn dates_follow_their_table() {
        let rows = [
            ("16.09.026", None),
            ("16.09.20266", None),
            ("016.09.2026", None),
            ("26-09-16", None),
            ("16.09.2026.", None),
            ("16-09-2026", None),
            ("16.09", None),
            ("16.09.2026", Some("2026-09-16")),
            ("6.9.2026", Some("2026-09-06")),
            ("16.09.26", Some("2026-09-16")),
            ("2026-09-16", Some("2026-09-16")),
            ("16/11/2017", Some("2017-11-16")),
            ("05/08/2026", None),
            ("05/05/2026", Some("2026-05-05")),
            ("16.09.2026,", Some("2026-09-16")),
            ("31.02.2026", None),
            ("2.7", None),
            ("Datum", None),
        ];
        for (word, expected) in rows {
            assert_eq!(text(date(&[word])).as_deref(), expected, "{word}");
        }
    }

    /// Words, and the amount read with the words skipped before it.
    type Row = (&'static [&'static str], Option<(&'static str, usize)>);

    #[test]
    fn amounts_follow_their_table() {
        let rows: [Row; 24] = [
            (&["571,04", "€"], Some(("571.04", 0))),
            (&["1.234,50"], Some(("1234.5", 0))),
            (&["1,234.50"], Some(("1234.5", 0))),
            (&["12.345.678,90"], Some(("12345678.9", 0))),
            (&["1.234.567"], Some(("1234567", 0))),
            (&["1234,567"], Some(("1234.567", 0))),
            (&["EUR", "-1,12"], Some(("-1.12", 1))),
            (&["€", "571.04"], Some(("571.04", 1))),
            (&["571,04€"], Some(("571.04", 0))),
            (&["€571.04"], Some(("571.04", 0))),
            (&["\u{2212}7,67"], Some(("-7.67", 0))),
            (&["218,48-"], Some(("-218.48", 0))),
            (&["0,025"], Some(("0.025", 0))),
            (&["100"], Some(("100", 0))),
            (&["1.234"], None),
            (&["1,234"], None),
            (&["1.23.4"], None),
            (&["1,2,3"], None),
            (&["1.234.5"], None),
            (&["1,234,5.0"], None),
            (&["1.0,5"], None),
            (&["12,5%"], None),
            (&["1", "234,50"], Some(("1", 0))),
            (&["in", "EUR"], None),
        ];
        for (words, expected) in rows {
            let read = amount(words).map(|read| (read.text, read.skipped));
            let expected = expected.map(|(text, skipped)| (text.to_owned(), skipped));
            assert_eq!(read, expected, "{words:?}");
        }
    }

    #[test]
    fn currencies_follow_their_table() {
        assert_eq!(currency_mark("€").as_deref(), Some("EUR"));
        assert_eq!(currency_mark("£").as_deref(), Some("GBP"));
        assert_eq!(currency_mark("EUR").as_deref(), Some("EUR"));
        assert_eq!(currency_mark("CHF").as_deref(), Some("CHF"));
        assert_eq!(currency_mark("GLN").as_deref(), Some("GLN"));
        for word in ["$", "Eur", "EURO"] {
            assert_eq!(currency_mark(word), None, "{word}");
        }
        assert_eq!(text(currency(&["EUR"])).as_deref(), Some("EUR"));
        assert_eq!(text(currency(&["€"])).as_deref(), Some("EUR"));
        assert_eq!(currency(&["Euro"]), None);
    }

    #[test]
    fn marked_amounts_follow_their_table() {
        assert!(has_decimals("571,04"));
        assert!(has_decimals("€571.04"));
        assert!(!has_decimals("372876"));
        assert!(!has_decimals("1.234"));
        assert_eq!(joined_symbol("€571.04").as_deref(), Some("EUR"));
        assert_eq!(joined_symbol("571,04€").as_deref(), Some("EUR"));
        assert_eq!(joined_symbol("€"), None);
        assert_eq!(joined_symbol("€100"), None);
    }
}
