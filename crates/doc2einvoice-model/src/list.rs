// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! A list with at least one element, for the business groups of cardinality
//! 1..n.

/// A list that is never empty.
///
/// | Given          | Result            |
/// |----------------|-------------------|
/// | no element     | none              |
/// | one element    | a list of one     |
/// | three elements | a list of three, in the given order |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    /// A list of exactly one element.
    pub fn one(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// The elements as a list, or none if there are no elements.
    #[must_use]
    pub fn from_vec(elements: Vec<T>) -> Option<Self> {
        let mut elements = elements.into_iter();
        let first = elements.next()?;
        Some(Self {
            first,
            rest: elements.collect(),
        })
    }

    /// The first element.
    pub fn first(&self) -> &T {
        &self.first
    }

    /// How many elements there are; never zero.
    pub fn count(&self) -> usize {
        self.rest.len() + 1
    }

    /// The elements in order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::NonEmpty;

    #[test]
    fn no_element_makes_no_list() {
        assert_eq!(NonEmpty::<u8>::from_vec(Vec::new()), None);
    }

    #[test]
    fn the_elements_keep_their_order() {
        let list = NonEmpty::from_vec(vec![1, 2, 3]);
        assert_eq!(list.as_ref().map(NonEmpty::count), Some(3));
        assert_eq!(
            list.map(|list| list.iter().copied().collect::<Vec<_>>()),
            Some(vec![1, 2, 3])
        );
    }

    #[test]
    fn a_list_of_one_has_its_element_first() {
        let list = NonEmpty::one('a');
        assert_eq!((list.count(), *list.first()), (1, 'a'));
    }
}
