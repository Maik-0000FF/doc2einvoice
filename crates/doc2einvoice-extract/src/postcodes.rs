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

/// The places of each postal code, as they are compared.
static PLACES: LazyLock<HashMap<&'static str, Vec<String>>> = LazyLock::new(|| {
    let mut places: HashMap<&str, Vec<String>> = HashMap::new();
    for line in TABLE.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if let [_, postcode, place] = fields[..]
            && fields.len() == FIELDS
        {
            places.entry(postcode).or_default().push(compared(place));
        }
    }
    places
});

/// A place as it is compared: in lower case, with umlauts and ß spelled out,
/// without accents, and without anything but letters and digits.
///
/// | Place            | Compared          |
/// |------------------|-------------------|
/// | `Düsseldorf`     | `duesseldorf`     |
/// | `DUESSELDORF`    | `duesseldorf`     |
/// | `Gießen`         | `giessen`         |
/// | `Besançon`       | `besancon`        |
/// | `Paris 01`       | `paris01`         |
/// | `Frankfurt/M.`   | `frankfurtm`      |
fn compared(place: &str) -> String {
    let lower = place.to_lowercase();
    let mut spelled = String::with_capacity(lower.len());
    for letter in lower.chars() {
        match SPELLED_OUT.iter().find(|(umlaut, _)| *umlaut == letter) {
            Some((_, letters)) => spelled.push_str(letters),
            None => spelled.push(letter),
        }
    }
    spelled
        .nfd()
        .filter(|c| !is_combining_mark(*c) && c.is_alphanumeric())
        .collect()
}

/// Whether `place` is a place of the postal code `postcode`: the one as it
/// is compared begins with the other, as "Frankfurt" for "Frankfurt am
/// Main" and "Paris" for "Paris 01".
///
/// | Postal code | Place          | Confirmed |
/// |-------------|----------------|-----------|
/// | `80333`     | `München`      | yes       |
/// | `40235`     | `DUESSELDORF`  | yes       |
/// | `75001`     | `Paris`        | yes       |
/// | `60311`     | `Frankfurt`    | yes       |
/// | `69876`     | `Frankfurt`    | no (another place has that code) |
/// | `12345`     | `Stück`        | no        |
/// | `99199`     | `Musterhausen` | no (no such code) |
/// | `80333`     | `M`            | no (too short) |
pub fn confirms(postcode: &str, place: &str) -> bool {
    let printed = compared(place);
    if printed.chars().count() < SHORTEST_PLACE {
        return false;
    }
    PLACES.get(postcode).is_some_and(|places| {
        places
            .iter()
            .any(|known| known.starts_with(&printed) || printed.starts_with(known.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::{compared, confirms};

    #[test]
    fn places_are_compared_after_their_table() {
        let rows = [
            ("Düsseldorf", "duesseldorf"),
            ("DUESSELDORF", "duesseldorf"),
            ("Gießen", "giessen"),
            ("Besançon", "besancon"),
            ("Paris 01", "paris01"),
            ("Frankfurt/M.", "frankfurtm"),
        ];
        for (place, expected) in rows {
            assert_eq!(compared(place), expected, "{place}");
        }
    }

    #[test]
    fn places_are_confirmed_after_their_table() {
        let rows = [
            ("80333", "München", true),
            ("40235", "DUESSELDORF", true),
            ("75001", "Paris", true),
            ("60311", "Frankfurt", true),
            ("69876", "Frankfurt", false),
            ("12345", "Stück", false),
            ("99199", "Musterhausen", false),
            ("80333", "M", false),
        ];
        for (postcode, place, expected) in rows {
            assert_eq!(confirms(postcode, place), expected, "{postcode} {place}");
        }
    }
}
