// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application core of doc2einvoice.
//!
//! Layers, from the outside in:
//!
//! - [`commands`]: the IPC boundary. Thin adapters the user interface calls;
//!   they hold no logic of their own.
//! - [`app_info`]: program identity, the data of the legal notice and the
//!   published files the validation uses.
//!
//! The core never opens a network connection: every document is processed on
//! the local machine.

pub mod app_info;
pub mod commands;

/// Builds the application and runs it until the last window closes.
///
/// # Errors
///
/// Returns the Tauri error if the application cannot be set up or its event
/// loop fails.
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
}
