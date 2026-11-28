// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Build script: hands the target the tools are compiled for to their code.
//! The tools run on the machine that builds them, so that target is the one a
//! package is built for when no other is asked for.

/// The variable Cargo gives a build script the target in.
const TARGET_VARIABLE: &str = "TARGET";
/// The variable the code of the tools reads it from.
const HOST_VARIABLE: &str = "XTASK_HOST";

fn main() {
    if let Ok(target) = std::env::var(TARGET_VARIABLE) {
        println!("cargo:rustc-env={HOST_VARIABLE}={target}");
    }
}
