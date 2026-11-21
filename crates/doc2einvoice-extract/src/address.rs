// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Addresses printed without labels: the name, street, postal code and
//! place of a party, found by where they stand.
//!
//! An address ends with the line of its postal code and place. Above it in
//! the same column stand its other lines; the first of them, from the top,
//! is the name, the others are the lines of the address, in the order the
//! standard has them too: the first line, as the street, then a second and a
//! third, as a floor. Where an address stands says whose it is:
//!
//! | Where                                         | Whose  | Postal code checked |
//! |-----------------------------------------------|--------|---------------------|
//! | in the section of a party ("Verkäufer:")      | that party | no (the place says it is an address) |
//! | in the address window of a letter (DIN 5008)  | buyer  | no                  |
//! | the return address above it ("A · B · C")     | seller | yes, with `GeoNames`: the separators alone do not make an address |
//!
//! The letterhead and the footer are not read: they often hold several
//! addresses, as of offices and banks.

use crate::labels;
use crate::page::{self, Line, Span, same_baseline};
use crate::postcodes;

/// Words or marks a country is written in before a postal code, as in
/// "DE 80333 München" or "D-80333 München".
const COUNTRY_WORDS: [&str; 7] = ["D", "DE", "A", "AT", "CH", "F", "FR"];
const COUNTRY_JOINER: char = '-';

/// How many digits a postal code of the countries read has: four in
/// Austria and Switzerland, five in Germany and France.
const POSTCODE_DIGITS: std::ops::RangeInclusive<usize> = 4..=5;

/// What ends a label, which no line of an address holds.
const LABEL_END: char = ':';

/// How far apart two lines of an address may stand, as a multiple of the
/// font size: the distance of a line, with room for a little more.
const ADDRESS_PITCH: f64 = 2.5;

/// How far left or right of the line of the postal code a line may start
/// and still stand in its column, as a multiple of the font size.
const COLUMN_SLACK: f64 = 1.0;

/// How many lines an address has above its postal code and place at most:
/// the name, and the three address lines of the standard.
const LINES_ABOVE: usize = 4;

/// What parts the return address of a letter, as in `Muster GmbH ·
/// Hauptstr. 1 · 12345 Stadt`.
const SEPARATORS: [&str; 5] = ["·", "•", "●", "|", "–"];

/// Points in a millimetre.
pub const POINTS_PER_MM: f64 = 72.0 / 25.4;

/// The address window of DIN 5008, for its forms A and B together, in
/// millimetres from the top left corner of the page: where its text starts
/// from the left, and between which heights it stands.
const WINDOW_LEFT_MM: std::ops::RangeInclusive<f64> = 15.0..=40.0;
const WINDOW_HEIGHT_MM: std::ops::RangeInclusive<f64> = 27.0..=90.0;

/// The page an address window stands on: the first.
const WINDOW_PAGE: usize = 0;

/// An address as read: each part with the words it was read from.
pub struct Address<'a> {
    pub name: Option<Span<'a>>,
    /// The address lines, from the first on.
    pub lines: Vec<Span<'a>>,
    /// The word of the postal code, and its digits without a country.
    pub postcode: Span<'a>,
    pub postcode_digits: String,
    pub place: Span<'a>,
}

/// The postal code that starts `words`, after any country in front of it,
/// and how many words it and the country take; a place must follow.
///
/// | Words                         | Postal code | Taken |
/// |-------------------------------|-------------|-------|
/// | `80333` `München`             | `80333`     | 1     |
/// | `DE` `80333` `München`        | `80333`     | 2     |
/// | `D-80333` `München`           | `80333`     | 1     |
/// | `1010` `Wien`                 | `1010`      | 1     |
/// | `80333`                       | none (no place) |   |
/// | `12345678` `Stadt`            | none (too many digits) | |
/// | `Hauptstr.` `1`               | none (a street) |   |
/// | `Tel.` `0800` `123`           | none        |       |
fn postcode_starting(words: &[&str]) -> Option<(String, usize)> {
    let (first, rest) = words.split_first()?;
    let (code, taken) = if COUNTRY_WORDS.contains(first) {
        (*rest.first()?, 2)
    } else if let Some((country, code)) = first.split_once(COUNTRY_JOINER)
        && COUNTRY_WORDS.contains(&country)
    {
        (code, 1)
    } else {
        (*first, 1)
    };
    let is_code = POSTCODE_DIGITS.contains(&code.len()) && code.chars().all(|c| c.is_ascii_digit());
    let has_place = words
        .get(taken)
        .is_some_and(|place| place.chars().any(char::is_alphabetic));
    (is_code && has_place).then(|| (code.to_owned(), taken))
}

/// The address that ends in `region`, lines of one page in reading order:
/// the first line that starts with a postal code, and the lines above it in
/// its column, up to a gap wider than a line, a label, or a line with text
/// beside it other than a label of the address, as "Anschrift:".
///
/// | Lines from the top                                   | Name, address lines        |
/// |------------------------------------------------------|----------------------------|
/// | `Kunden AG`, `Hauptstr. 1`, `80333 München`          | `Kunden AG`; `Hauptstr. 1` |
/// | `Metallbau GmbH`, `Pappelallee 15`, `Hof 3`, place   | `Metallbau GmbH`; `Pappelallee 15`, `Hof 3` |
/// | `Nummer` beside `: 549910`, `Kunden AG`, street, place | `Kunden AG`; street      |
/// | `Globale Nummer: 4000…`, `Kunden AG`, street, place  | `Kunden AG`; street        |
/// | `Kunden AG`, place                                   | `Kunden AG`; none          |
/// | `Hauptstr. 1` beside `Anschrift:`, place             | `Hauptstr. 1`; none (the caller knows a labelled name) |
pub fn in_region<'a>(lines: &[Line<'a>], region: &[usize]) -> Option<Address<'a>> {
    let (at, digits, taken) = region.iter().find_map(|index| {
        let texts = lines[*index].texts();
        postcode_starting(&texts).map(|(code, taken)| (*index, code, taken))
    })?;
    let place_line = &lines[at];
    let head = place_line.words.first()?;
    let slack = head.font_size * COLUMN_SLACK;
    let reach = head.font_size * ADDRESS_PITCH;
    let mut column: Vec<usize> = region
        .iter()
        .copied()
        .filter(|index| {
            lines[*index].words.first().is_some_and(|word| {
                (word.left - head.left).abs() <= slack && word.baseline < head.baseline
            })
        })
        .collect();
    column.sort_by(|a, b| {
        lines[*b].words[0]
            .baseline
            .total_cmp(&lines[*a].words[0].baseline)
    });
    let mut above: Vec<&Line<'a>> = Vec::new();
    let mut lowest = head.baseline;
    for index in column {
        let line = &lines[index];
        let baseline = line.words[0].baseline;
        let labelled = line.texts().iter().any(|text| text.ends_with(LABEL_END));
        let beside = page::text_beside(lines, index);
        let other_label = !beside.is_empty() && !labels::names_address(&beside);
        if lowest - baseline > reach || labelled || other_label {
            break;
        }
        above.push(line);
        lowest = baseline;
        if above.len() == LINES_ABOVE {
            break;
        }
    }
    above.reverse();
    let mut spans = above.iter().map(|line| whole(line));
    Some(Address {
        name: spans.next(),
        lines: spans.collect(),
        postcode: Span {
            page: place_line.page,
            words: vec![&place_line.words[taken - 1]],
        },
        postcode_digits: digits,
        place: Span {
            page: place_line.page,
            words: place_line.words[taken..].iter().collect(),
        },
    })
}

fn whole<'a>(line: &Line<'a>) -> Span<'a> {
    Span {
        page: line.page,
        words: line.words.iter().collect(),
    }
}

/// The lines of the first page that stand in the address window, without
/// the return address, which stands in it too.
pub fn window(lines: &[Line]) -> Vec<usize> {
    let returns = return_baselines(lines);
    (0..lines.len())
        .filter(|index| {
            let line = &lines[*index];
            line.page == WINDOW_PAGE
                && line.words.first().is_some_and(|word| {
                    WINDOW_LEFT_MM.contains(&(word.left / POINTS_PER_MM))
                        && WINDOW_HEIGHT_MM.contains(&(word.baseline / POINTS_PER_MM))
                        && !returns
                            .iter()
                            .any(|baseline| same_baseline(*baseline, word.baseline, word.font_size))
                })
        })
        .collect()
}

/// The baselines of the first page that hold a separator of a return
/// address.
fn return_baselines(lines: &[Line]) -> Vec<f64> {
    lines
        .iter()
        .filter(|line| line.page == WINDOW_PAGE)
        .filter(|line| line.texts().iter().any(|text| SEPARATORS.contains(text)))
        .filter_map(|line| line.words.first().map(|word| word.baseline))
        .collect()
}

/// The return address of the first page: the words of a baseline that holds
/// separators, cut at them; the part that is a postal code and place
/// `GeoNames` confirms ends the address, the part before it is the street,
/// the first part the name.
///
/// | Parts                                          | Name, street, place |
/// |------------------------------------------------|---------------------|
/// | `Muster GmbH` · `Hauptstr. 1` · `80333 München` | all three         |
/// | `Muster GmbH` · `80333 München`                | name and place      |
/// | `Muster GmbH` · `Hauptstr. 1` · `12345 Stadthausen` | none (not a place) |
/// | `Tel. 0800` · `Fax 0801`                       | none                |
pub fn return_address<'a>(lines: &[Line<'a>]) -> Option<Address<'a>> {
    for baseline in return_baselines(lines) {
        let mut words: Vec<&'a doc2einvoice_pdf::Word> = lines
            .iter()
            .filter(|line| line.page == WINDOW_PAGE)
            .flat_map(|line| line.words.iter())
            .filter(|word| same_baseline(word.baseline, baseline, word.font_size))
            .collect();
        words.sort_by(|a, b| a.left.total_cmp(&b.left));
        let parts: Vec<Vec<&doc2einvoice_pdf::Word>> = words
            .split(|word| SEPARATORS.contains(&word.text.as_str()))
            .filter(|part| !part.is_empty())
            .map(<[_]>::to_vec)
            .collect();
        let found = parts.iter().enumerate().find_map(|(index, part)| {
            let texts: Vec<&str> = part.iter().map(|word| word.text.as_str()).collect();
            let (code, taken) = postcode_starting(&texts)?;
            let place: Vec<&str> = texts[taken..].to_vec();
            postcodes::confirms(&code, &place.join(" ")).then_some((index, code, taken))
        });
        if let Some((index, digits, taken)) = found {
            let part = |at: usize| Span {
                page: WINDOW_PAGE,
                words: parts[at].clone(),
            };
            return Some(Address {
                name: (index > 0).then(|| part(0)),
                lines: (1..index).map(part).collect(),
                postcode: Span {
                    page: WINDOW_PAGE,
                    words: vec![parts[index][taken - 1]],
                },
                postcode_digits: digits,
                place: Span {
                    page: WINDOW_PAGE,
                    words: parts[index][taken..].to_vec(),
                },
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::postcode_starting;

    /// Words, and the postal code that starts them with the words it takes.
    type Row = (&'static [&'static str], Option<(&'static str, usize)>);

    #[test]
    fn postcodes_start_lines_after_their_table() {
        let rows: [Row; 8] = [
            (&["80333", "München"], Some(("80333", 1))),
            (&["DE", "80333", "München"], Some(("80333", 2))),
            (&["D-80333", "München"], Some(("80333", 1))),
            (&["1010", "Wien"], Some(("1010", 1))),
            (&["80333"], None),
            (&["12345678", "Stadt"], None),
            (&["Hauptstr.", "1"], None),
            (&["Tel.", "0800", "123"], None),
        ];
        for (words, expected) in rows {
            let found = postcode_starting(words);
            let expected = expected.map(|(code, taken)| (code.to_owned(), taken));
            assert_eq!(found, expected, "{words:?}");
        }
    }
}
