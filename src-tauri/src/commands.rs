// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The IPC boundary: every function the user interface may call.
//!
//! Commands only translate between the interface and the core. Each one must be
//! registered in [`crate::run`] to be reachable.
//!
//! The interface never names a file. The file dialog of the operating system
//! is opened here, and the path the user chose stays in the core.

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::app_info::AppInfo;
use crate::checking;
use crate::checking::outcome::{CheckOutcome, ProfileKind, Refusal};
use crate::checking::service::ValidationService;

/// The kind of file the dialog offers, as it names it, and its extensions.
const INVOICE_FILE_KIND: &str = "XML";
const INVOICE_FILE_EXTENSIONS: [&str; 1] = ["xml"];

/// Returns the program identity for the legal notice.
#[tauri::command]
#[must_use]
pub fn app_info() -> AppInfo {
    AppInfo::current()
}

/// Lets the user choose an invoice and checks it as the profile it suggests.
///
/// The dialog belongs to the window that asked: it stays in front of it, and
/// the window takes no input until the dialog is closed.
#[tauri::command]
pub async fn check_invoice(app: AppHandle, window: WebviewWindow) -> CheckOutcome {
    off_the_event_loop(move || {
        let Some(chosen) = app
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter(INVOICE_FILE_KIND, &INVOICE_FILE_EXTENSIONS)
            .blocking_pick_file()
        else {
            return CheckOutcome::Cancelled;
        };
        // What the dialog returns that is no path in the file system cannot be
        // read as a file.
        let Ok(path) = chosen.into_path() else {
            return CheckOutcome::Refused {
                refusal: Refusal::Unreadable,
            };
        };
        checking::check_file(&app.state::<ValidationService>(), &path)
    })
    .await
}

/// Checks the invoice of the last check again, as the given profile.
#[tauri::command]
pub async fn check_invoice_again(app: AppHandle, profile: ProfileKind) -> CheckOutcome {
    off_the_event_loop(move || {
        checking::check_again(&app.state::<ValidationService>(), profile.into())
    })
    .await
}

/// Runs work that waits (for the user, for the validation) where it does not
/// hold up the window. Work that ends without an answer failed inside the
/// program.
async fn off_the_event_loop(work: impl FnOnce() -> CheckOutcome + Send + 'static) -> CheckOutcome {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .unwrap_or(CheckOutcome::Refused {
            refusal: Refusal::Internal,
        })
}
