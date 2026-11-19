// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Validation against the XML Schema of a syntax.
//!
//! A specification requires an invoice to be valid against the XML Schema of
//! its syntax before its rules are applied. The schema files are embedded
//! unchanged, each with the checksum of its published form.
//!
//! The schemas of a syntax refer to each other by relative location. Those
//! references are resolved among the embedded files only: nothing is ever read
//! from the file system or fetched from a network, and a reference to a file
//! that is not embedded fails the compilation instead of being skipped.

use std::panic::{AssertUnwindSafe, catch_unwind};

use oxixml_schema::{ResolvedSchema, SchemaError, SchemaResolver, SchemaSet};

use crate::embedded::{self, EmbeddedFile};
use crate::error::XmlSchemaError;
use crate::report::SchemaFinding;
use crate::syntax::Syntax;

const PATH_SEPARATOR: char = '/';
const PARENT_DIRECTORY: &str = "..";
const CURRENT_DIRECTORY: &str = ".";
/// Error code the resolver reports for a reference it cannot satisfy.
const UNRESOLVED_REFERENCE_CODE: &str = "src-resolve";

/// Embeds a file of the `schemas` directory at the repository root.
macro_rules! embedded {
    ($path:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/",
            $path
        ))
    };
}

static UBL_FILES: [EmbeddedFile; 16] = [
    EmbeddedFile {
        name: "common/CCTS_CCT_SchemaModule-2.1.xsd",
        sha256: "dd546e4809df86b6445589f69f0d6c9df162840ae386574ddfc1da7638103e15",
        text: embedded!("ubl-2.1/common/CCTS_CCT_SchemaModule-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-CommonAggregateComponents-2.1.xsd",
        sha256: "580b5af6f68f7f556bd15945ba0e819cbf561442c81694f9b9468036ebedec4d",
        text: embedded!("ubl-2.1/common/UBL-CommonAggregateComponents-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-CommonBasicComponents-2.1.xsd",
        sha256: "a3b349bf92e5cff26e303d2f05b7e00c884dcafcfd9e12755fb80289abf5e22e",
        text: embedded!("ubl-2.1/common/UBL-CommonBasicComponents-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-CommonExtensionComponents-2.1.xsd",
        sha256: "ad7a4e490978adfbcfc5ec0bb20941cf11ac960ccf0c4de8791a7c731a8dbe87",
        text: embedded!("ubl-2.1/common/UBL-CommonExtensionComponents-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-CommonSignatureComponents-2.1.xsd",
        sha256: "4fa9e2370100040fe14c43e135ef77e2eb66b21cb8dbfc2ffb8d82ae991fe92e",
        text: embedded!("ubl-2.1/common/UBL-CommonSignatureComponents-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-CoreComponentParameters-2.1.xsd",
        sha256: "8be3379dbdcbcc7802fafdd16bac72c48fff1c0bb364213a31a17911dd06100b",
        text: embedded!("ubl-2.1/common/UBL-CoreComponentParameters-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-ExtensionContentDataType-2.1.xsd",
        sha256: "fcee77a11870208e6377ea6311b9f2a050bca24bdad8606ea02d71e9f9e72f8d",
        text: embedded!("ubl-2.1/common/UBL-ExtensionContentDataType-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-QualifiedDataTypes-2.1.xsd",
        sha256: "7dcb156e610239c97ae70940cf4653b88e48c3595bf5f56a2204a32e2893e6cf",
        text: embedded!("ubl-2.1/common/UBL-QualifiedDataTypes-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-SignatureAggregateComponents-2.1.xsd",
        sha256: "9234c2ca48dbfa9a22a786112bb075c5922a305170920eaab1e3c04fa0b7344b",
        text: embedded!("ubl-2.1/common/UBL-SignatureAggregateComponents-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-SignatureBasicComponents-2.1.xsd",
        sha256: "0fbe2d7afff0c1e11164b8ec83e13f18801021c3c87e390a9d76f9cf862f6a64",
        text: embedded!("ubl-2.1/common/UBL-SignatureBasicComponents-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-UnqualifiedDataTypes-2.1.xsd",
        sha256: "09052d406b4293e2a5f9c2bfee6df10ad4d8d5f0b36e24a6349d7f7936d89eb6",
        text: embedded!("ubl-2.1/common/UBL-UnqualifiedDataTypes-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-XAdESv132-2.1.xsd",
        sha256: "a4f726bcf8cc3f7d9ffa4dab99e005535a8e8b60dced1e5d94578d2e05afa96e",
        text: embedded!("ubl-2.1/common/UBL-XAdESv132-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-XAdESv141-2.1.xsd",
        sha256: "1fa4625e9cefcb7a9abb5ac1b64315547450031eece8a55bd584e4ba4b79dbc1",
        text: embedded!("ubl-2.1/common/UBL-XAdESv141-2.1.xsd"),
    },
    EmbeddedFile {
        name: "common/UBL-xmldsig-core-schema-2.1.xsd",
        sha256: "101909c9f06456d61ddcc4fb982f1d40dc357b439f393b1a2eb46e42acd60809",
        text: embedded!("ubl-2.1/common/UBL-xmldsig-core-schema-2.1.xsd"),
    },
    EmbeddedFile {
        name: "maindoc/UBL-CreditNote-2.1.xsd",
        sha256: "1e117b6c1ab713604b29b0d685442a81a0c78a82575445ee8c43e6050cda7454",
        text: embedded!("ubl-2.1/maindoc/UBL-CreditNote-2.1.xsd"),
    },
    EmbeddedFile {
        name: "maindoc/UBL-Invoice-2.1.xsd",
        sha256: "3a5aacd823f0e5b8f25ae7b5191c2002d5333ba351d87de9371ed62dca2b2b0c",
        text: embedded!("ubl-2.1/maindoc/UBL-Invoice-2.1.xsd"),
    },
];

static CII_FILES: [EmbeddedFile; 4] = [
    EmbeddedFile {
        name: "CrossIndustryInvoice_100pD16B.xsd",
        sha256: "3baf143b90f289d62c5dfb086aa51fa183490a2d0b38a40c6b31dcae0760f2ba",
        text: embedded!("cii-d16b/CrossIndustryInvoice_100pD16B.xsd"),
    },
    EmbeddedFile {
        name: "CrossIndustryInvoice_QualifiedDataType_100pD16B.xsd",
        sha256: "6649215914728652e883662bec1d3eab008c215e86a59f3bb417db9723ee8c5c",
        text: embedded!("cii-d16b/CrossIndustryInvoice_QualifiedDataType_100pD16B.xsd"),
    },
    EmbeddedFile {
        name: "CrossIndustryInvoice_ReusableAggregateBusinessInformationEntity_100pD16B.xsd",
        sha256: "1c42244298a863bcc0b5196983ff0a5d084fcabe58aea77ef82b242e7cd0ddfd",
        text: embedded!(
            "cii-d16b/CrossIndustryInvoice_ReusableAggregateBusinessInformationEntity_100pD16B.xsd"
        ),
    },
    EmbeddedFile {
        name: "CrossIndustryInvoice_UnqualifiedDataType_100pD16B.xsd",
        sha256: "04fce6f6f28521886731294be18a4e6bb5cb3003456457d99ad5c4d3e652f497",
        text: embedded!("cii-d16b/CrossIndustryInvoice_UnqualifiedDataType_100pD16B.xsd"),
    },
];

/// The distribution the Cross Industry Invoice schema files are taken from:
/// the configuration of the official XRechnung validator, which bundles the
/// files as UN/CEFACT published them.
const SCHEMA_DISTRIBUTION: &str =
    "https://github.com/itplr-kosit/validator-configuration-xrechnung";

static UBL: XmlSchemaInfo = XmlSchemaInfo {
    key: "ubl-2.1",
    title: "OASIS Universal Business Language 2.1",
    published: "2013-11-04",
    source: "http://docs.oasis-open.org/ubl/os-UBL-2.1/",
    syntax: Syntax::Ubl,
    files: &UBL_FILES,
    documents: &[
        "maindoc/UBL-Invoice-2.1.xsd",
        "maindoc/UBL-CreditNote-2.1.xsd",
    ],
};

static CII: XmlSchemaInfo = XmlSchemaInfo {
    key: "cii-d16b",
    title: "UN/CEFACT Cross Industry Invoice D16B",
    published: "2016-10-10",
    source: SCHEMA_DISTRIBUTION,
    syntax: Syntax::Cii,
    files: &CII_FILES,
    documents: &["CrossIndustryInvoice_100pD16B.xsd"],
};

/// The XML Schema of a syntax and where it comes from.
#[derive(Debug)]
pub struct XmlSchemaInfo {
    /// A stable identifier, for example `ubl-2.1`.
    pub key: &'static str,
    /// The name of the schema as its publisher calls it.
    pub title: &'static str,
    /// The date of the release, as `YYYY-MM-DD`.
    pub published: &'static str,
    /// Where the schema files can be obtained.
    pub source: &'static str,
    /// The syntax the schema describes.
    pub syntax: Syntax,
    files: &'static [EmbeddedFile],
    /// The files that declare the document elements; the others are reached
    /// through the references between the files.
    documents: &'static [&'static str],
}

impl XmlSchemaInfo {
    /// The XML Schema of the given syntax.
    #[must_use]
    pub fn of(syntax: Syntax) -> &'static Self {
        match syntax {
            Syntax::Ubl => &UBL,
            Syntax::Cii => &CII,
        }
    }

    /// The embedded files of the schema.
    pub fn files(&self) -> impl Iterator<Item = &'static EmbeddedFile> {
        self.files.iter()
    }

    /// Compiles the schema for validating documents.
    ///
    /// The checksum of every embedded file is verified first.
    ///
    /// # Errors
    ///
    /// Returns [`XmlSchemaError::Integrity`] if an embedded file is not the
    /// file its publisher released, and [`XmlSchemaError::Compile`] if the
    /// schema cannot be assembled from the embedded files.
    pub fn compile(&'static self) -> Result<XmlSchema, XmlSchemaError> {
        if let Some(file) = embedded::first_altered(self.files()) {
            return Err(XmlSchemaError::Integrity { file: file.name });
        }
        contained(|| self.assemble())?
    }

    fn assemble(&'static self) -> Result<XmlSchema, XmlSchemaError> {
        let mut set =
            SchemaSet::new().with_resolver(Box::new(EmbeddedResolver { files: self.files }));
        for name in self.documents {
            let file = find(self.files, name).ok_or_else(|| XmlSchemaError::Compile {
                reason: format!("the schema document `{name}` is not embedded"),
            })?;
            set.add_document(Some(file.name), file.text)
                .map_err(|error| compile_error(&error))?;
        }
        Ok(XmlSchema {
            compiled: set.compile().map_err(|error| compile_error(&error))?,
        })
    }
}

fn compile_error(error: &SchemaError) -> XmlSchemaError {
    XmlSchemaError::Compile {
        reason: error.to_string(),
    }
}

fn find(files: &'static [EmbeddedFile], name: &str) -> Option<&'static EmbeddedFile> {
    files.iter().find(|file| file.name == name)
}

/// A compiled XML Schema, ready to validate documents.
///
/// A compiled schema is bound to the thread it was compiled on.
pub struct XmlSchema {
    compiled: oxixml_schema::Schema,
}

impl XmlSchema {
    /// Validates a document. Returns what makes it invalid; empty if the
    /// document is valid against the schema.
    ///
    /// # Errors
    ///
    /// Returns [`XmlSchemaError::ProcessorFailure`] if the schema processor
    /// fails in a way it cannot report. The document is then neither valid nor
    /// invalid: no verdict was reached.
    pub fn validate(&self, document: &str) -> Result<Vec<SchemaFinding>, XmlSchemaError> {
        contained(|| {
            self.compiled
                .validate_str(document)
                .errors
                .iter()
                .map(|error| SchemaFinding {
                    code: error.code().to_owned(),
                    message: error.message().to_owned(),
                    location: error.path().to_owned(),
                })
                .collect()
        })
    }
}

/// Runs the schema processor and turns a panic of it into an error. A fault of
/// the processor on some input must end this validation, not the application.
fn contained<T>(work: impl FnOnce() -> T) -> Result<T, XmlSchemaError> {
    catch_unwind(AssertUnwindSafe(work)).map_err(|_| XmlSchemaError::ProcessorFailure)
}

/// Resolves the references between schema files among the embedded files.
#[derive(Debug)]
struct EmbeddedResolver {
    files: &'static [EmbeddedFile],
}

impl SchemaResolver for EmbeddedResolver {
    fn resolve(
        &mut self,
        location: &str,
        base: Option<&str>,
    ) -> Result<Option<ResolvedSchema>, SchemaError> {
        let name = join(base.unwrap_or_default(), location);
        // A reference that cannot be satisfied is an error, not something to
        // skip: a schema assembled without one of its parts accepts documents
        // it must refuse.
        let file = name
            .as_deref()
            .and_then(|name| find(self.files, name))
            .ok_or_else(|| {
                SchemaError::new(
                    UNRESOLVED_REFERENCE_CODE,
                    format!("the schema file `{location}` is not embedded"),
                )
            })?;
        Ok(Some(ResolvedSchema {
            uri: file.name.to_owned(),
            text: file.text.to_owned(),
        }))
    }
}

/// Resolves a relative location against the file that refers to it, both as
/// paths with `/`. `None` if the location leads out of the embedded files.
///
/// | base                | location             | result                |
/// |---------------------|----------------------|-----------------------|
/// | `maindoc/a.xsd`     | `../common/b.xsd`    | `common/b.xsd`        |
/// | `common/a.xsd`      | `b.xsd`              | `common/b.xsd`        |
/// | `a.xsd`             | `./b.xsd`            | `b.xsd`               |
/// | (none)              | `b.xsd`              | `b.xsd`               |
/// | `a.xsd`             | `../b.xsd`           | none: above the root  |
/// | `a.xsd`             | `/etc/b.xsd`         | none: absolute        |
/// | `a.xsd`             | `http://host/b.xsd`  | `http:/host/b.xsd`: a name no embedded file has |
fn join(base: &str, location: &str) -> Option<String> {
    if location.starts_with(PATH_SEPARATOR) {
        return None;
    }
    let mut segments: Vec<&str> = base.split(PATH_SEPARATOR).collect();
    // The last segment of the base is the file that refers, not a directory.
    segments.pop();
    for segment in location.split(PATH_SEPARATOR) {
        match segment {
            CURRENT_DIRECTORY | "" => {}
            PARENT_DIRECTORY => {
                segments.pop()?;
            }
            name => segments.push(name),
        }
    }
    Some(segments.join(&PATH_SEPARATOR.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{XmlSchemaInfo, join};
    use crate::error::XmlSchemaError;
    use crate::syntax::Syntax;

    const SYNTAXES: [Syntax; 2] = [Syntax::Ubl, Syntax::Cii];

    #[test]
    fn locations_are_resolved_among_the_embedded_files_only() {
        assert_eq!(
            join("maindoc/a.xsd", "../common/b.xsd").as_deref(),
            Some("common/b.xsd")
        );
        assert_eq!(
            join("common/a.xsd", "b.xsd").as_deref(),
            Some("common/b.xsd")
        );
        assert_eq!(join("a.xsd", "./b.xsd").as_deref(), Some("b.xsd"));
        assert_eq!(join("", "b.xsd").as_deref(), Some("b.xsd"));
        assert_eq!(join("a.xsd", "../b.xsd"), None);
        assert_eq!(join("a.xsd", "/etc/b.xsd"), None);
        // Not refused by the join, but no embedded file has such a name.
        assert_eq!(
            join("a.xsd", "http://host/b.xsd").as_deref(),
            Some("http:/host/b.xsd")
        );
    }

    #[test]
    fn every_embedded_file_is_the_published_file() {
        for syntax in SYNTAXES {
            for file in XmlSchemaInfo::of(syntax).files() {
                assert!(file.is_unchanged(), "`{}` was altered", file.name);
            }
        }
    }

    #[test]
    fn every_schema_compiles_from_the_embedded_files_alone() -> Result<(), XmlSchemaError> {
        for syntax in SYNTAXES {
            XmlSchemaInfo::of(syntax).compile()?;
        }
        Ok(())
    }

    #[test]
    fn a_document_of_another_vocabulary_is_invalid() -> Result<(), XmlSchemaError> {
        for syntax in SYNTAXES {
            let schema = XmlSchemaInfo::of(syntax).compile()?;

            assert!(!schema.validate("<unknown/>")?.is_empty());
        }
        Ok(())
    }

    #[test]
    fn a_panic_of_the_processor_becomes_an_error() {
        let result: Result<(), XmlSchemaError> =
            super::contained(|| std::panic::resume_unwind(Box::new("processor fault")));

        assert!(matches!(result, Err(XmlSchemaError::ProcessorFailure)));
        assert!(matches!(super::contained(|| 7), Ok(7)));
    }
}
