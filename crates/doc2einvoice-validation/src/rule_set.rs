// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The published rule sets, embedded unchanged, with their provenance.
//!
//! This module is the one place where a rule set's version, publication date,
//! licence, source and checksums are stated. Adding a rule set means adding a
//! variant to [`RuleSet`] and its entry here.

use doc2einvoice_schematron::{IncludedFiles, Schema};

use crate::embedded::{self, EmbeddedFile};
use crate::error::RuleSetError;
use crate::syntax::Syntax;

const EN16931_TITLE: &str = "EN 16931 validation artefacts";
const EN16931_VERSION: &str = "1.3.17";
const EN16931_PUBLISHED: &str = "2026-10-12";
const EN16931_LICENSE: &str = "EUPL-1.2";
const EN16931_SOURCE: &str = "https://github.com/ConnectingEurope/eInvoicing-EN16931";

const XRECHNUNG_TITLE: &str = "XRechnung 3.0.2 Schematron rules";
const XRECHNUNG_VERSION: &str = "2.6.0";
const XRECHNUNG_PUBLISHED: &str = "2026-09-02";
const XRECHNUNG_LICENSE: &str = "Apache-2.0";
const XRECHNUNG_SOURCE: &str = "https://github.com/itplr-kosit/xrechnung-schematron";

/// The `href` by which the XRechnung rule files include their shared file.
const XRECHNUNG_COMMON_HREF: &str = "../common.sch";

/// Embeds a file of the `rulesets` directory at the repository root.
macro_rules! embedded {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rulesets/",
            $path
        ))
    };
}

static EN16931_UBL_FILE: EmbeddedFile = EmbeddedFile {
    name: "EN16931-UBL-validation-preprocessed.sch",
    sha256: "eb27995ba7bace330475a0afd54a21bb0ebb050d55774949d5e10f65ecef7b95",
    text: embedded!("en16931/EN16931-UBL-validation-preprocessed.sch"),
};
static EN16931_CII_FILE: EmbeddedFile = EmbeddedFile {
    name: "EN16931-CII-validation-preprocessed.sch",
    sha256: "74fa94df0513881b9c8e369650bb2282f9c8a30bb52ce4ce43d1219e88ccedc2",
    text: embedded!("en16931/EN16931-CII-validation-preprocessed.sch"),
};
static XRECHNUNG_UBL_FILE: EmbeddedFile = EmbeddedFile {
    name: "XRechnung-UBL-validation.sch",
    sha256: "e143facbe9e6a39cc89d68de0576e81806f21a64830b102250c706a524f35ee6",
    text: embedded!("xrechnung/ubl/XRechnung-UBL-validation.sch"),
};
static XRECHNUNG_CII_FILE: EmbeddedFile = EmbeddedFile {
    name: "XRechnung-CII-validation.sch",
    sha256: "53bb5c0664fc603bbaaddeed26eb59525a05d3359d1a63f148876b2f2eedde1d",
    text: embedded!("xrechnung/cii/XRechnung-CII-validation.sch"),
};
static XRECHNUNG_COMMON_FILE: EmbeddedFile = EmbeddedFile {
    name: "common.sch",
    sha256: "7443d2f4c12d3e1ffa6b38f7f83cea89a9b03343f31f726083018d8985492a12",
    text: embedded!("xrechnung/common.sch"),
};

static EN16931_UBL: RuleSetInfo = RuleSetInfo {
    key: "en16931-ubl",
    title: EN16931_TITLE,
    version: EN16931_VERSION,
    published: EN16931_PUBLISHED,
    license: EN16931_LICENSE,
    source: EN16931_SOURCE,
    syntax: Syntax::Ubl,
    rules: &EN16931_UBL_FILE,
    includes: &[],
};
static EN16931_CII: RuleSetInfo = RuleSetInfo {
    key: "en16931-cii",
    title: EN16931_TITLE,
    version: EN16931_VERSION,
    published: EN16931_PUBLISHED,
    license: EN16931_LICENSE,
    source: EN16931_SOURCE,
    syntax: Syntax::Cii,
    rules: &EN16931_CII_FILE,
    includes: &[],
};
static XRECHNUNG_UBL: RuleSetInfo = RuleSetInfo {
    key: "xrechnung-ubl",
    title: XRECHNUNG_TITLE,
    version: XRECHNUNG_VERSION,
    published: XRECHNUNG_PUBLISHED,
    license: XRECHNUNG_LICENSE,
    source: XRECHNUNG_SOURCE,
    syntax: Syntax::Ubl,
    rules: &XRECHNUNG_UBL_FILE,
    includes: &[(XRECHNUNG_COMMON_HREF, &XRECHNUNG_COMMON_FILE)],
};
static XRECHNUNG_CII: RuleSetInfo = RuleSetInfo {
    key: "xrechnung-cii",
    title: XRECHNUNG_TITLE,
    version: XRECHNUNG_VERSION,
    published: XRECHNUNG_PUBLISHED,
    license: XRECHNUNG_LICENSE,
    source: XRECHNUNG_SOURCE,
    syntax: Syntax::Cii,
    rules: &XRECHNUNG_CII_FILE,
    includes: &[(XRECHNUNG_COMMON_HREF, &XRECHNUNG_COMMON_FILE)],
};

/// One published rule file for one syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleSet {
    /// The rules of EN 16931 for UBL.
    En16931Ubl,
    /// The rules of EN 16931 for CII.
    En16931Cii,
    /// The additional rules of the German XRechnung for UBL.
    XRechnungUbl,
    /// The additional rules of the German XRechnung for CII.
    XRechnungCii,
}

impl RuleSet {
    /// Every rule set this crate embeds.
    pub const ALL: [Self; 4] = [
        Self::En16931Ubl,
        Self::En16931Cii,
        Self::XRechnungUbl,
        Self::XRechnungCii,
    ];

    /// What the rule set is and where it comes from.
    #[must_use]
    pub fn info(self) -> &'static RuleSetInfo {
        match self {
            Self::En16931Ubl => &EN16931_UBL,
            Self::En16931Cii => &EN16931_CII,
            Self::XRechnungUbl => &XRECHNUNG_UBL,
            Self::XRechnungCii => &XRECHNUNG_CII,
        }
    }

    /// Compiles the rule set for validating documents.
    ///
    /// The checksum of every embedded file is verified first.
    ///
    /// # Errors
    ///
    /// Returns [`RuleSetError::Integrity`] if an embedded file is not the file
    /// its publisher released, and [`RuleSetError::Schema`] if the rule file
    /// cannot be compiled.
    pub fn compile(self) -> Result<Schema, RuleSetError> {
        let info = self.info();
        info.verify_integrity()?;

        let includes: Vec<(&str, &str)> = info
            .includes
            .iter()
            .map(|(href, file)| (*href, file.text))
            .collect();
        Ok(Schema::compile(info.rules.text, &IncludedFiles(&includes))?)
    }
}

/// What a rule set is and where it comes from.
#[derive(Debug)]
pub struct RuleSetInfo {
    /// A stable identifier, for example `en16931-ubl`.
    pub key: &'static str,
    /// The name of the rule set as its publisher calls it.
    pub title: &'static str,
    /// The version the publisher gave this release.
    pub version: &'static str,
    /// The date of the release, as `YYYY-MM-DD`.
    pub published: &'static str,
    /// The SPDX identifier of the licence the rule files are published under.
    pub license: &'static str,
    /// Where the publisher releases the rule set.
    pub source: &'static str,
    /// The syntax the rule set validates.
    pub syntax: Syntax,
    rules: &'static EmbeddedFile,
    /// The files the rule file includes, by the `href` it uses for them.
    includes: &'static [(&'static str, &'static EmbeddedFile)],
}

impl RuleSetInfo {
    /// The embedded files of the rule set: the rule file first, then the files
    /// it includes.
    pub fn files(&self) -> impl Iterator<Item = &'static EmbeddedFile> {
        std::iter::once(self.rules).chain(self.includes.iter().map(|(_, file)| *file))
    }

    fn verify_integrity(&self) -> Result<(), RuleSetError> {
        match embedded::first_altered(self.files()) {
            Some(file) => Err(RuleSetError::Integrity { file: file.name }),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuleSet;
    use crate::error::RuleSetError;

    #[test]
    fn every_embedded_file_is_the_published_file() {
        for rule_set in RuleSet::ALL {
            for file in rule_set.info().files() {
                assert!(file.is_unchanged(), "`{}` was altered", file.name);
            }
        }
    }

    #[test]
    fn every_rule_set_compiles() -> Result<(), RuleSetError> {
        for rule_set in RuleSet::ALL {
            let schema = rule_set.compile()?;

            assert!(
                schema.assertion_ids().count() > 0,
                "{rule_set:?} has no assertions"
            );
        }
        Ok(())
    }

    #[test]
    fn the_keys_are_unique() {
        let mut keys: Vec<&str> = RuleSet::ALL
            .iter()
            .map(|rule_set| rule_set.info().key)
            .collect();
        keys.sort_unstable();
        keys.dedup();

        assert_eq!(keys.len(), RuleSet::ALL.len());
    }
}
