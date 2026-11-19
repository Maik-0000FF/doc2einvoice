// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Files of other publishers that are compiled into the program unchanged.

use sha2::{Digest, Sha256};

/// One embedded file with the checksum of its published form.
///
/// Rule files and XML Schema files are embedded exactly as published. The
/// checksum is verified before a file is used, so a file that was altered,
/// by accident or on purpose, is refused instead of silently trusted.
#[derive(Debug)]
pub struct EmbeddedFile {
    /// The name of the file, as a path inside the publication it belongs to.
    pub name: &'static str,
    /// The SHA-256 checksum of the published file, in lower-case hexadecimal.
    pub sha256: &'static str,
    pub(crate) text: &'static str,
}

impl EmbeddedFile {
    /// Whether the embedded text is byte for byte the published file.
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        hex(&Sha256::digest(self.text.as_bytes())) == self.sha256
    }
}

/// The first of the given files that is not the published file, if any.
pub(crate) fn first_altered(
    mut files: impl Iterator<Item = &'static EmbeddedFile>,
) -> Option<&'static EmbeddedFile> {
    files.find(|file| !file.is_unchanged())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            // Writing to a String cannot fail.
            let _ = write!(text, "{byte:02x}");
            text
        })
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedFile, first_altered, hex};

    // SHA-256 of the three bytes "abc".
    static UNCHANGED: EmbeddedFile = EmbeddedFile {
        name: "unchanged",
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        text: "abc",
    };
    static ALTERED: EmbeddedFile = EmbeddedFile {
        name: "altered",
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        text: "abd",
    };

    #[test]
    fn a_file_with_its_published_checksum_is_unchanged() {
        assert!(UNCHANGED.is_unchanged());
        assert!(!ALTERED.is_unchanged());
    }

    #[test]
    fn the_first_altered_file_is_found() {
        assert!(first_altered([&UNCHANGED].into_iter()).is_none());
        assert_eq!(
            first_altered([&UNCHANGED, &ALTERED].into_iter()).map(|file| file.name),
            Some("altered")
        );
    }

    #[test]
    fn bytes_are_written_as_lower_case_hexadecimal() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }
}
