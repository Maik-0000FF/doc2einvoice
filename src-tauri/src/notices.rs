// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The list of the libraries this build contains, with the licence texts they
//! ask to be passed on.
//!
//! The list is generated when a package is built and embedded by the build
//! script. It is carried to the interface as it was written: its form is
//! defined by its generator, and the interface reads it as the type derived
//! there. A build that is no package carries no list.

/// The list as the build script handed it over; empty where none was
/// generated.
const EMBEDDED: &str = include_str!(env!("LIBRARY_NOTICES_FILE"));

/// The list of the contained libraries as JSON, if this build carries one.
#[must_use]
pub fn embedded() -> Option<&'static str> {
    (!EMBEDDED.is_empty()).then_some(EMBEDDED)
}

#[cfg(test)]
mod tests {
    use super::embedded;

    /// What the interface reads from the list.
    const READ_FIELDS: [&str; 2] = ["texts", "libraries"];

    /// A build made for the tests carries a list only where one was generated
    /// for this target before. Where it does, the list is what the interface
    /// expects.
    #[test]
    fn a_carried_list_holds_what_the_interface_reads() -> Result<(), serde_json::Error> {
        let Some(list) = embedded() else {
            return Ok(());
        };
        let list: serde_json::Value = serde_json::from_str(list)?;

        for field in READ_FIELDS {
            assert!(
                list.get(field)
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|entries| !entries.is_empty()),
                "{field}"
            );
        }
        Ok(())
    }
}
