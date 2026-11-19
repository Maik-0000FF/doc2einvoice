// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Loading the file a user chose into memory.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::outcome::Refusal;

/// The largest file that is read. An invoice with its attachments embedded is
/// a few megabytes; what is far larger is not an invoice, and reading it would
/// only exhaust the memory.
pub const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

/// The mark some editors put in front of a text in UTF-8. It is not part of
/// the document.
const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// A file read into memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The name of the file, without its directory.
    pub file_name: String,
    /// The content of the file.
    pub text: String,
}

/// Reads the file at the given path.
///
/// | Path leads to                                  | Result                       |
/// |------------------------------------------------|------------------------------|
/// | a file in UTF-8, up to the size limit          | the document                 |
/// | the same with a byte order mark in front       | the document without it      |
/// | an empty file                                  | the document, with no text   |
/// | a symbolic link to such a file                 | the document it leads to     |
/// | a file of exactly the size limit               | the document                 |
/// | a file larger than the size limit              | refused: too large           |
/// | a file that grows beyond the limit while read  | refused: too large           |
/// | a directory                                    | refused: not a file          |
/// | a device or a pipe, which may never end        | refused: not a file          |
/// | nothing, or a file that may not be read        | refused: not readable        |
/// | a file in another encoding (Latin-1, UTF-16)   | refused: not UTF-8           |
/// | a file whose name is not valid Unicode         | the document, the name shown |
/// |                                                | with replacement characters  |
///
/// Whether the content is an invoice is not decided here.
///
/// # Errors
///
/// Returns the [`Refusal`] that says why the file was not read.
pub fn load(path: &Path) -> Result<Document, Refusal> {
    let file = File::open(path).map_err(|_| Refusal::Unreadable)?;
    let metadata = file.metadata().map_err(|_| Refusal::Unreadable)?;
    if !metadata.is_file() {
        return Err(Refusal::NotAFile);
    }
    if metadata.len() > MAX_DOCUMENT_BYTES {
        return Err(too_large());
    }

    // One byte more than allowed is asked for, so that a file that grew after
    // its size was read is noticed.
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Refusal::Unreadable)?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > MAX_DOCUMENT_BYTES) {
        return Err(too_large());
    }

    let text = String::from_utf8(bytes).map_err(|_| Refusal::NotUtf8)?;
    Ok(Document {
        file_name: path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        text: text
            .strip_prefix(BYTE_ORDER_MARK)
            .map_or_else(|| text.clone(), str::to_owned),
    })
}

fn too_large() -> Refusal {
    Refusal::TooLarge {
        limit_bytes: MAX_DOCUMENT_BYTES,
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{Document, MAX_DOCUMENT_BYTES, load};
    use crate::checking::outcome::Refusal;

    const NAME: &str = "invoice.xml";
    const TEXT: &str = "<Invoice>ä</Invoice>";

    fn written(directory: &TempDir, content: &[u8]) -> io::Result<std::path::PathBuf> {
        let path = directory.path().join(NAME);
        fs::write(&path, content)?;
        Ok(path)
    }

    /// A file of the given size that occupies no space on the disk.
    fn sparse(path: &Path, bytes: u64) -> io::Result<()> {
        File::create(path)?.set_len(bytes)
    }

    #[test]
    fn a_file_in_utf8_is_read_with_its_name() -> io::Result<()> {
        let directory = TempDir::new()?;
        let path = written(&directory, TEXT.as_bytes())?;

        assert_eq!(
            load(&path),
            Ok(Document {
                file_name: NAME.to_owned(),
                text: TEXT.to_owned(),
            })
        );
        Ok(())
    }

    #[test]
    fn a_byte_order_mark_is_not_part_of_the_document() -> io::Result<()> {
        let directory = TempDir::new()?;
        let path = written(&directory, format!("\u{FEFF}{TEXT}").as_bytes())?;

        assert_eq!(
            load(&path).map(|document| document.text),
            Ok(TEXT.to_owned())
        );
        Ok(())
    }

    #[test]
    fn an_empty_file_is_a_document_without_text() -> io::Result<()> {
        let directory = TempDir::new()?;
        let path = written(&directory, b"")?;

        assert_eq!(load(&path).map(|document| document.text), Ok(String::new()));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_leads_to_its_file() -> io::Result<()> {
        let directory = TempDir::new()?;
        let path = written(&directory, TEXT.as_bytes())?;
        let link = directory.path().join("link.xml");
        std::os::unix::fs::symlink(&path, &link)?;

        assert_eq!(
            load(&link).map(|document| document.text),
            Ok(TEXT.to_owned())
        );
        Ok(())
    }

    #[test]
    fn the_size_limit_itself_is_allowed_and_one_byte_more_is_not() -> io::Result<()> {
        let directory = TempDir::new()?;
        let path = directory.path().join(NAME);

        sparse(&path, MAX_DOCUMENT_BYTES)?;
        assert!(load(&path).is_ok());

        sparse(&path, MAX_DOCUMENT_BYTES + 1)?;
        assert_eq!(
            load(&path),
            Err(Refusal::TooLarge {
                limit_bytes: MAX_DOCUMENT_BYTES
            })
        );
        Ok(())
    }

    #[test]
    fn a_directory_is_not_a_file() -> io::Result<()> {
        let directory = TempDir::new()?;

        assert_eq!(load(directory.path()), Err(Refusal::NotAFile));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn a_device_that_never_ends_is_not_a_file() {
        assert_eq!(load(Path::new("/dev/zero")), Err(Refusal::NotAFile));
    }

    #[test]
    fn what_does_not_exist_is_not_readable() -> io::Result<()> {
        let directory = TempDir::new()?;

        assert_eq!(
            load(&directory.path().join("missing.xml")),
            Err(Refusal::Unreadable)
        );
        Ok(())
    }

    #[test]
    fn another_encoding_is_refused() -> io::Result<()> {
        let directory = TempDir::new()?;
        // "ä" in Latin-1, and a text in UTF-16 with its byte order mark.
        let latin1 = written(&directory, b"<Invoice>\xE4</Invoice>")?;
        assert_eq!(load(&latin1), Err(Refusal::NotUtf8));

        let utf16 = written(&directory, b"\xFF\xFE<\x00I\x00")?;
        assert_eq!(load(&utf16), Err(Refusal::NotUtf8));
        Ok(())
    }
}
