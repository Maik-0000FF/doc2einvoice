// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The rule that decides whether a library needs the settings for its licence
//! texts, and when the list is not generated.

use thiserror::Error;

use super::settings::{SETTINGS_FILE, Settling};

/// Where the licence texts of a library come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled<'a> {
    /// From its own package, found there without help.
    OwnTexts,
    /// From its own package and from what the settings say.
    Settings(&'a Settling),
}

/// Why the texts of a library are not settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Unsettled {
    /// Nothing says where a text comes from.
    #[error(
        "its package does not hold the texts of the licences it states; settle it in {SETTINGS_FILE}"
    )]
    NoText,
    /// The settings settle what needs no settling any more.
    #[error(
        "its package holds the texts of the licences it states, so its entry in {SETTINGS_FILE} is not needed"
    )]
    NotNeeded,
}

/// Decides where the licence texts of a library come from.
///
/// A package holds what its library states when the licences whose texts
/// were found in it fulfil the stated licence: every one of those it asks
/// for together, and one of those it offers a choice of.
///
/// | Its package holds what it states | The settings settle it | Outcome              |
/// |----------------------------------|------------------------|----------------------|
/// | yes                              | no                     | its own texts        |
/// | yes                              | yes                    | refused: not needed  |
/// | no                               | yes                    | what the settings say |
/// | no                               | no                     | refused: no text     |
pub fn settle(
    package_holds_what_it_states: bool,
    settling: Option<&Settling>,
) -> Result<Settled<'_>, Unsettled> {
    match (package_holds_what_it_states, settling) {
        (true, None) => Ok(Settled::OwnTexts),
        (true, Some(_)) => Err(Unsettled::NotNeeded),
        (false, Some(settling)) => Ok(Settled::Settings(settling)),
        (false, None) => Err(Unsettled::NoText),
    }
}

#[cfg(test)]
mod tests {
    use super::{Settled, Unsettled, settle};
    use crate::notices::settings::Settling;

    #[test]
    fn where_the_texts_come_from_follows_its_table() {
        let settling = Settling::StandardText;
        let table = [
            (true, None, Ok(Settled::OwnTexts)),
            (true, Some(&settling), Err(Unsettled::NotNeeded)),
            (false, Some(&settling), Ok(Settled::Settings(&settling))),
            (false, None, Err(Unsettled::NoText)),
        ];
        for (holds, settling, expected) in table {
            assert_eq!(settle(holds, settling), expected, "{holds} {settling:?}");
        }
    }
}
