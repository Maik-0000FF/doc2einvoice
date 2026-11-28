// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The list of the libraries this build contains, with the licence texts they
//! ask to be passed on.
//!
//! The list is generated when a package is built and embedded by the build
//! script. It is carried to the interface as it was written: its form is
//! defined by its generator, and the interface reads it as the type derived
//! there. Nothing here knows what the list holds. A build for development
//! carries no list.

/// The list as the build script handed it over; empty where the build
/// carries none. The build script sets the variable named here and names it
/// as `EMBEDDED_VARIABLE`.
const EMBEDDED: &str = include_str!(env!("LIBRARY_NOTICES_FILE"));

/// The list of the contained libraries as JSON, if this build carries one.
#[must_use]
pub fn embedded() -> Option<&'static str> {
    (!EMBEDDED.is_empty()).then_some(EMBEDDED)
}

#[cfg(test)]
mod tests {
    use super::embedded;

    /// The tests are built for development.
    #[test]
    fn a_build_for_development_carries_no_list() {
        assert_eq!(embedded(), None);
    }
}
