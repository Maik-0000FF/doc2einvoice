// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Program identity and the data of the legal notice.
//!
//! Every value is read from the crate manifest at compile time, so the manifest
//! is the single place where name, version, author and licence are stated.

use serde::Serialize;
use ts_rs::TS;

/// What the program is and under which terms it is distributed.
///
/// The user interface shows this as its legal notice. The licence requires that
/// notice, including the copyright holder, to be preserved in modified versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppInfo {
    /// Program name.
    pub name: String,
    /// Program version (semantic versioning).
    pub version: String,
    /// Copyright holder named in the legal notice.
    pub copyright_holder: String,
    /// SPDX identifier of the licence.
    pub license: String,
    /// Where the corresponding source code is published.
    pub repository: String,
}

impl AppInfo {
    /// The identity of the running build.
    #[must_use]
    pub fn current() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            copyright_holder: env!("CARGO_PKG_AUTHORS").to_owned(),
            license: env!("CARGO_PKG_LICENSE").to_owned(),
            repository: env!("CARGO_PKG_REPOSITORY").to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppInfo;

    #[test]
    fn every_field_of_the_legal_notice_is_stated() {
        let info = AppInfo::current();

        for field in [
            &info.name,
            &info.version,
            &info.copyright_holder,
            &info.license,
            &info.repository,
        ] {
            assert!(!field.trim().is_empty());
        }
    }

    #[test]
    fn serializes_with_the_field_names_the_frontend_binding_uses() -> Result<(), serde_json::Error>
    {
        let json = serde_json::to_value(AppInfo::current())?;

        assert!(json.get("copyrightHolder").is_some());
        Ok(())
    }
}
