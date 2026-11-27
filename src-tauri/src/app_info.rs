// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Program identity, the data of the legal notice, the published files the
//! validation works with, and the material of other authors the application
//! contains.
//!
//! Nothing is stated here a second time: identity and licence are read from
//! the crate manifest at compile time, and the rule sets and schemas describe
//! themselves in the validation crate.

use doc2einvoice_validation::{RuleSet, RuleSetInfo, Syntax, XmlSchemaInfo};
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
    /// The material of other authors the application contains, besides the
    /// rule sets and the schemas.
    pub materials: Vec<Material>,
}

/// Material of another author that the application contains, with what its
/// licence asks to be stated wherever it is passed on: who holds the
/// copyright, under which terms, and the terms themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Material {
    /// The name its author gives it.
    pub title: String,
    /// The copyright notice of its author, word for word.
    pub copyright: String,
    /// SPDX identifier of its licence.
    pub license: String,
    /// The licence, in full.
    pub license_text: String,
    /// Where its author publishes it.
    pub source: String,
    /// What was changed from the original, if anything.
    pub changes: Option<String>,
}

/// The typeface of the interface, contained unchanged.
const TYPEFACE_TITLE: &str = "Inter";
const TYPEFACE_COPYRIGHT: &str =
    "Copyright (c) 2016 The Inter Project Authors (https://github.com/rsms/inter)";
const TYPEFACE_LICENSE: &str = "OFL-1.1";
const TYPEFACE_LICENSE_TEXT: &str = include_str!("../../LICENSES/OFL-1.1.txt");
const TYPEFACE_SOURCE: &str = "https://rsms.me/inter/";

/// The material of other authors the application contains. What belongs to
/// the interface is stated here; a crate that holds material of its own
/// states it itself, and it is listed here once the application depends on
/// that crate.
fn materials() -> Vec<Material> {
    vec![Material {
        title: TYPEFACE_TITLE.to_owned(),
        copyright: TYPEFACE_COPYRIGHT.to_owned(),
        license: TYPEFACE_LICENSE.to_owned(),
        license_text: TYPEFACE_LICENSE_TEXT.to_owned(),
        source: TYPEFACE_SOURCE.to_owned(),
        changes: None,
    }]
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

impl From<&RuleSetInfo> for Publication {
    fn from(info: &RuleSetInfo) -> Self {
        Self {
            title: info.title.to_owned(),
            version: Some(info.version.to_owned()),
            published: info.published.to_owned(),
            license: Some(info.license.to_owned()),
            source: info.source.to_owned(),
        }
    }
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
            materials: materials(),
        }
    }
}

/// The embedded rule sets. A release that serves several syntaxes is one
/// publication and is named once.
fn rule_sets() -> Vec<Publication> {
    let mut publications = Vec::new();
    for rule_set in RuleSet::ALL {
        let publication = Publication::from(rule_set.info());
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

    use super::{AppInfo, Publication, TYPEFACE_COPYRIGHT, TYPEFACE_LICENSE};

    /// The statement of the terms of every file of the repository.
    const REPOSITORY_TERMS: &str = include_str!("../../REUSE.toml");
    /// What a copyright notice starts with, which the statement of the
    /// repository leaves out.
    const COPYRIGHT_MARK: &str = "Copyright (c) ";
    /// What starts an entry of that statement, and the files of the
    /// typeface, by which its entry is told from the others.
    const ENTRY_MARK: &str = "[[annotations]]";
    const TYPEFACE_FILES: &str = "src/lib/assets/fonts/inter/";
    /// The lines of an entry that state the holder and the licence.
    const HOLDER_LINE: &str = "SPDX-FileCopyrightText";
    const LICENSE_LINE: &str = "SPDX-License-Identifier";

    /// What the line of an entry that starts with `key` states, without its
    /// quotation marks.
    ///
    /// | Entry                                  | Key                       | Stated    |
    /// |----------------------------------------|---------------------------|-----------|
    /// | `SPDX-License-Identifier = "OFL-1.1"`  | `SPDX-License-Identifier` | `OFL-1.1` |
    /// | the same, after other lines            | `SPDX-License-Identifier` | `OFL-1.1` |
    /// | the same                               | `SPDX-FileCopyrightText`  | none      |
    /// | `SPDX-License-Identifier = OFL-1.1`    | `SPDX-License-Identifier` | none (not quoted) |
    /// | nothing                                | any                       | none      |
    fn stated<'a>(entry: &'a str, key: &str) -> Option<&'a str> {
        entry.lines().find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key)
                .then(|| value.trim().strip_prefix('"')?.strip_suffix('"'))
                .flatten()
        })
    }

    #[test]
    fn what_an_entry_states_follows_its_table() {
        let entry = "path = [\"a/**\"]\nSPDX-License-Identifier = \"OFL-1.1\"";
        let table = [
            (
                "SPDX-License-Identifier = \"OFL-1.1\"",
                LICENSE_LINE,
                Some("OFL-1.1"),
            ),
            (entry, LICENSE_LINE, Some("OFL-1.1")),
            (entry, HOLDER_LINE, None),
            ("SPDX-License-Identifier = OFL-1.1", LICENSE_LINE, None),
            ("", LICENSE_LINE, None),
        ];
        for (entry, key, expected) in table {
            assert_eq!(stated(entry, key), expected, "{entry}");
        }
    }

    #[test]
    fn every_material_states_what_its_licence_asks_for() {
        let info = AppInfo::current();

        assert!(!info.materials.is_empty());
        for material in &info.materials {
            for field in [
                &material.title,
                &material.copyright,
                &material.license,
                &material.license_text,
                &material.source,
            ] {
                assert!(!field.trim().is_empty(), "{}", material.title);
            }
        }
    }

    /// The repository states the terms of the typeface as well, in the entry
    /// of its files; the two must say the same.
    #[test]
    fn the_typeface_is_stated_as_the_repository_states_it() {
        let entries: Vec<&str> = REPOSITORY_TERMS
            .split(ENTRY_MARK)
            .filter(|entry| entry.contains(TYPEFACE_FILES))
            .collect();
        let [entry] = entries[..] else {
            unreachable!("not one entry of the typeface: {}", entries.len())
        };

        assert_eq!(
            stated(entry, HOLDER_LINE),
            TYPEFACE_COPYRIGHT.strip_prefix(COPYRIGHT_MARK)
        );
        assert_eq!(stated(entry, LICENSE_LINE), Some(TYPEFACE_LICENSE));
    }

    #[test]
    fn no_two_materials_share_a_title() {
        let info = AppInfo::current();
        let titles: HashSet<&str> = info
            .materials
            .iter()
            .map(|material| material.title.as_str())
            .collect();

        assert_eq!(titles.len(), info.materials.len());
    }

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

        for field in ["copyrightHolder", "ruleSets", "xmlSchemas", "materials"] {
            assert!(json.get(field).is_some(), "{field}");
        }
        Ok(())
    }
}
