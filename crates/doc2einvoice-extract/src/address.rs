// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Addresses printed without labels: the name, the lines, the postal code
//! and the place of a party, found by where they stand.
//!
//! An address ends with the line of its postal code and place. Above it in
//! the same column stand its other lines; the first of them, from the top,
//! is the name, the others are the lines of the address, in the order the
//! standard has them too: the first line, as the street, then a second and a
//! third, as a floor. Where an address stands says whose it is:
//!
//! | Where                                              | Whose      | Postal code checked |
//! |----------------------------------------------------|------------|---------------------|
//! | in the section of a party ("Verkäufer:")           | that party | no: the place says it is an address |
//! | in the address zone of the window (DIN 5008)       | buyer      | no                  |
//! | the return address in the zone above it ("A · B · C") | seller  | yes, with `GeoNames`: separators alone do not make an address |
//! | a line of the letterhead in that form ("A - B - C") | seller    | yes, with `GeoNames` |
//! | beside the address under the heading of a section, in a column of its own | buyer | no |
//!
//! Otherwise the letterhead and the footer are not read: they often hold
//! several addresses, as of offices and banks, and nothing says whose an
//! address there is.

use std::ops::RangeInclusive;

use doc2einvoice_pdf::Word;

use crate::labels;
use crate::page::{self, Line, Span, same_baseline};
use crate::postcodes;

/// The words a country is written in before a postal code, as in
/// "DE 80333 München" or "D-80333 München", and the country each stands for.
const COUNTRY_WORDS: [(&str, &str); 7] = [
    ("D", "DE"),
    ("DE", "DE"),
    ("A", "AT"),
    ("AT", "AT"),
    ("CH", "CH"),
    ("F", "FR"),
    ("FR", "FR"),
];
const COUNTRY_JOINER: char = '-';

/// The names a country is written out with on a line of its own below the
/// place of an address, in German, English and French, as
/// [`labels::normalized`] gives them, and the code of each. They are those
/// of the countries whose postal codes are read.
const COUNTRY_NAMES: [(&str, &str); 11] = [
    ("deutschland", "DE"),
    ("germany", "DE"),
    ("allemagne", "DE"),
    ("frankreich", "FR"),
    ("france", "FR"),
    ("österreich", "AT"),
    ("austria", "AT"),
    ("autriche", "AT"),
    ("schweiz", "CH"),
    ("switzerland", "CH"),
    ("suisse", "CH"),
];

/// How many digits a postal code of the countries read has: four in
/// Austria and Switzerland, five in Germany and France.
const POSTCODE_DIGITS: RangeInclusive<usize> = 4..=5;

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

/// What parts a line of the letterhead besides: a hyphen standing alone, as
/// in `Muster SARL - 1 rue du Stade - 75001 Paris`. In the zone of the
/// return address it is not taken for a separator.
const LETTERHEAD_SEPARATOR: &str = "-";

/// Points in a millimetre.
pub const POINTS_PER_MM: f64 = 72.0 / 25.4;

/// The window of a letter after DIN 5008, in millimetres from the top left
/// corner of the page: its left edge and width, how far its top lies below
/// the top of the page in form A and in form B, its height, and the height
/// of its zone for additions and notes, where the return address stands,
/// above the address.
const WINDOW_LEFT_MM: f64 = 20.0;
const WINDOW_WIDTH_MM: f64 = 85.0;
const FORM_A_TOP_MM: f64 = 27.0;
const FORM_B_TOP_MM: f64 = 45.0;
const WINDOW_HEIGHT_MM: f64 = 45.0;
const ADDITIONS_ZONE_MM: f64 = 17.7;

/// How far left of the window its text may start, in millimetres, as
/// printers do not hold its edge exactly, and how far into it.
const WINDOW_TEXT_SLACK_MM: f64 = 5.0;
const WINDOW_TEXT_INDENT_MM: f64 = 20.0;
const WINDOW_TEXT_START_MM: RangeInclusive<f64> =
    (WINDOW_LEFT_MM - WINDOW_TEXT_SLACK_MM)..=(WINDOW_LEFT_MM + WINDOW_TEXT_INDENT_MM);

/// Where across the page the window reaches: what stands further right, as
/// the information block of DIN 5008, is not beside its lines.
const WINDOW_ACROSS_MM: RangeInclusive<f64> =
    (WINDOW_LEFT_MM - WINDOW_TEXT_SLACK_MM)..=(WINDOW_LEFT_MM + WINDOW_WIDTH_MM);

/// The heights of the address zones of both forms together: below the zone
/// for additions of form A, down to the bottom of the window of form B. A
/// letterhead above it is not in it.
const ADDRESS_ZONE_MM: RangeInclusive<f64> =
    (FORM_A_TOP_MM + ADDITIONS_ZONE_MM)..=(FORM_B_TOP_MM + WINDOW_HEIGHT_MM);

/// The heights of the zones for additions of both forms together, where
/// the return address stands.
const RETURN_ZONE_MM: RangeInclusive<f64> = FORM_A_TOP_MM..=(FORM_B_TOP_MM + ADDITIONS_ZONE_MM);

/// The page a window stands on: the first.
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
    /// The country written before the postal code, if one is.
    pub country: Option<Country<'a>>,
}

/// The country of an address, as it is written before the postal code.
pub struct Country<'a> {
    /// The code of the country.
    pub code: &'static str,
    /// The word it is written in.
    pub word: Span<'a>,
    /// Whether the word holds the code itself, as "DE", or something that
    /// stands for it: an older mark of the country, as the "D" of "D-80333",
    /// or its name written out.
    pub as_code: bool,
}

/// The country written before a postal code: its code, and whether the
/// code itself is what is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Written {
    code: &'static str,
    as_code: bool,
}

/// A postal code that starts a line: its digits, the country written before
/// it if any, and how many words both take.
#[derive(Debug, PartialEq, Eq)]
struct Postcode {
    digits: String,
    country: Option<Written>,
    taken: usize,
}

/// The postal code that starts `words`, after any country in front of it;
/// a place must follow.
///
/// | Words                         | Postal code | Country | Taken |
/// |-------------------------------|-------------|---------|-------|
/// | `80333` `München`             | `80333`     | none    | 1     |
/// | `DE` `80333` `München`        | `80333`     | DE, as its code | 2 |
/// | `DE-80333` `München`          | `80333`     | DE, as its code | 1 |
/// | `D-80333` `München`           | `80333`     | DE, by an older mark | 1 |
/// | `A-1010` `Wien`               | `1010`      | AT, by an older mark | 1 |
/// | `1010` `Wien`                 | `1010`      | none    | 1     |
/// | `80333`                       | none (no place) |     |       |
/// | `12345678` `Stadt`            | none (too many digits) | | |
/// | `Hauptstr.` `1`               | none (a street) |     |       |
/// | `Tel.` `0800` `123`           | none        |         |       |
fn postcode_starting(words: &[&str]) -> Option<Postcode> {
    let (first, rest) = words.split_first()?;
    let country_of = |word: &str| {
        COUNTRY_WORDS
            .iter()
            .find(|(written, _)| *written == word)
            .map(|(written, code)| Written {
                code,
                as_code: written == code,
            })
    };
    let (code, country, taken) = if let Some(country) = country_of(first) {
        (*rest.first()?, Some(country), 2)
    } else if let Some((written, code)) = first.split_once(COUNTRY_JOINER)
        && let Some(country) = country_of(written)
    {
        (code, Some(country), 1)
    } else {
        (*first, None, 1)
    };
    let is_code = POSTCODE_DIGITS.contains(&code.len()) && code.chars().all(|c| c.is_ascii_digit());
    let has_place = words
        .get(taken)
        .is_some_and(|place| place.chars().any(char::is_alphabetic));
    (is_code && has_place).then(|| Postcode {
        digits: code.to_owned(),
        country,
        taken,
    })
}

/// The country a line names that holds nothing else.
///
/// | Line               | Country |
/// |--------------------|---------|
/// | `Deutschland`      | DE      |
/// | `GERMANY`          | DE      |
/// | `France`           | FR      |
/// | `Schweiz`          | CH      |
/// | `Deutschland` `GmbH` | none (more than the name) |
/// | `Bayern`           | none    |
/// | nothing            | none    |
fn country_named(words: &[&str]) -> Option<&'static str> {
    let [word] = words else {
        return None;
    };
    let name = labels::normalized(word);
    COUNTRY_NAMES
        .iter()
        .find(|(written, _)| *written == name)
        .map(|(_, code)| *code)
}

/// The address that ends in `region`, lines of one page in reading order:
/// the first line that starts with a postal code, and the lines above it in
/// its column, up to a gap wider than a line, a heading, a note of the window
/// as "Persönlich", a label, or a line
/// with text beside it other than a label of the address, as "Anschrift:".
/// Only text whose left edge lies in `across`, in points, counts as beside.
///
/// | Lines from the top                                   | Name, address lines        |
/// |------------------------------------------------------|----------------------------|
/// | `Kunden AG`, `Hauptstr. 1`, `80333 München`          | `Kunden AG`; `Hauptstr. 1` |
/// | `Metallbau GmbH`, `Pappelallee 15`, `Hof 3`, place   | `Metallbau GmbH`; `Pappelallee 15`, `Hof 3` |
/// | `Verkäufer`, `Kunden AG`, street, place              | `Kunden AG`; street (a heading ends it) |
/// | `Persönlich`, `Kunden AG`, street, place             | `Kunden AG`; street (a note ends it) |
/// | `Nummer` beside `: 549910`, `Kunden AG`, street, place | `Kunden AG`; street      |
/// | `Globale Nummer: 4000…`, `Kunden AG`, street, place  | `Kunden AG`; street        |
/// | `Kunden AG` with `Kundennummer: 4711` outside `across`, street, place | `Kunden AG`; street |
/// | `Kunden AG`, place                                   | `Kunden AG`; none          |
/// | `Hauptstr. 1` beside `Anschrift:`, place             | `Hauptstr. 1`; none (the caller knows a labelled name) |
pub fn in_region<'a>(
    lines: &[Line<'a>],
    region: &[usize],
    across: &RangeInclusive<f64>,
) -> Option<Address<'a>> {
    let (at, postcode) = region.iter().find_map(|index| {
        postcode_starting(&lines[*index].texts()).map(|postcode| (*index, postcode))
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
        let texts = line.texts();
        let baseline = line.words[0].baseline;
        let ends = lowest - baseline > reach
            || labels::heading(&texts).is_some()
            || labels::is_window_note(&texts)
            || texts.iter().any(|text| text.ends_with(LABEL_END))
            || beside_is_other(lines, index, across);
        if ends {
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
    // The country may be written out on the line below the place, in its
    // column: the name stands for the code.
    let named = region
        .iter()
        .map(|index| &lines[*index])
        .filter(|line| {
            line.words.first().is_some_and(|word| {
                let below = word.baseline - head.baseline;
                (word.left - head.left).abs() <= slack && below > 0.0 && below <= reach
            })
        })
        .min_by(|a, b| a.words[0].baseline.total_cmp(&b.words[0].baseline))
        .and_then(|line| {
            country_named(&line.texts()).map(|code| Country {
                code,
                word: whole(line),
                as_code: false,
            })
        });
    Some(Address {
        name: spans.next(),
        lines: spans.collect(),
        postcode: Span {
            page: place_line.page,
            words: vec![&place_line.words[postcode.taken - 1]],
        },
        postcode_digits: postcode.digits,
        place: Span {
            page: place_line.page,
            words: place_line.words[postcode.taken..].iter().collect(),
        },
        country: postcode
            .country
            .and_then(|written| {
                Some(Country {
                    code: written.code,
                    word: Span {
                        page: place_line.page,
                        words: vec![place_line.words.first()?],
                    },
                    as_code: written.as_code,
                })
            })
            .or(named),
    })
}

/// Whether text other than a label of the address stands beside `line`,
/// where its left edge lies in `across`.
fn beside_is_other(lines: &[Line], line: usize, across: &RangeInclusive<f64>) -> bool {
    let beside: Vec<&str> = page::words_beside(lines, line)
        .into_iter()
        .filter(|word| across.contains(&word.left))
        .map(|word| word.text.as_str())
        .collect();
    !beside.is_empty() && !labels::names_address(&beside)
}

fn whole<'a>(line: &Line<'a>) -> Span<'a> {
    Span {
        page: line.page,
        words: line.words.iter().collect(),
    }
}

/// Where across the page anything counts, in points: all of it.
pub const WHOLE_PAGE: RangeInclusive<f64> = f64::MIN..=f64::MAX;

/// Where across the page the window reaches, in points.
pub fn window_across() -> RangeInclusive<f64> {
    points(&WINDOW_ACROSS_MM)
}

fn points(millimetres: &RangeInclusive<f64>) -> RangeInclusive<f64> {
    (millimetres.start() * POINTS_PER_MM)..=(millimetres.end() * POINTS_PER_MM)
}

/// Whether `word` starts a line of the window, in the zone of heights
/// `zone`.
fn starts_in_window(word: &Word, zone: &RangeInclusive<f64>) -> bool {
    points(&WINDOW_TEXT_START_MM).contains(&word.left) && points(zone).contains(&word.baseline)
}

/// The lines of the first page that stand in the address zone of the
/// window, without those of the return address, which may stand in it too.
pub fn window(lines: &[Line]) -> Vec<usize> {
    let returns = return_baselines(lines);
    (0..lines.len())
        .filter(|index| {
            let line = &lines[*index];
            line.page == WINDOW_PAGE
                && line.words.first().is_some_and(|word| {
                    starts_in_window(word, &ADDRESS_ZONE_MM)
                        && !returns
                            .iter()
                            .any(|baseline| same_baseline(*baseline, word.baseline, word.font_size))
                })
        })
        .collect()
}

/// The baselines of the first page in the zone for additions of the window
/// whose line holds a separator of a return address.
fn return_baselines(lines: &[Line]) -> Vec<f64> {
    let across = window_across();
    lines
        .iter()
        .filter(|line| line.page == WINDOW_PAGE)
        .filter(|line| {
            line.words.first().is_some_and(|word| {
                across.contains(&word.left) && points(&RETURN_ZONE_MM).contains(&word.baseline)
            })
        })
        .filter(|line| line.texts().iter().any(|text| SEPARATORS.contains(text)))
        .filter_map(|line| line.words.first().map(|word| word.baseline))
        .collect()
}

/// Which words part a line that holds an address in one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parted {
    /// As the return address: by its separators.
    ReturnAddress,
    /// As a line of the letterhead: by those and by a hyphen standing alone.
    Letterhead,
}

impl Parted {
    fn parts(self, word: &str) -> bool {
        SEPARATORS.contains(&word) || (self == Self::Letterhead && word == LETTERHEAD_SEPARATOR)
    }
}

/// The address the words of one baseline hold in one line: cut at the
/// separators, the part that is a postal code and place `GeoNames` confirms
/// ends the address, the parts before it are its lines, the first part the
/// name. The name of a country in the part after it stands for its code.
///
/// | Parts                                          | Name, lines, place  |
/// |------------------------------------------------|---------------------|
/// | `Muster GmbH` · `Hauptstr. 1` · `80333 München` | all three          |
/// | `Muster GmbH` · `80333 München`                | name and place      |
/// | `Au bon moulin` - `1 rue du Stade` - `75001 Paris` - `France`, in the letterhead | all three, and FR |
/// | the same, as a return address                  | none (a hyphen parts nothing there) |
/// | `Muster GmbH` · `Hauptstr. 1` · `12345 Stadthausen` | none (not a place) |
/// | `Tel. 0800` · `Fax 0801`                       | none                |
/// | `80333 München`, no separator                  | none (not one line of an address) |
fn in_one_line<'a>(mut words: Vec<&'a Word>, page: usize, parted: Parted) -> Option<Address<'a>> {
    words.sort_by(|a, b| a.left.total_cmp(&b.left));
    if !words.iter().any(|word| parted.parts(&word.text)) {
        return None;
    }
    let parts: Vec<Vec<&Word>> = words
        .split(|word| parted.parts(&word.text))
        .filter(|part| !part.is_empty())
        .map(<[_]>::to_vec)
        .collect();
    let texts = |part: &[&'a Word]| -> Vec<&'a str> {
        part.iter().map(|word| word.text.as_str()).collect()
    };
    let (index, postcode) = parts.iter().enumerate().find_map(|(index, part)| {
        let texts = texts(part);
        let postcode = postcode_starting(&texts)?;
        let place = texts[postcode.taken..].join(" ");
        postcodes::confirms(
            postcode.country.map(|written| written.code),
            &postcode.digits,
            &place,
        )
        .then_some((index, postcode))
    })?;
    let part = |at: usize| Span {
        page,
        words: parts[at].clone(),
    };
    let written = postcode.country.and_then(|written| {
        Some(Country {
            code: written.code,
            word: Span {
                page,
                words: vec![*parts[index].first()?],
            },
            as_code: written.as_code,
        })
    });
    let named = parts.get(index + 1).and_then(|after| {
        country_named(&texts(after)).map(|code| Country {
            code,
            word: part(index + 1),
            as_code: false,
        })
    });
    Some(Address {
        name: (index > 0).then(|| part(0)),
        lines: (1..index).map(part).collect(),
        postcode: Span {
            page,
            words: vec![parts[index][postcode.taken - 1]],
        },
        postcode_digits: postcode.digits,
        place: Span {
            page,
            words: parts[index][postcode.taken..].to_vec(),
        },
        country: written.or(named),
    })
}

/// The return address in the zone for additions of the window: the words
/// of a baseline there that holds separators, as far as the window reaches,
/// read as an address in one line. A line with separators in the footer is
/// none: it is not in the zone.
pub fn return_address<'a>(lines: &[Line<'a>]) -> Option<Address<'a>> {
    let across = window_across();
    return_baselines(lines).into_iter().find_map(|baseline| {
        let words: Vec<&'a Word> = lines
            .iter()
            .filter(|line| line.page == WINDOW_PAGE)
            .flat_map(|line| line.words.iter())
            .filter(|word| {
                same_baseline(word.baseline, baseline, word.font_size)
                    && across.contains(&word.left)
            })
            .collect();
        in_one_line(words, WINDOW_PAGE, Parted::ReturnAddress)
    })
}

/// The address of the sender in the letterhead: the first of the lines
/// `letterhead`, those of the first page above everything labelled, that
/// holds an address in one line, parted by separators or hyphens standing
/// alone, with the words on its baseline.
///
/// | Lines of the letterhead                                  | Address        |
/// |----------------------------------------------------------|----------------|
/// | `Au bon moulin - 1 rue du Stade - 75001 Paris - France`  | it, and FR     |
/// | `+33 4 50 44 68 99 - www.example.org`                    | none           |
/// | `Au bon moulin`, `1 rue du Stade`, `75001 Paris` below each other | none (not one line) |
pub fn letterhead<'a>(lines: &[Line<'a>], letterhead: &[usize]) -> Option<Address<'a>> {
    letterhead.iter().find_map(|index| {
        let line = lines.get(*index)?;
        let head = line.words.first()?;
        let words: Vec<&'a Word> = lines
            .iter()
            .filter(|other| other.page == line.page)
            .flat_map(|other| other.words.iter())
            .filter(|word| same_baseline(word.baseline, head.baseline, word.font_size))
            .collect();
        in_one_line(words, line.page, Parted::Letterhead)
    })
}

/// The addresses that end in `region`, each as the lines of the region in
/// its column: for every line that starts with a postal code, the lines
/// that start where it does. Two addresses beside each other are two
/// columns; a line that starts with a postal code in the column of one
/// before it adds none.
///
/// | Lines of the region (left edge)                         | Addresses |
/// |---------------------------------------------------------|-----------|
/// | name, street, `69001 Lyon` (all at 37)                  | one       |
/// | the same at 37 and again at 326                         | two       |
/// | `Nummer:` at 66, `DE 12345 Leipzig` at 305              | one, the lines at 305 |
/// | name, street (no postal code)                           | none      |
pub fn blocks(lines: &[Line], region: &[usize]) -> Vec<Vec<usize>> {
    let head_of = |index: usize| lines.get(index).and_then(|line| line.words.first());
    let mut blocks: Vec<Vec<usize>> = Vec::new();
    for index in region {
        let starts = postcode_starting(&lines[*index].texts()).is_some();
        let known = blocks.iter().any(|block| block.contains(index));
        if let (true, false, Some(head)) = (starts, known, head_of(*index)) {
            blocks.push(
                region
                    .iter()
                    .copied()
                    .filter(|other| head_of(*other).is_some_and(|word| same_column(word, head)))
                    .collect(),
            );
        }
    }
    blocks
}

/// Whether two words start in one column: no further apart than the slack
/// of a column at the font size of the second.
pub fn same_column(word: &Word, other: &Word) -> bool {
    (word.left - other.left).abs() <= other.font_size * COLUMN_SLACK
}

/// The stretch across the page in which only what starts in the column of
/// `word` lies: for an address beside another, whose lines are no text
/// beside its own.
pub fn column_of(word: &Word) -> RangeInclusive<f64> {
    let slack = word.font_size * COLUMN_SLACK;
    (word.left - slack)..=(word.left + slack)
}

#[cfg(test)]
mod tests {
    use super::{
        ADDRESS_ZONE_MM, Postcode, RETURN_ZONE_MM, Written, country_named, postcode_starting,
    };

    /// Words, and the postal code that starts them: its digits, the country
    /// written before it with whether its code is what is written, and the
    /// words both take.
    type Row = (
        &'static [&'static str],
        Option<(&'static str, Option<(&'static str, bool)>, usize)>,
    );

    #[test]
    fn named_countries_follow_their_table() {
        let table: [(&[&str], Option<&str>); 7] = [
            (&["Deutschland"], Some("DE")),
            (&["GERMANY"], Some("DE")),
            (&["France"], Some("FR")),
            (&["Schweiz"], Some("CH")),
            (&["Deutschland", "GmbH"], None),
            (&["Bayern"], None),
            (&[], None),
        ];
        for (words, expected) in table {
            assert_eq!(country_named(words), expected, "{words:?}");
        }
    }

    #[test]
    fn postcodes_start_lines_after_their_table() {
        let rows: [Row; 10] = [
            (&["80333", "München"], Some(("80333", None, 1))),
            (
                &["DE", "80333", "München"],
                Some(("80333", Some(("DE", true)), 2)),
            ),
            (
                &["DE-80333", "München"],
                Some(("80333", Some(("DE", true)), 1)),
            ),
            (
                &["D-80333", "München"],
                Some(("80333", Some(("DE", false)), 1)),
            ),
            (&["A-1010", "Wien"], Some(("1010", Some(("AT", false)), 1))),
            (&["1010", "Wien"], Some(("1010", None, 1))),
            (&["80333"], None),
            (&["12345678", "Stadt"], None),
            (&["Hauptstr.", "1"], None),
            (&["Tel.", "0800", "123"], None),
        ];
        for (words, expected) in rows {
            let expected = expected.map(|(digits, country, taken)| Postcode {
                digits: digits.to_owned(),
                country: country.map(|(code, as_code)| Written { code, as_code }),
                taken,
            });
            assert_eq!(postcode_starting(words), expected, "{words:?}");
        }
    }

    #[test]
    fn the_zones_of_the_window_follow_din_5008() {
        let (address, returns) = (ADDRESS_ZONE_MM, RETURN_ZONE_MM);
        assert!((address.start() - 44.7).abs() < 1e-9 && (address.end() - 90.0).abs() < 1e-9);
        assert!((returns.start() - 27.0).abs() < 1e-9 && (returns.end() - 62.7).abs() < 1e-9);
    }
}
