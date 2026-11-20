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
//! - [`cli`]: the command line, interpreted before the application starts.
//! - [`checking`]: checks an invoice the user chose and describes the result.
//! - `web_view`: notices when the web view of the window is lost.
//! - `texts`: the texts the core shows itself when no interface is left.
//!
//! The core never opens a network connection: every document is processed on
//! the local machine.

pub mod app_info;
pub mod checking;
pub mod cli;
pub mod commands;
mod texts;
mod web_view;

/// How a run of the application ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// The last window was closed.
    Closed,
    /// The process that draws the user interface ended, so the window could
    /// not show the application any more. See the `web_view` module.
    WebViewLost,
}

/// Builds the application and runs it until the last window closes or the
/// user interface is lost. If `file` names a file, the window checks it as
/// soon as it is up.
///
/// # Errors
///
/// Returns the Tauri error if the validation thread cannot be started or the
/// application cannot be built.
///
/// # Panics
///
/// Panics if a window of the configuration cannot be created or the watch
/// over the web view cannot be started. Both happen inside the event loop,
/// where the framework turns a failed setup into a panic instead of returning
/// it.
pub fn run(file: Option<std::path::PathBuf>) -> tauri::Result<Ending> {
    let watch = web_view::Watch::default();
    let watch_at_setup = watch.clone();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(checking::service::ValidationService::start()?)
        .manage(commands::StartFile::new(file))
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::check_invoice,
            commands::check_invoice_again,
            commands::check_start_file,
        ])
        .setup(move |app| Ok(watch_at_setup.start(app.handle())?))
        .build(tauri::generate_context!())?;

    // The status the event loop names is not looked at: how the run ended is
    // what the watch saw.
    let _status = app.run_return(|_, _| {});
    Ok(if watch.web_view_lost() {
        Ending::WebViewLost
    } else {
        Ending::Closed
    })
}
