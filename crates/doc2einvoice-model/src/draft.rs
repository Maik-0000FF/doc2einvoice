// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! An invoice as it is being put together: values found on a page, given by
//! a model or typed by the user, each with where it came from, before the
//! invoice is complete.
//!
//! A draft holds text, not typed values: turning text into an amount or a
//! date is a step of its own that can fail. The text is written in one form
//! per data type, whatever the page printed, so that every later step reads
//! it the same way:
//!
//! | Data type                                  | Form          | Page may print |
//! |--------------------------------------------|---------------|----------------|
//! | date                                       | `2026-11-21`  | 21.11.2026     |
//! | amount, unit price, quantity, percentage   | `1234.5`      | 1.234,50 €     |
//! | code, identifier, reference, text          | as printed    |                |
//!
//! Nothing in a draft is required; what an invoice requires is checked when
//! it is made from the draft.

/// Where a value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Found on the page by a rule of the extraction.
    Rule,
    /// Proposed by a language model.
    Model,
    /// Typed or confirmed by the user.
    User,
}

/// Where on a page a value was found, in points from the top left corner of
/// the page as it is shown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    /// The page, counted from zero.
    pub page: usize,
    /// The left edge.
    pub left: f64,
    /// The top edge.
    pub top: f64,
    /// The right edge.
    pub right: f64,
    /// The bottom edge.
    pub bottom: f64,
}

/// One occurrence of a repeated business group, such as the second invoice
/// line: the number of the group and the place of the occurrence, counted
/// from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    /// The number of the business group, 25 for BG-25.
    pub group: u16,
    /// The place of this occurrence among those of the group.
    pub index: usize,
}

/// One value of a draft.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The number of the business term, 27 for BT-27.
    pub term: u16,
    /// The occurrences of repeated groups the value stands in, outermost
    /// first: none for a term of the invoice itself, the line for a term of
    /// an invoice line.
    pub within: Vec<Occurrence>,
    /// The value as text.
    pub text: String,
    /// Where it came from.
    pub origin: Origin,
    /// Where it was found on a page, if it was.
    pub location: Option<Location>,
}

/// An invoice being put together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Draft {
    /// The values, in the order they were found or given.
    pub entries: Vec<Entry>,
}

impl Draft {
    /// The values of the business term `term`, in every occurrence.
    pub fn values_of(&self, term: u16) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(move |entry| entry.term == term)
    }
}

#[cfg(test)]
mod tests {
    use super::{Draft, Entry, Occurrence, Origin};

    /// The business group of an invoice line.
    const LINE: u16 = 25;

    fn entry(term: u16, within: Vec<Occurrence>, text: &str) -> Entry {
        Entry {
            term,
            within,
            text: text.to_owned(),
            origin: Origin::Rule,
            location: None,
        }
    }

    #[test]
    fn the_values_of_a_term_are_found_in_every_occurrence() {
        let draft = Draft {
            entries: vec![
                entry(1, Vec::new(), "RE-1"),
                entry(
                    153,
                    vec![Occurrence {
                        group: LINE,
                        index: 0,
                    }],
                    "Apfel",
                ),
                entry(
                    153,
                    vec![Occurrence {
                        group: LINE,
                        index: 1,
                    }],
                    "Birne",
                ),
            ],
        };
        let names: Vec<&str> = draft
            .values_of(153)
            .map(|entry| entry.text.as_str())
            .collect();
        assert_eq!(names, ["Apfel", "Birne"]);
        assert_eq!(draft.values_of(2).count(), 0);
    }
}
