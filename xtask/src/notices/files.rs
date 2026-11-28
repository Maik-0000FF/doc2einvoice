// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The files of the package of a library: which there are, what their names
//! say they are, and whether one of them holds a given text.

use std::path::Path;

/// What the first word of the name of a licence file is, in lower case.
const LICENSE_WORDS: [&str; 4] = ["licence", "license", "copying", "unlicense"];
/// What the first word of the name of a notice file is, in lower case.
const NOTICE_WORDS: [&str; 2] = ["notice", "notices"];
/// What ends a word in the name of a file.
const WORD_ENDS: [char; 3] = ['.', '-', '_'];
/// The extensions of the program code of a library, in lower case: a file
/// that ends in one is code, whatever its name starts with.
const PROGRAM_EXTENSIONS: [&str; 5] = ["rs", "js", "mjs", "cjs", "ts"];

/// What the name of a file says the file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Document {
    /// The text of a licence.
    License,
    /// A notice that is passed on with the library.
    Notice,
}

/// What a file of a package is, by its name. Only the name of a regular file
/// is asked here; a directory is nothing of the kind.
///
/// A name that only might be a document counts as one: the generator then
/// stops and asks, where the other reading would pass a notice over silently.
///
/// | Name                  | Document             |
/// |-----------------------|----------------------|
/// | `LICENSE`             | licence              |
/// | `LICENSE.md`          | licence              |
/// | `licence.txt`         | licence              |
/// | `LICENSE-MIT`         | licence              |
/// | `LICENSE_APACHE-2.0`  | licence              |
/// | `license-apache-2.0`  | licence              |
/// | `LICENSE.spdx`        | licence              |
/// | `COPYING`             | licence              |
/// | `UNLICENSE`           | licence              |
/// | `NOTICE`              | notice               |
/// | `NOTICE.txt`          | notice               |
/// | `NOTICES`             | notice               |
/// | `NOTICE-THIRD-PARTY`  | notice               |
/// | `NOTICE_APACHE`       | notice               |
/// | `notice_board.rs`     | none: program code   |
/// | `license.js`          | none: program code   |
/// | `LICENSE.RS`          | none: program code   |
/// | `noticeboard.md`      | none: another word   |
/// | `relicense.md`        | none: another word   |
/// | `README.md`           | none                 |
/// | `.license`            | none: no first word  |
/// | empty                 | none                 |
pub fn document(name: &str) -> Option<Document> {
    let name = name.to_lowercase();
    let extension = Path::new(&name)
        .extension()
        .and_then(|extension| extension.to_str());
    if extension.is_some_and(|extension| PROGRAM_EXTENSIONS.contains(&extension)) {
        return None;
    }
    let word = name.split(WORD_ENDS).next().unwrap_or_default();
    if LICENSE_WORDS.contains(&word) {
        Some(Document::License)
    } else if NOTICE_WORDS.contains(&word) {
        Some(Document::Notice)
    } else {
        None
    }
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

    use super::{Document, document, holds_text, regular_files};

    #[test]
    fn what_a_name_says_follows_its_table() {
        let table = [
            ("LICENSE", Some(Document::License)),
            ("LICENSE.md", Some(Document::License)),
            ("licence.txt", Some(Document::License)),
            ("LICENSE-MIT", Some(Document::License)),
            ("LICENSE_APACHE-2.0", Some(Document::License)),
            ("license-apache-2.0", Some(Document::License)),
            ("LICENSE.spdx", Some(Document::License)),
            ("COPYING", Some(Document::License)),
            ("UNLICENSE", Some(Document::License)),
            ("NOTICE", Some(Document::Notice)),
            ("NOTICE.txt", Some(Document::Notice)),
            ("NOTICES", Some(Document::Notice)),
            ("NOTICE-THIRD-PARTY", Some(Document::Notice)),
            ("NOTICE_APACHE", Some(Document::Notice)),
            ("notice_board.rs", None),
            ("license.js", None),
            ("LICENSE.RS", None),
            ("noticeboard.md", None),
            ("relicense.md", None),
            ("README.md", None),
            (".license", None),
            ("", None),
        ];
        for (name, expected) in table {
            assert_eq!(document(name), expected, "{name}");
        }
    }

    /// The directory of this crate: it holds its manifest and its build
    /// script as files and its code in a directory.
    const CRATE: &str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn only_regular_files_are_named() -> std::io::Result<()> {
        let files = regular_files(Path::new(CRATE))?;

        assert!(files.contains(&"Cargo.toml".to_owned()));
        assert!(files.contains(&"build.rs".to_owned()));
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
