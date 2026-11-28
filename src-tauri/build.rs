// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Build script: generates the Tauri context and capability schemas, and
//! hands the list of the contained libraries to the code that embeds it.
//!
//! The list is generated when a package is built (`cargo xtask notices`), one
//! file per target. A build of the program as it is packaged takes the one
//! of its target and fails without it: no package leaves without its list. A
//! build for development embeds none, whatever lies there from an earlier
//! package: the libraries may have changed since.

use std::error::Error;
use std::path::Path;
use std::{env, fs, io};

use location::Location;

// The generator states where the lists lie; the same code reads it here.
#[path = "../xtask/src/notices/location.rs"]
#[allow(
    dead_code,
    reason = "the generator uses the rest of what this module offers"
)]
mod location;

/// The variables Cargo names the directory of this package, the target and
/// the output directory in.
const PACKAGE_DIRECTORY_VARIABLE: &str = "CARGO_MANIFEST_DIR";
const TARGET_VARIABLE: &str = "TARGET";
const OUTPUT_DIRECTORY_VARIABLE: &str = "OUT_DIR";
/// The file the list is handed over as, in the output directory.
const EMBEDDED_FILE: &str = "library-notices.json";
/// The variable the code reads the place of that file from.
const EMBEDDED_VARIABLE: &str = "LIBRARY_NOTICES_FILE";

/// What a build embeds as the list of its libraries.
///
/// | The build                 | The list of its target         | Embedded        |
/// |---------------------------|--------------------------------|-----------------|
/// | for development           | not read                       | nothing         |
/// | of the packaged program   | holds a list                   | the list        |
/// | of the packaged program   | missing or unreadable          | the build fails |
/// | of the packaged program   | empty                          | the build fails |
fn embedded(development: bool, list: &Path) -> Result<String, String> {
    if development {
        return Ok(String::new());
    }
    let refused = |reason: &dyn std::fmt::Display| {
        format!(
            "{}: {reason}. A package carries the list of its libraries; generate it with \
             `cargo xtask notices`.",
            list.display()
        )
    };
    let text = fs::read_to_string(list).map_err(|error: io::Error| refused(&error))?;
    if text.trim().is_empty() {
        return Err(refused(&"the list is empty"));
    }
    Ok(text)
}

fn embed_library_notices() -> Result<(), Box<dyn Error>> {
    let location = Location::find(Path::new(&env::var(PACKAGE_DIRECTORY_VARIABLE)?))?;
    let list = location.list(&env::var(TARGET_VARIABLE)?);
    let development = tauri_build::is_dev();
    println!("cargo:rerun-if-changed={}", location.manifest().display());
    // A build for development does not read the list and need not be run
    // again when it changes.
    if !development {
        println!("cargo:rerun-if-changed={}", list.display());
    }

    let handed_over = Path::new(&env::var(OUTPUT_DIRECTORY_VARIABLE)?).join(EMBEDDED_FILE);
    fs::write(&handed_over, embedded(development, &list)?)?;
    println!(
        "cargo:rustc-env={EMBEDDED_VARIABLE}={}",
        handed_over.display()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    embed_library_notices()?;
    tauri_build::build();
    Ok(())
}
