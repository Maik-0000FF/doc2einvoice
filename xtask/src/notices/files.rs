// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The files in the root of the package of a library: which there are, which
//! of them their names say are passed on, and whether one holds a given text.
//!
//! Nothing here guesses what a file is. A file whose name says licence,
//! notice or copyright is passed on with the library, word for word, unless
//! the settings name it as none of these. That is the one rule and the one
//! place to settle a file.

use std::path::Path;

/// What a name says when the file holds the text of a licence, in lower
/// case.
const LICENSE_WORDS: [&str; 3] = ["license", "licence", "copying"];
/// What a name says when the file holds a notice or a statement of
/// copyright, in lower case.
const NOTICE_WORDS: [&str; 2] = ["notice", "copyright"];

fn says(name: &str, words: &[&str]) -> bool {
    let name = name.to_lowercase();
    words.iter().any(|word| name.contains(word))
}

/// Whether the name of a file says it holds the text of a licence.
pub fn names_license(name: &str) -> bool {
    says(name, &LICENSE_WORDS)
}

/// Whether the name of a file in the root of a package says it is passed on
/// with the library.
///
/// | Name                    | Passed on                                |
/// |-------------------------|------------------------------------------|
/// | `LICENSE`               | yes                                      |
/// | `LICENSE-MIT`           | yes                                      |
/// | `license-apache-2.0`    | yes                                      |
/// | `licence.txt`           | yes                                      |
/// | `UNLICENSE`             | yes                                      |
/// | `COPYING`               | yes                                      |
/// | `COPYRIGHT`             | yes                                      |
/// | `NOTICE`                | yes                                      |
/// | `THIRD-PARTY-NOTICES`   | yes                                      |
/// | `LICENSE.spdx`          | yes                                      |
/// | `Notice.svelte`         | yes: the settings name it as none        |
/// | `notice_board.rs`       | yes: the settings name it as none        |
/// | `README.md`             | no                                       |
/// | `Cargo.toml`            | no                                       |
/// | empty                   | no                                       |
pub fn names_document(name: &str) -> bool {
    names_license(name) || says(name, &NOTICE_WORDS)
}

/// The names of the regular files a directory holds itself, in order. A
/// directory in it is not named, nor is what cannot be told to be a file.
pub fn regular_files(directory: &Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        // A link is followed: what it leads to decides.
        if std::fs::metadata(entry.path()).is_ok_and(|metadata| metadata.is_file()) {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Whether one of the named files of a directory holds the text, word for
/// word. A file that cannot be read does not hold it.
pub fn holds_text(directory: &Path, files: &[String], text: &str) -> bool {
    files.iter().any(|file| {
        std::fs::read(directory.join(file)).is_ok_and(|content| content == text.as_bytes())
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{holds_text, names_document, names_license, regular_files};

    #[test]
    fn what_is_passed_on_follows_its_table() {
        let table = [
            ("LICENSE", true, true),
            ("LICENSE-MIT", true, true),
            ("license-apache-2.0", true, true),
            ("licence.txt", true, true),
            ("UNLICENSE", true, true),
            ("COPYING", true, true),
            ("COPYRIGHT", true, false),
            ("NOTICE", true, false),
            ("THIRD-PARTY-NOTICES", true, false),
            ("LICENSE.spdx", true, true),
            ("Notice.svelte", true, false),
            ("notice_board.rs", true, false),
            ("README.md", false, false),
            ("Cargo.toml", false, false),
            ("", false, false),
        ];
        for (name, document, license) in table {
            assert_eq!(names_document(name), document, "{name}");
            assert_eq!(names_license(name), license, "{name}");
        }
    }

    /// The directory of this crate: it holds its manifest as a file and its
    /// code in a directory.
    const CRATE: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn only_regular_files_are_named() -> std::io::Result<()> {
        let files = regular_files(Path::new(CRATE))?;

        assert!(files.contains(&"Cargo.toml".to_owned()));
        assert!(!files.contains(&"src".to_owned()));
        assert!(files.is_sorted());
        Ok(())
    }

    #[test]
    fn a_directory_that_is_not_there_names_no_files() {
        assert!(regular_files(&Path::new(CRATE).join("no-such-directory")).is_err());
    }

    #[test]
    fn a_file_holds_a_text_only_word_for_word() -> std::io::Result<()> {
        let directory = Path::new(CRATE);
        let files = ["Cargo.toml".to_owned(), "no-such-file".to_owned()];
        let manifest = std::fs::read_to_string(directory.join("Cargo.toml"))?;

        assert!(holds_text(directory, &files, &manifest));
        assert!(!holds_text(directory, &files, manifest.trim_end()));
        assert!(!holds_text(directory, &files[1..], &manifest));
        assert!(!holds_text(directory, &[], &manifest));
        // A directory is not a file that holds a text.
        assert!(!holds_text(directory, &["src".to_owned()], ""));
        Ok(())
    }
}
