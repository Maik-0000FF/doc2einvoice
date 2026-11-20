// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Texts the core shows itself.
//!
//! Every text a user reads is drawn by the user interface. Where no interface
//! is left, because the web view that draws it was lost, the core has to speak
//! itself, through a dialog of the operating system. It states no text of its
//! own for that: the texts are those of the language files of the interface,
//! the one place texts are written, and they are part of the program.
//!
//! The language is the first of those the operating system reports that the
//! interface is translated into, as in the interface itself.

use std::collections::HashMap;

/// A language of the interface: its code and its language file.
struct Language {
    code: &'static str,
    messages: &'static str,
}

/// The languages the interface is translated into. The first one is used when
/// none of the preferred ones is available. A test holds this list against the
/// language files.
const LANGUAGES: [Language; 2] = [
    Language {
        code: "en",
        messages: include_str!("../../src/lib/i18n/messages/en.json"),
    },
    Language {
        code: "de",
        messages: include_str!("../../src/lib/i18n/messages/de.json"),
    },
];

/// Separates the language of a language tag from what follows it.
const SUBTAG_SEPARATOR: char = '-';

/// A text the core shows without the user interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Text {
    /// Title of the dialog that says the window can no longer be shown.
    WebViewLostTitle,
    /// What that dialog says.
    WebViewLostText,
}

impl Text {
    /// Every text, for the tests.
    #[cfg(test)]
    const ALL: [Self; 2] = [Self::WebViewLostTitle, Self::WebViewLostText];

    /// The name of the text in the language files.
    const fn key(self) -> &'static str {
        match self {
            Self::WebViewLostTitle => "fault.webViewLost.title",
            Self::WebViewLostText => "fault.webViewLost.text",
        }
    }
}

/// The text in the language of the operating system.
pub(crate) fn text(text: Text) -> String {
    in_language(chosen(sys_locale::get_locales()), text)
}

/// The text in the given language. If its language file lacks the text or
/// cannot be read, the name of the text is returned: something is shown, never
/// nothing. A test holds every language file against every text.
fn in_language(language: &Language, text: Text) -> String {
    serde_json::from_str::<HashMap<String, String>>(language.messages)
        .ok()
        .and_then(|mut messages| messages.remove(text.key()))
        .unwrap_or_else(|| text.key().to_owned())
}

/// Chooses the language from the languages the user prefers, in the order the
/// operating system states them.
///
/// | Preferred             | Result                                           |
/// |-----------------------|--------------------------------------------------|
/// | none                  | the default                                      |
/// | `de`                  | German                                           |
/// | `de-AT`, `de-Latn-DE` | German: region and script do not matter          |
/// | `DE`, `De-at`         | German: case does not matter                     |
/// | `fr`, `de`            | German: the first one that is available          |
/// | `en-US`, `de`         | English: the first one that is available         |
/// | `fr`                  | the default                                      |
/// | `dex`                 | the default: a different language                |
/// | `` (empty), `de`      | German: what names no language is skipped        |
/// | `de_DE`               | the default: no language tag, never guessed      |
fn chosen(preferred: impl IntoIterator<Item = String>) -> &'static Language {
    preferred
        .into_iter()
        .find_map(|tag| {
            let language = tag
                .split(SUBTAG_SEPARATOR)
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            LANGUAGES
                .iter()
                .find(|candidate| candidate.code == language)
        })
        .unwrap_or(&LANGUAGES[0])
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::{LANGUAGES, Text, chosen, in_language};

    const ENGLISH: &str = "en";
    const GERMAN: &str = "de";
    /// Where the language files lie, seen from the manifest of this crate.
    const LANGUAGE_FILES: &str = "../src/lib/i18n/messages";
    const LANGUAGE_FILE_EXTENSION: &str = "json";
    /// Opens a placeholder in a text of the interface.
    const PLACEHOLDER_START: char = '{';

    fn code_for(preferred: &[&str]) -> &'static str {
        chosen(preferred.iter().map(|tag| (*tag).to_owned())).code
    }

    #[test]
    fn the_language_is_chosen_as_the_table_states() {
        for (preferred, expected) in [
            (vec![], ENGLISH),
            (vec!["de"], GERMAN),
            (vec!["de-AT"], GERMAN),
            (vec!["de-Latn-DE"], GERMAN),
            (vec!["DE"], GERMAN),
            (vec!["De-at"], GERMAN),
            (vec!["fr", "de"], GERMAN),
            (vec!["en-US", "de"], ENGLISH),
            (vec!["fr"], ENGLISH),
            (vec!["dex"], ENGLISH),
            (vec!["", "de"], GERMAN),
            (vec!["de_DE"], ENGLISH),
        ] {
            assert_eq!(code_for(&preferred), expected, "{preferred:?}");
        }
    }

    #[test]
    fn every_language_has_every_text() {
        for language in &LANGUAGES {
            for text in Text::ALL {
                let shown = in_language(language, text);

                assert_ne!(shown, text.key(), "{} {text:?}", language.code);
                assert!(!shown.trim().is_empty(), "{} {text:?}", language.code);
                // Nothing fills in a placeholder here.
                assert!(
                    !shown.contains(PLACEHOLDER_START),
                    "{} {text:?}",
                    language.code
                );
            }
        }
    }

    #[test]
    fn the_languages_are_those_of_the_language_files() -> std::io::Result<()> {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join(LANGUAGE_FILES);
        let mut files = BTreeSet::new();
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|e| e == LANGUAGE_FILE_EXTENSION)
                && let Some(code) = path.file_stem().and_then(|stem| stem.to_str())
            {
                files.insert(code.to_owned());
            }
        }
        let known: BTreeSet<String> = LANGUAGES.iter().map(|l| l.code.to_owned()).collect();

        assert_eq!(known, files);
        Ok(())
    }
}
