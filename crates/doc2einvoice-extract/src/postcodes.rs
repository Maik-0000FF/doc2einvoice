// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The postal codes of Austria, Switzerland, Germany and France and the
//! places they belong to, as `GeoNames` publishes them
//! (`data/postcodes.tsv`, made by `tools/update-postcodes.sh`).
//!
//! They confirm that a line such as "80333 München" is the postal code and
//! place of an address, where nothing else on the page says that a line is
//! an address: "12345 Stück" has the same form.

use std::collections::HashMap;
use std::sync::LazyLock;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// The table, a header line and one line per postal code and place, the
/// fields separated by tabs: country, postal code, place.
const TABLE: &str = include_str!("../data/postcodes.tsv");

/// The fields of a line of the table.
const FIELDS: usize = 3;

/// Letters a place is printed in either way, as "DUESSELDORF" for
/// "Düsseldorf": each with the letters it stands for.
const SPELLED_OUT: [(char, &str); 4] = [('ä', "ae"), ('ö', "oe"), ('ü', "ue"), ('ß', "ss")];

/// A place shorter than this, once compared, confirms nothing: a single
/// letter begins too many places.
const SHORTEST_PLACE: usize = 3;

/// The countries of the table by the number of digits of their postal
/// codes, for a postal code printed without its country.
const COUNTRIES_BY_DIGITS: [(usize, &[&str]); 2] = [(4, &["AT", "CH"]), (5, &["DE", "FR"])];

/// A country and a postal code of it, and a place as it is compared: its
/// words.
type Key = (&'static str, &'static str);
type Place = Vec<String>;

/// The places of each country and postal code, as they are compared.
static PLACES: LazyLock<HashMap<Key, Vec<Place>>> = LazyLock::new(|| {
    let mut places: HashMap<Key, Vec<Place>> = HashMap::new();
    for line in TABLE.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if let [country, postcode, place] = fields[..]
            && fields.len() == FIELDS
        {
            places
                .entry((country, postcode))
                .or_default()
                .push(compared(place));
        }
    }
    places
});

/// A place as it is compared: its words, each in lower case, with umlauts
/// and ß spelled out, without accents, and without anything but letters and
/// digits.
///
/// | Place            | Compared               |
/// |------------------|------------------------|
/// | `Düsseldorf`     | `duesseldorf`          |
/// | `DUESSELDORF`    | `duesseldorf`          |
/// | `Gießen`         | `giessen`              |
/// | `Besançon`       | `besancon`             |
/// | `Paris 01`       | `paris`, `01`          |
/// | `Frankfurt am Main` | `frankfurt`, `am`, `main` |
/// | `Saint-Étienne`  | `saint`, `etienne`     |
fn compared(place: &str) -> Place {
    let lower = place.to_lowercase();
    let mut spelled = String::with_capacity(lower.len());
    for letter in lower.chars() {
        match SPELLED_OUT.iter().find(|(umlaut, _)| *umlaut == letter) {
            Some((_, letters)) => spelled.push_str(letters),
            None => spelled.push(letter),
        }
    }
    let plain: String = spelled.nfd().filter(|c| !is_combining_mark(*c)).collect();
    plain
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether `place` is a place of the postal code `postcode` in `country`,
/// or, if the country is not printed, in a country with postal codes of its
/// length. The words of the one are the first words of the other, as
/// "Frankfurt" of "Frankfurt am Main" and "Paris" of "Paris 01".
///
/// Without a country, a code of Austria also confirms a place of
/// Switzerland with the same code, and one of Germany one of France: the
/// line does not say which it is, and the place decides.
///
/// | Country | Postal code | Place          | Confirmed |
/// |---------|-------------|----------------|-----------|
/// | none    | `80333`     | `München`      | yes       |
/// | DE      | `80333`     | `München`      | yes       |
/// | FR      | `80333`     | `München`      | no (not in France) |
/// | none    | `40235`     | `DUESSELDORF`  | yes       |
/// | none    | `75001`     | `Paris`        | yes       |
/// | none    | `60311`     | `Frankfurt`    | yes       |
/// | none    | `69876`     | `Frankfurt`    | no (another place has that code) |
/// | none    | `3034`      | `Hofmann Logistik` | no (`Hof` is not the word `Hofmann`) |
/// | none    | `12345`     | `Stück`        | no        |
/// | none    | `99199`     | `Musterhausen` | no (no such code) |
/// | none    | `80333`     | `M`            | no (too short) |
pub fn confirms(country: Option<&str>, postcode: &str, place: &str) -> bool {
    let printed = compared(place);
    if printed.concat().chars().count() < SHORTEST_PLACE {
        return false;
    }
    let countries: Vec<&str> = match country {
        Some(country) => vec![country],
        None => COUNTRIES_BY_DIGITS
            .iter()
            .find(|(digits, _)| *digits == postcode.len())
            .map_or_else(Vec::new, |(_, countries)| countries.to_vec()),
    };
    countries.iter().any(|country| {
        PLACES.get(&(*country, postcode)).is_some_and(|places| {
            places
                .iter()
                .any(|known| known.starts_with(&printed) || printed.starts_with(known))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{compared, confirms};

    #[test]
    fn places_are_compared_after_their_table() {
        let rows: [(&str, &[&str]); 7] = [
            ("Düsseldorf", &["duesseldorf"]),
            ("DUESSELDORF", &["duesseldorf"]),
            ("Gießen", &["giessen"]),
            ("Besançon", &["besancon"]),
            ("Paris 01", &["paris", "01"]),
            ("Frankfurt am Main", &["frankfurt", "am", "main"]),
            ("Saint-Étienne", &["saint", "etienne"]),
        ];
        for (place, expected) in rows {
            assert_eq!(compared(place), expected, "{place}");
        }
    }

    #[test]
    fn places_are_confirmed_after_their_table() {
        let rows = [
            (None, "80333", "München", true),
            (Some("DE"), "80333", "München", true),
            (Some("FR"), "80333", "München", false),
            (None, "40235", "DUESSELDORF", true),
            (None, "75001", "Paris", true),
            (None, "60311", "Frankfurt", true),
            (None, "69876", "Frankfurt", false),
            (None, "3034", "Hofmann Logistik", false),
            (None, "12345", "Stück", false),
            (None, "99199", "Musterhausen", false),
            (None, "80333", "M", false),
        ];
        for (country, postcode, place, expected) in rows {
            assert_eq!(
                confirms(country, postcode, place),
                expected,
                "{postcode} {place}"
            );
        }
    }
}
