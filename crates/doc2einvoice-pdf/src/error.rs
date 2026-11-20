// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Errors of reading a PDF.

/// Why a PDF could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PdfError {
    /// The data is not a PDF, or it is damaged beyond reading.
    #[error("the data is not a readable PDF")]
    Invalid,

    /// The PDF is encrypted and could not be opened: it needs a password, or
    /// its encryption is damaged or of a kind the interpreter does not know.
    /// A PDF that is encrypted only to restrict what may be done with it
    /// opens without a password and is read.
    #[error("the PDF is encrypted and cannot be opened")]
    Encrypted,

    /// The data is larger than the reader accepts.
    #[error("the PDF is larger than {limit} bytes")]
    TooLarge {
        /// The largest size that is read, in bytes.
        limit: usize,
    },

    /// The PDF has more pages than the reader accepts.
    #[error("the PDF has {pages} pages, more than {limit}")]
    TooManyPages {
        /// Number of pages of the PDF.
        pages: usize,
        /// The largest number of pages that is read.
        limit: usize,
    },

    /// The pages show more characters than the reader accepts.
    #[error("the PDF shows more than {limit} characters")]
    TooMuchText {
        /// The largest number of characters that is read.
        limit: usize,
    },

    /// The PDF interpreter failed in a way it could not report as an error.
    #[error("the PDF interpreter failed")]
    ReaderFailure,
}
