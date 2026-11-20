// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The limits on what is read.
//!
//! Input table. Each row is one test below. The PDF has two pages of two
//! characters each.
//!
//! | limits                                           | expected                    |
//! |--------------------------------------------------|-----------------------------|
//! | exactly its bytes, pages and characters          | read                        |
//! | one byte less than it has                        | `TooLarge`                  |
//! | the same, for data that is no PDF                | `TooLarge`, not `Invalid`   |
//! | one page less than it has                        | `TooManyPages`              |
//! | one character less, the last on the second page  | `TooMuchText`               |
//! | one character less than a single page shows      | `TooMuchText`               |
//! | the limits `read` applies                        | read, and they are the      |
//! |                                                  | default                     |

mod support;

use doc2einvoice_pdf::{Limits, PdfError, read, read_within};
use support::{pdf_of, text_at};

const X: f64 = 50.0;
const Y: f64 = 700.0;
const PAGES: usize = 2;
const GLYPHS_PER_PAGE: usize = 2;
const GLYPHS: usize = PAGES * GLYPHS_PER_PAGE;

fn two_pages() -> Vec<u8> {
    pdf_of(&[&text_at(X, Y, "AB"), &text_at(X, Y, "CD")])
}

/// The limits the PDF of two pages meets exactly.
fn exact(data: &[u8]) -> Limits {
    Limits {
        bytes: data.len(),
        pages: PAGES,
        glyphs: GLYPHS,
    }
}

#[test]
fn a_pdf_at_the_limits_is_read() -> Result<(), PdfError> {
    let data = two_pages();
    let pages = read_within(&data, exact(&data))?;

    assert_eq!(pages.len(), PAGES);
    assert!(
        pages
            .iter()
            .all(|page| page.glyphs.len() == GLYPHS_PER_PAGE)
    );
    Ok(())
}

#[test]
fn one_byte_too_many_is_refused() {
    let data = two_pages();
    let limit = data.len() - 1;
    let limits = Limits {
        bytes: limit,
        ..exact(&data)
    };

    assert_eq!(
        read_within(&data, limits),
        Err(PdfError::TooLarge { limit })
    );
}

#[test]
fn the_size_is_judged_before_the_content() {
    let data = b"not a PDF";
    let limit = data.len() - 1;
    let limits = Limits {
        bytes: limit,
        ..Limits::DEFAULT
    };

    assert_eq!(read_within(data, limits), Err(PdfError::TooLarge { limit }));
}

#[test]
fn one_page_too_many_is_refused() {
    let data = two_pages();
    let limit = PAGES - 1;
    let limits = Limits {
        pages: limit,
        ..exact(&data)
    };

    assert_eq!(
        read_within(&data, limits),
        Err(PdfError::TooManyPages {
            pages: PAGES,
            limit
        })
    );
}

#[test]
fn one_character_too_many_over_all_pages_is_refused() {
    let data = two_pages();
    let limit = GLYPHS - 1;
    let limits = Limits {
        glyphs: limit,
        ..exact(&data)
    };

    assert_eq!(
        read_within(&data, limits),
        Err(PdfError::TooMuchText { limit })
    );
}

#[test]
fn one_character_too_many_on_a_page_is_refused() {
    let data = pdf_of(&[&text_at(X, Y, "AB")]);
    let limit = GLYPHS_PER_PAGE - 1;
    let limits = Limits {
        glyphs: limit,
        ..Limits::DEFAULT
    };

    assert_eq!(
        read_within(&data, limits),
        Err(PdfError::TooMuchText { limit })
    );
}

#[test]
fn read_applies_the_default_limits() -> Result<(), PdfError> {
    let data = two_pages();

    assert_eq!(Limits::default(), Limits::DEFAULT);
    assert_eq!(read(&data)?, read_within(&data, Limits::DEFAULT)?);
    Ok(())
}
