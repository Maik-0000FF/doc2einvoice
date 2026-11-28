// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Build script: generates the Tauri context and capability schemas, and
//! hands the list of the contained libraries to the code that embeds it.
//!
//! The list is generated when a package is built (`cargo xtask notices`), one
//! file per target. This script takes the one of the target it builds for. It
//! decides nothing: a build without a list embeds an empty one, and the
//! program says that it carries none.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

/// The variable that names the directory the lists are generated in. Cargo
/// sets it from its configuration in this repository.
const NOTICES_DIRECTORY_VARIABLE: &str = "NOTICES_DIR";
/// The variables Cargo names the target and the output directory in.
const TARGET_VARIABLE: &str = "TARGET";
const OUTPUT_DIRECTORY_VARIABLE: &str = "OUT_DIR";
/// The file the list is handed over as, in the output directory.
const EMBEDDED_FILE: &str = "library-notices.json";
/// The variable the code reads the place of that file from.
const EMBEDDED_VARIABLE: &str = "LIBRARY_NOTICES_FILE";

/// The list generated for a target, or an empty text where there is none.
///
/// | The file of the target        | Result            |
/// |-------------------------------|-------------------|
/// | holds a list                  | the list          |
/// | does not exist                | empty             |
/// | cannot be read                | the build fails   |
fn generated(list: &Path) -> io::Result<String> {
    match fs::read_to_string(list) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error),
    }
}

fn embed_library_notices() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed={NOTICES_DIRECTORY_VARIABLE}");
    let directory = PathBuf::from(env::var(NOTICES_DIRECTORY_VARIABLE).map_err(|_| {
        format!("{NOTICES_DIRECTORY_VARIABLE} is not set; build with Cargo inside the repository")
    })?);
    let list = directory.join(format!("{}.json", env::var(TARGET_VARIABLE)?));

    // The directory is watched, so that a list that appears or disappears is
    // noticed; it has to exist for that. The list itself is watched only
    // while it exists: a watched file that is missing would run this script
    // on every build.
    fs::create_dir_all(&directory)?;
    println!("cargo:rerun-if-changed={}", directory.display());
    if list.exists() {
        println!("cargo:rerun-if-changed={}", list.display());
    }

    let embedded = PathBuf::from(env::var(OUTPUT_DIRECTORY_VARIABLE)?).join(EMBEDDED_FILE);
    fs::write(&embedded, generated(&list)?)?;
    println!("cargo:rustc-env={EMBEDDED_VARIABLE}={}", embedded.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    embed_library_notices()?;
    tauri_build::build();
    Ok(())
}
