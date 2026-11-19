// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Program identity, the data of the legal notice, and the published files the
//! validation works with.
//!
//! Nothing is stated here a second time: identity and licence are read from
//! the crate manifest at compile time, and the rule sets and schemas describe
//! themselves in the validation crate.

use doc2einvoice_validation::{RuleSet, Syntax, XmlSchemaInfo};
use serde::Serialize;
use ts_rs::TS;

/// What the program is, under which terms it is distributed and which
/// published files its validation uses.
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
    /// The rule sets the validation embeds, each named once.
    pub rule_sets: Vec<Publication>,
    /// The XML Schemas the validation embeds.
    pub xml_schemas: Vec<Publication>,
}

/// A file set of another publisher that the validation uses unchanged. A
/// statement about conformance refers to exactly these releases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Publication {
    /// The name its publisher gives it.
    pub title: String,
    /// The version of the release, if its publisher numbers releases.
    pub version: Option<String>,
    /// The date of the release, as `YYYY-MM-DD`.
    pub published: String,
    /// SPDX identifier of its licence, if it is published under one.
    pub license: Option<String>,
    /// Where its publisher releases it.
    pub source: String,
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
            rule_sets: rule_sets(),
            xml_schemas: xml_schemas(),
        }
    }
}

/// The embedded rule sets. A release that serves several syntaxes is one
/// publication and is named once.
fn rule_sets() -> Vec<Publication> {
    let mut publications = Vec::new();
    for rule_set in RuleSet::ALL {
        let info = rule_set.info();
        let publication = Publication {
            title: info.title.to_owned(),
            version: Some(info.version.to_owned()),
            published: info.published.to_owned(),
            license: Some(info.license.to_owned()),
            source: info.source.to_owned(),
        };
        if !publications.contains(&publication) {
            publications.push(publication);
        }
    }
    publications
}

/// The XML Schema of every syntax. Their publishers date the releases and state
/// the terms in a notice of their own, not as a version and a licence identifier.
fn xml_schemas() -> Vec<Publication> {
    Syntax::ALL
        .into_iter()
        .map(XmlSchemaInfo::of)
        .map(|info| Publication {
            title: info.title.to_owned(),
            version: None,
            published: info.published.to_owned(),
            license: None,
            source: info.source.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use doc2einvoice_validation::{RuleSet, Syntax};

    use super::{AppInfo, Publication};

    /// Length of a date written as `YYYY-MM-DD`.
    const DATE_LENGTH: usize = 10;

    fn publications(info: &AppInfo) -> impl Iterator<Item = &Publication> {
        info.rule_sets.iter().chain(&info.xml_schemas)
    }

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
    fn every_embedded_rule_set_is_named_exactly_once() {
        let info = AppInfo::current();
        let embedded: HashSet<&str> = RuleSet::ALL
            .iter()
            .map(|rule_set| rule_set.info().title)
            .collect();
        let named: Vec<&str> = info
            .rule_sets
            .iter()
            .map(|publication| publication.title.as_str())
            .collect();

        assert_eq!(named.len(), embedded.len());
        assert_eq!(named.iter().copied().collect::<HashSet<_>>(), embedded);
    }

    #[test]
    fn the_schema_of_every_syntax_is_named() {
        assert_eq!(AppInfo::current().xml_schemas.len(), Syntax::ALL.len());
    }

    /// The user interface tells the rows of its table apart by their title.
    #[test]
    fn no_two_publications_share_a_title() {
        let info = AppInfo::current();
        let titles: HashSet<&str> = publications(&info)
            .map(|publication| publication.title.as_str())
            .collect();

        assert_eq!(titles.len(), publications(&info).count());
    }

    #[test]
    fn every_publication_states_title_date_and_source() {
        let info = AppInfo::current();

        assert!(!info.rule_sets.is_empty());
        assert!(!info.xml_schemas.is_empty());
        for publication in publications(&info) {
            assert!(!publication.title.trim().is_empty());
            assert!(!publication.source.trim().is_empty());
            assert_eq!(publication.published.len(), DATE_LENGTH);
        }
    }

    #[test]
    fn serializes_with_the_field_names_the_frontend_binding_uses() -> Result<(), serde_json::Error>
    {
        let json = serde_json::to_value(AppInfo::current())?;

        for field in ["copyrightHolder", "ruleSets", "xmlSchemas"] {
            assert!(json.get(field).is_some(), "{field}");
        }
        Ok(())
    }
}
