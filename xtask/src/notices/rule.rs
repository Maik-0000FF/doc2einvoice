// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The rule that decides where the licence text of a library comes from, and
//! when the list is not generated.

use thiserror::Error;

use super::settings::{Clarification, SETTINGS_FILE};

/// Where the licence text of a library comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled<'a> {
    /// From its own package, found there without help.
    OwnText,
    /// From the file of its package the settings name.
    Clarified(&'a Clarification),
    /// The standard text of its licence, because its package holds none.
    StandardText,
}

/// Why the text of a library is not settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Unsettled {
    /// Nothing says where its text comes from.
    #[error("its package holds no licence text that is recognised; settle it in {SETTINGS_FILE}")]
    NoText,
    /// The settings settle what needs no settling any more.
    #[error("its package holds its licence text, so its entry in {SETTINGS_FILE} is not needed")]
    NotNeeded,
    /// The settings settle it in two ways.
    #[error("{SETTINGS_FILE} both clarifies it and lets the standard text stand for it")]
    Both,
}

/// Decides where the licence text of a library comes from.
///
/// A library lacks a text of its own when, for a licence that applies to it,
/// no text was found in its package.
///
/// | Lacks a text of its own | Clarified | Standard text allowed | Outcome                |
/// |-------------------------|-----------|-----------------------|------------------------|
/// | no                      | no        | no                    | its own text           |
/// | no                      | yes       | no                    | refused: not needed    |
/// | no                      | no        | yes                   | refused: not needed    |
/// | yes                     | yes       | no                    | the clarified file     |
/// | yes                     | no        | yes                   | the standard text      |
/// | yes                     | no        | no                    | refused: no text       |
/// | either                  | yes       | yes                   | refused: both          |
pub fn settle(
    lacks_own_text: bool,
    clarification: Option<&Clarification>,
    standard_text_allowed: bool,
) -> Result<Settled<'_>, Unsettled> {
    match (lacks_own_text, clarification, standard_text_allowed) {
        (_, Some(_), true) => Err(Unsettled::Both),
        (false, None, false) => Ok(Settled::OwnText),
        (false, _, _) => Err(Unsettled::NotNeeded),
        (true, Some(clarification), false) => Ok(Settled::Clarified(clarification)),
        (true, None, true) => Ok(Settled::StandardText),
        (true, None, false) => Err(Unsettled::NoText),
    }
}

#[cfg(test)]
mod tests {
    use super::{Settled, Unsettled, settle};
    use crate::notices::settings::Clarification;

    #[test]
    fn where_a_text_comes_from_follows_its_table() {
        let clarification = Clarification {
            license: "MIT".to_owned(),
            file: "LICENSE".to_owned(),
        };
        let clarified = Some(&clarification);
        let table = [
            (false, None, false, Ok(Settled::OwnText)),
            (false, clarified, false, Err(Unsettled::NotNeeded)),
            (false, None, true, Err(Unsettled::NotNeeded)),
            (
                true,
                clarified,
                false,
                Ok(Settled::Clarified(&clarification)),
            ),
            (true, None, true, Ok(Settled::StandardText)),
            (true, None, false, Err(Unsettled::NoText)),
            (false, clarified, true, Err(Unsettled::Both)),
            (true, clarified, true, Err(Unsettled::Both)),
        ];
        for (lacks, clarification, standard, expected) in table {
            assert_eq!(
                settle(lacks, clarification, standard),
                expected,
                "{lacks} {clarification:?} {standard}"
            );
        }
    }
}
