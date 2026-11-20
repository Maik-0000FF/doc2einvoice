// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading the characters of a PDF with the PDF interpreter.
//!
//! The interpreter runs the drawing instructions of each page as a renderer
//! would. Instead of painting, the device below notes every character with the
//! place it would be painted at.
//!
//! A PDF comes from outside and may be far larger than any invoice, by
//! accident or on purpose. [`Limits`] bound what is read. They bound the
//! memory a reading takes. They do not bound its time: the interpreter cannot
//! be stopped inside a page, so a single page that draws a vast number of
//! characters is run to its end before it is refused. That is a decision: the
//! limit on the size of the data keeps such a page within what a machine
//! works off in reasonable time.

use std::panic::{AssertUnwindSafe, catch_unwind};

use hayro_interpret::font::{Glyph as DrawnGlyph, GlyphRun};
use hayro_interpret::hayro_cmap::BfString;
use hayro_interpret::hayro_syntax::{LoadPdfError, Pdf};
use hayro_interpret::kurbo::{Affine, BezPath, Point, Rect};
use hayro_interpret::{
    BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageDrawProps,
    InterpreterCache, InterpreterSettings, SoftMask, interpret_page,
};

use crate::error::PdfError;
use crate::glyph::{Glyph, Page};

/// Units per em in the coordinates the interpreter places a character in.
const GLYPH_UNITS_PER_EM: f64 = 1000.0;

/// Half the side of the area the interpreter is told is visible. Characters
/// are noted wherever they stand, so the area only has to contain any page.
const VISIBLE_HALF_EXTENT: f64 = 1.0e6;

const BYTES_PER_MEBIBYTE: usize = 1024 * 1024;

/// How large a PDF may be to be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest size of the data, in bytes.
    pub bytes: usize,
    /// The largest number of pages.
    pub pages: usize,
    /// The largest number of characters on all pages together.
    pub glyphs: usize,
}

impl Limits {
    /// The limits [`read`] applies.
    ///
    /// An invoice is a few pages of a few thousand characters each and rarely
    /// reaches a mebibyte; an itemised bill runs to some hundred pages, and a
    /// scan can weigh tens of mebibytes. The limits leave room for all of
    /// these and keep the characters of one PDF within a few hundred
    /// mebibytes of memory.
    pub const DEFAULT: Self = Self {
        bytes: 64 * BYTES_PER_MEBIBYTE,
        pages: 1000,
        glyphs: 2_000_000,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Reads the characters of every page of a PDF, within [`Limits::DEFAULT`].
///
/// # Errors
///
/// Returns a [`PdfError`] as [`read_within`] states.
pub fn read(data: &[u8]) -> Result<Vec<Page>, PdfError> {
    read_within(data, Limits::DEFAULT)
}

/// Reads the characters of every page of a PDF that stays within `limits`.
///
/// | Data                                             | Result                      |
/// |--------------------------------------------------|-----------------------------|
/// | a PDF with text                                  | its pages with their glyphs |
/// | a PDF whose pages show no text (a scan, a blank) | its pages without glyphs    |
/// | exactly as many bytes, pages and characters as   | read                        |
/// | the limits allow                                 |                             |
/// | one byte more than the limit, PDF or not         | [`PdfError::TooLarge`]      |
/// | one page more than the limit                     | [`PdfError::TooManyPages`]  |
/// | one character more than the limit, on one page   | [`PdfError::TooMuchText`]   |
/// | or spread over several                           |                             |
/// | characters the file gives no text for            | not counted against the     |
/// |                                                  | limit: they are not kept    |
/// | no data at all, or data that is no PDF           | [`PdfError::Invalid`]       |
/// | an encrypted PDF that opens without a password   | read like any other         |
/// | an encrypted PDF that does not                   | [`PdfError::Encrypted`]     |
/// | a PDF on which the interpreter panics            | [`PdfError::ReaderFailure`] |
///
/// The limits are checked in the order of the table: the size before anything
/// is parsed, the pages before any is interpreted, the characters page by
/// page.
///
/// # Errors
///
/// Returns a [`PdfError`] as the table states.
pub fn read_within(data: &[u8], limits: Limits) -> Result<Vec<Page>, PdfError> {
    if data.len() > limits.bytes {
        return Err(PdfError::TooLarge {
            limit: limits.bytes,
        });
    }
    // A fault of the interpreter on some file must end this reading, not the
    // application.
    catch_unwind(AssertUnwindSafe(|| read_pages(data, limits)))
        .map_err(|_| PdfError::ReaderFailure)?
}

fn read_pages(data: &[u8], limits: Limits) -> Result<Vec<Page>, PdfError> {
    let pdf = Pdf::new(data.to_vec()).map_err(|error| match error {
        LoadPdfError::Decryption(_) => PdfError::Encrypted,
        LoadPdfError::Invalid => PdfError::Invalid,
    })?;
    let page_count = pdf.pages().len();
    if page_count > limits.pages {
        return Err(PdfError::TooManyPages {
            pages: page_count,
            limit: limits.pages,
        });
    }

    let cache = InterpreterCache::new();
    let visible = Rect::new(
        -VISIBLE_HALF_EXTENT,
        -VISIBLE_HALF_EXTENT,
        VISIBLE_HALF_EXTENT,
        VISIBLE_HALF_EXTENT,
    );
    let mut glyphs_left = limits.glyphs;
    let mut pages = Vec::with_capacity(page_count);
    for page in pdf.pages().iter() {
        let (width, height) = page.render_dimensions();
        let mut collector = Collector {
            to_display: Affine::new(page.initial_transform(true).as_coeffs()),
            glyphs: Vec::new(),
            glyphs_left,
            over_limit: false,
            unreadable: 0,
        };
        let mut context = Context::new(
            Affine::IDENTITY,
            visible,
            &cache,
            pdf.xref(),
            InterpreterSettings::default(),
        );
        interpret_page(page, &mut context, &mut collector);
        if collector.over_limit {
            return Err(PdfError::TooMuchText {
                limit: limits.glyphs,
            });
        }
        glyphs_left = collector.glyphs_left;
        pages.push(Page {
            width: f64::from(width),
            height: f64::from(height),
            glyphs: collector.glyphs,
            unreadable_glyphs: collector.unreadable,
        });
    }
    Ok(pages)
}

/// Notes the characters the interpreter draws.
struct Collector {
    /// From the coordinates of the file to those of the displayed page.
    to_display: Affine,
    glyphs: Vec<Glyph>,
    /// How many more characters may be kept.
    glyphs_left: usize,
    /// Whether a character was drawn after none could be kept any more.
    over_limit: bool,
    unreadable: usize,
}

impl Collector {
    /// Turns a drawn character into a [`Glyph`].
    ///
    /// | Character                                     | Result                    |
    /// |-----------------------------------------------|---------------------------|
    /// | has a text, a finite place and a size above 0 | kept                      |
    /// | its text is white space only                  | kept: the layout reads it |
    /// |                                               | as a gap                  |
    /// | the file gives no text for it                 | `None`                    |
    /// | its text is empty                             | `None`                    |
    /// | place or size is not a finite number          | `None`                    |
    /// | its size is zero                              | `None`                    |
    /// | the font states no advance, or not a finite   | kept with advance zero    |
    /// | one                                           |                           |
    fn glyph(&self, drawn: &DrawnGlyph<'_>, placement: Affine) -> Option<Glyph> {
        let text = match drawn.as_unicode()? {
            BfString::Char(character) => character.to_string(),
            BfString::String(text) => text,
        };
        let placement = self.to_display * placement;
        let origin = placement * Point::ZERO;
        let em = placement * Point::new(GLYPH_UNITS_PER_EM, 0.0) - origin;
        let font_size = em.hypot();
        let usable = !text.is_empty()
            && origin.x.is_finite()
            && origin.y.is_finite()
            && font_size.is_finite()
            && font_size > 0.0;
        if !usable {
            return None;
        }
        let advance = match drawn {
            DrawnGlyph::Outline(outline) => outline.advance_width(),
            DrawnGlyph::Type3(type3) => type3.advance_width(),
        }
        .map(|units| font_size * f64::from(units) / GLYPH_UNITS_PER_EM)
        .filter(|advance| advance.is_finite())
        .unwrap_or(0.0);

        Some(Glyph {
            text,
            x: origin.x,
            y: origin.y,
            advance,
            font_size,
            rotation: em.y.atan2(em.x),
        })
    }

    /// Keeps a character, unless the limit is reached.
    fn keep(&mut self, glyph: Glyph) {
        match self.glyphs_left.checked_sub(1) {
            Some(left) => {
                self.glyphs_left = left;
                self.glyphs.push(glyph);
            }
            None => self.over_limit = true,
        }
    }
}

impl Device<'_> for Collector {
    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, '_>, props: DrawProps<'_>, _: &DrawMode) {
        for drawn in run.glyphs() {
            match self.glyph(drawn, props.transform * drawn.transform()) {
                Some(glyph) => self.keep(glyph),
                None => self.unreadable += 1,
            }
        }
    }

    // Everything but text is of no interest to a reader.
    fn draw_path(&mut self, _: &BezPath, _: DrawProps<'_>, _: &DrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
    fn draw_image(&mut self, _: Image<'_, '_>, _: ImageDrawProps<'_>) {}
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}
