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
use doc2einvoice_model::{Amount, CurrencyCode, DATE_FORMAT};
use iban::{Iban, IbanLike};
use rust_decimal::Decimal;

use crate::labels::{CLOSING_BRACKET, NUMBER_MARKERS, OPENING_BRACKET, bracketed, normalized};

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

/// The letters of the country a VAT identifier starts with.
const VAT_COUNTRY_LETTERS: usize = 2;

/// How many words a VAT identifier printed in groups spans at most, as
/// `DE 123 456 789`.
const VAT_WORDS_MOST: usize = 5;

/// How many words an IBAN printed in groups of four spans at most: 34
/// characters.
const IBAN_WORDS_MOST: usize = 9;

/// The sign of a rate, as in `19 %` or `19%`.
const PERCENT: char = '%';

/// How many words after a label a number marker may stand, not counting
/// words in brackets.
const MARKER_REACH: usize = 3;

/// The highest rate of a tax: the whole of what it is charged on.
const RATE_MOST: Decimal = Decimal::ONE_HUNDRED;

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
        let opens = word.starts_with(OPENING_BRACKET);
        let inside = depth > 0 || opens;
        if opens {
            depth += 1;
        }
        if word.ends_with(CLOSING_BRACKET) {
            depth = depth.saturating_sub(1);
        }
        if inside {
            continue;
        }
        if NUMBER_MARKERS.contains(&normalized(word).as_str()) {
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
    let usable = bracketed(word).is_none()
        && word.chars().any(|c| c.is_ascii_digit())
        && date(&[word]).is_none();
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

/// The amount the first word after any currency marks and rates stands
/// for.
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
/// | `19` `%` `11,20`          | `11.2`, two words skipped (a rate before it) |
/// | `19%` `11,20`             | `11.2`, one word skipped   |
/// | `EUR` `19,00%` `11,20`    | `11.2`, two words skipped  |
/// | `571,04€`, `€571.04`      | `571.04`                   |
/// | `−7,67` (minus sign)      | `-7.67`                    |
/// | `218,48-`                 | `-218.48`                  |
/// | `0,025`                   | `0.025`                    |
/// | `100`                     | `100`                      |
/// | `1.234`, `1,234`          | none (thousands or decimals) |
/// | `1.23.4`, `1,2,3`, `1.234.5` | none (groups out of place) |
/// | `1,234,5.0`, `1.0,5`      | none                       |
/// | `12,5%`, `19` `%`         | none (only a rate)         |
/// | `(19` `%)` `11,20`        | `11.2` (a rate in brackets passed over) |
/// | `119%` `11,20`, `-5%` `11,20` | none (no rate to pass over, and no number) |
/// | `1` `234,50` (two words)  | `1` (a space may part columns as well as groups) |
/// | `in` `EUR`                | none                       |
pub fn amount(words: &[&str]) -> Option<Read> {
    let (skipped, value) = number_after_marks(words)?;
    Some(Read {
        text: value.normalize().to_string(),
        skipped,
        used: 1,
    })
}

/// A monetary amount, as a total is: an amount with at most the decimal
/// places the model allows.
///
/// | Words       | Read     |
/// |-------------|----------|
/// | `571,04`    | `571.04` |
/// | `1.234,50`  | `1234.5` |
/// | `1234,567`  | none (three decimal places) |
/// | `0,025`     | none     |
/// | `19` `%` `11,20` | `11.2` |
pub fn monetary(words: &[&str]) -> Option<Read> {
    let (skipped, value) = number_after_marks(words)?;
    let amount = Amount::new(value).ok()?;
    Some(Read {
        text: amount.value().to_string(),
        skipped,
        used: 1,
    })
}

/// The number of the first word after currency marks and rates, and how
/// many words were skipped before it.
fn number_after_marks(words: &[&str]) -> Option<(usize, Decimal)> {
    let mut skipped = 0;
    loop {
        let word = words.get(skipped)?.trim_end_matches(TRAILING);
        if currency_mark(word).is_some() {
            skipped += 1;
        } else if let Some(rate) = rate(&words[skipped..]) {
            skipped += rate.used;
        } else {
            return Some((skipped, number(strip_currency(word))?));
        }
    }
}

/// A rate with its sign, joined to it or as the next word, in brackets or
/// not.
///
/// | Words               | Read                 |
/// |---------------------|----------------------|
/// | `19%`               | `19`                 |
/// | `19` `%`            | `19`, two words      |
/// | `19,00` `%`         | `19`, two words      |
/// | `20,0%`, `5.5%`     | `20`, `5.5`          |
/// | `(19` `%)`          | `19`, two words      |
/// | `(19%)`, `19%,`     | `19`                 |
/// | `0%`, `100%`        | `0`, `100`           |
/// | `19`, `19` `EUR`    | none (no sign)       |
/// | `%`, `(%)`          | none (no number)     |
/// | `%` `19`            | none (the sign first) |
/// | `-5%`, `5-%`        | none (below nothing) |
/// | `119%`              | none (above the whole) |
/// | `1.234%`            | none (thousands or decimals) |
/// | `MwSt%`             | none                 |
/// | nothing             | none                 |
pub fn rate(words: &[&str]) -> Option<Read> {
    let closed = |word: &str| -> String {
        word.trim_end_matches(TRAILING)
            .trim_end_matches(CLOSING_BRACKET)
            .to_owned()
    };
    let head = closed(words.first()?.trim_start_matches(OPENING_BRACKET));
    let sign_follows = words
        .get(1)
        .is_some_and(|next| closed(next) == PERCENT.to_string());
    let (digits, used) = match head.strip_suffix(PERCENT) {
        Some(digits) => (digits, 1),
        None if sign_follows => (head.as_str(), 2),
        None => return None,
    };
    let value = number(digits).filter(|value| may_be_rate(*value))?;
    Some(Read {
        text: value.normalize().to_string(),
        skipped: 0,
        used,
    })
}

/// Whether a number can be the rate of a tax: from nothing to the whole.
pub fn may_be_rate(value: Decimal) -> bool {
    (Decimal::ZERO..=RATE_MOST).contains(&value)
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

/// The currency a word marks: a symbol that names one currency, or a code
/// the standard allows.
///
/// | Word   | Currency |
/// |--------|----------|
/// | `€`    | `EUR`    |
/// | `£`    | `GBP`    |
/// | `EUR`  | `EUR`    |
/// | `CHF`  | `CHF`    |
/// | `$`    | none (several currencies use it) |
/// | `VAT`, `NET`, `GLN` | none (three capitals, but no currency) |
/// | `Eur`, `EURO` | none |
pub fn currency_mark(word: &str) -> Option<String> {
    if let Some((_, code)) = CURRENCY_SYMBOLS.iter().find(|(symbol, _)| *symbol == word) {
        return Some((*code).to_owned());
    }
    CurrencyCode::new(word)
        .ok()
        .map(|code| code.as_str().to_owned())
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

/// The VAT identifier the first words stand for, whether printed in one or
/// in groups: the fewest words from the first on that start with the two
/// letters of a country and have the form of a VAT identifier of that
/// country, as the library for VAT identifiers checks it. Only the form is
/// checked, not the check digit: an identifier that is printed wrong is
/// better questioned in the form than left out.
///
/// | Words                         | Read            |
/// |-------------------------------|-----------------|
/// | `DE123456789`                 | `DE123456789`   |
/// | `DE` `123` `456` `789`        | `DE123456789`, four words |
/// | `ATU12345678`                 | `ATU12345678`   |
/// | `FR11999999998`               | `FR11999999998` |
/// | `DE123456789,`                | `DE123456789`   |
/// | `DE012345678`                 | none (no German identifier starts with 0) |
/// | `DE12345678`                  | none (eight digits) |
/// | `DE`                          | none            |
/// | `123456789`                   | none (no country) |
/// | `DEUTSCHLAND`                 | none            |
/// | `de123456789`                 | none            |
pub fn vat_identifier(words: &[&str]) -> Option<Read> {
    let first = words.first()?;
    let country = first.get(..VAT_COUNTRY_LETTERS)?;
    if !country.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    (1..=VAT_WORDS_MOST.min(words.len())).find_map(|used| {
        let joined: String = words[..used]
            .iter()
            .map(|word| word.trim_end_matches(TRAILING))
            .collect();
        let checked = vat_id_validator::check_vat(&joined);
        let text = checked.value.filter(|_| checked.is_valid_format)?;
        text.starts_with(country).then_some(Read {
            text,
            skipped: 0,
            used,
        })
    })
}

/// The IBAN the first words stand for, whether printed in one or in groups
/// of four, as the library for IBANs checks it: the fewest words from the
/// first on that form a valid IBAN. It is written in its electronic form.
///
/// | Words                                    | Read                      |
/// |------------------------------------------|---------------------------|
/// | `DE02120300000000202051`                 | that, one word            |
/// | `DE02` `1203` `0000` `0000` `2020` `51`  | `DE02120300000000202051`, six words |
/// | the same, then `BIC:`                    | the same                  |
/// | `DE02` `1203` `0000` `0000` `2020` `52`  | none (wrong check digits) |
/// | `4711`                                   | none                      |
pub fn iban(words: &[&str]) -> Option<Read> {
    (1..=IBAN_WORDS_MOST.min(words.len())).find_map(|used| {
        let joined: String = words[..used]
            .iter()
            .map(|word| word.trim_end_matches(TRAILING))
            .collect();
        let parsed = joined.parse::<Iban>().ok()?;
        Some(Read {
            text: parsed.electronic_str().to_owned(),
            skipped: 0,
            used,
        })
    })
}

/// A text as printed: all the words, one space between each, if one of
/// them holds a letter.
///
/// | Words                       | Read                    |
/// |-----------------------------|-------------------------|
/// | `MUSTERLIEFERANT` `GMBH`    | `MUSTERLIEFERANT GMBH`  |
/// | `Kunden` `AG` `Mitte`       | `Kunden AG Mitte`       |
/// | `4711`                      | none (no letter)        |
/// | nothing                     | none                    |
pub fn text(words: &[&str]) -> Option<Read> {
    let has_letter = words
        .iter()
        .any(|word| word.chars().any(char::is_alphabetic));
    has_letter.then(|| Read {
        text: words.join(" "),
        skipped: 0,
        used: words.len(),
    })
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
    use rust_decimal::Decimal;

    use super::{
        amount, currency, currency_mark, date, has_decimals, iban, identifier, joined_symbol,
        may_be_rate, monetary, rate, text as text_of, vat_identifier,
    };

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
        let rows: [Row; 31] = [
            (&["(19", "%)", "11,20"], Some(("11.2", 2))),
            (&["119%", "11,20"], None),
            (&["-5%", "11,20"], None),
            (&["19", "%", "11,20"], Some(("11.2", 2))),
            (&["19%", "11,20"], Some(("11.2", 1))),
            (&["EUR", "19,00%", "11,20"], Some(("11.2", 2))),
            (&["19", "%"], None),
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
    fn totals_follow_their_table() {
        let rows: [Row; 5] = [
            (&["571,04"], Some(("571.04", 0))),
            (&["1.234,50"], Some(("1234.5", 0))),
            (&["1234,567"], None),
            (&["0,025"], None),
            (&["19", "%", "11,20"], Some(("11.2", 2))),
        ];
        for (words, expected) in rows {
            let read = monetary(words).map(|read| (read.text, read.skipped));
            let expected = expected.map(|(text, skipped)| (text.to_owned(), skipped));
            assert_eq!(read, expected, "{words:?}");
        }
    }

    #[test]
    fn vat_identifiers_follow_their_table() {
        let rows: [Row; 11] = [
            (&["DE123456789"], Some(("DE123456789", 1))),
            (&["DE", "123", "456", "789"], Some(("DE123456789", 4))),
            (&["ATU12345678"], Some(("ATU12345678", 1))),
            (&["FR11999999998"], Some(("FR11999999998", 1))),
            (&["DE123456789,"], Some(("DE123456789", 1))),
            (&["DE012345678"], None),
            (&["DE12345678"], None),
            (&["DE"], None),
            (&["123456789"], None),
            (&["DEUTSCHLAND"], None),
            (&["de123456789"], None),
        ];
        for (words, expected) in rows {
            let read = vat_identifier(words).map(|read| (read.text, read.used));
            let expected = expected.map(|(text, used)| (text.to_owned(), used));
            assert_eq!(read, expected, "{words:?}");
        }
    }

    #[test]
    fn ibans_follow_their_table() {
        let grouped = ["DE02", "1203", "0000", "0000", "2020", "51", "BIC:"];
        let read = iban(&grouped).map(|read| (read.text, read.used));
        assert_eq!(read, Some(("DE02120300000000202051".to_owned(), 6)));
        assert_eq!(
            text(iban(&["DE02120300000000202051"])).as_deref(),
            Some("DE02120300000000202051")
        );
        assert_eq!(iban(&["DE02", "1203", "0000", "0000", "2020", "52"]), None);
        assert_eq!(iban(&["4711"]), None);
    }

    #[test]
    fn texts_follow_their_table() {
        assert_eq!(
            text(text_of(&["MUSTERLIEFERANT", "GMBH"])).as_deref(),
            Some("MUSTERLIEFERANT GMBH")
        );
        assert_eq!(
            text(text_of(&["Kunden", "AG", "Mitte"])).as_deref(),
            Some("Kunden AG Mitte")
        );
        assert_eq!(text_of(&["4711"]), None);
        assert_eq!(text_of(&[]), None);
    }

    #[test]
    fn currencies_follow_their_table() {
        assert_eq!(currency_mark("€").as_deref(), Some("EUR"));
        assert_eq!(currency_mark("£").as_deref(), Some("GBP"));
        assert_eq!(currency_mark("EUR").as_deref(), Some("EUR"));
        assert_eq!(currency_mark("CHF").as_deref(), Some("CHF"));
        for word in ["$", "VAT", "NET", "GLN", "Eur", "EURO"] {
            assert_eq!(currency_mark(word), None, "{word}");
        }
        assert_eq!(text(currency(&["EUR"])).as_deref(), Some("EUR"));
        assert_eq!(text(currency(&["€"])).as_deref(), Some("EUR"));
        assert_eq!(currency(&["Euro"]), None);
    }

    #[test]
    fn rates_follow_their_table() {
        let rows: [Row; 21] = [
            (&["19%"], Some(("19", 1))),
            (&["19", "%"], Some(("19", 2))),
            (&["19,00", "%"], Some(("19", 2))),
            (&["20,0%"], Some(("20", 1))),
            (&["5.5%"], Some(("5.5", 1))),
            (&["(19", "%)"], Some(("19", 2))),
            (&["(19%)"], Some(("19", 1))),
            (&["19%,"], Some(("19", 1))),
            (&["0%"], Some(("0", 1))),
            (&["100%"], Some(("100", 1))),
            (&["19"], None),
            (&["19", "EUR"], None),
            (&["%"], None),
            (&["(%)"], None),
            (&["%", "19"], None),
            (&["-5%"], None),
            (&["5-%"], None),
            (&["119%"], None),
            (&["1.234%"], None),
            (&["MwSt%"], None),
            (&[], None),
        ];
        for (words, expected) in rows {
            let read = rate(words);
            let got = read.as_ref().map(|read| (read.text.as_str(), read.used));
            assert_eq!(got, expected, "{words:?}");
        }
        assert!(may_be_rate(Decimal::ZERO));
        assert!(may_be_rate(Decimal::ONE_HUNDRED));
        assert!(!may_be_rate(Decimal::NEGATIVE_ONE));
        assert!(!may_be_rate(Decimal::ONE_THOUSAND));
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
