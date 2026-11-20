// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a reader delivers: the characters of a page with their places.

/// One character drawn on a page.
///
/// All lengths are in points and all places in the coordinates of the
/// displayed page (see the crate documentation).
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// The text the character stands for. Usually one character; a ligature
    /// stands for several.
    pub text: String,
    /// Horizontal place of the origin: where the character starts on its
    /// baseline.
    pub x: f64,
    /// Vertical place of the origin, which lies on the baseline.
    pub y: f64,
    /// How far the next character is moved along the baseline. Zero if the
    /// font does not state it.
    pub advance: f64,
    /// Size of the font as it appears on the page.
    pub font_size: f64,
    /// Angle of the baseline in radians. Zero is text running to the right.
    pub rotation: f64,
}

/// The characters of one page.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Width of the visible page.
    pub width: f64,
    /// Height of the visible page.
    pub height: f64,
    /// The characters in the order the file draws them, which need not be the
    /// order they are read in.
    pub glyphs: Vec<Glyph>,
    /// Characters that are drawn but could not be read: the file does not say
    /// which text they stand for, or their place is not a usable number.
    pub unreadable_glyphs: usize,
}
