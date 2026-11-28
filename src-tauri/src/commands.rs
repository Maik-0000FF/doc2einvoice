// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The IPC boundary: every function the user interface may call.
//!
//! Commands only translate between the interface and the core. Each one must be
//! registered in [`crate::run`] to be reachable.
//!
//! The interface never names a file. The file dialog of the operating system
//! is opened here, and the path the user chose stays in the core. The same
//! holds for a file the program was started with: the interface asks for its
//! check, not for its name.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::app_info::AppInfo;
use crate::checking;
use crate::checking::document::INVOICE_EXTENSION;
use crate::checking::outcome::{CheckOutcome, ProfileKind, Refusal};
use crate::checking::service::ValidationService;
use crate::notices;

/// The kind of file the dialog offers, as it names it.
const INVOICE_FILE_KIND: &str = "XML";

/// The file the program was started with, until the interface has asked for
/// it. It is handed out once: a window that loads again does not check the
/// file a second time on its own.
#[derive(Debug, Default)]
pub struct StartFile(Mutex<Option<PathBuf>>);

impl StartFile {
    /// Keeps the file the command line named, if it named one.
    #[must_use]
    pub fn new(file: Option<PathBuf>) -> Self {
        Self(Mutex::new(file))
    }

    /// The file, the first time this is asked; nothing ever after.
    ///
    /// | State                                       | Result      |
    /// |---------------------------------------------|-------------|
    /// | started with a file, asked for the first time | the file  |
    /// | asked again                                 | nothing     |
    /// | started without a file                      | nothing     |
    /// | a thread panicked while it held the file    | nothing: no |
    /// |                                             | file is     |
    /// |                                             | checked     |
    /// |                                             | unasked     |
    fn take(&self) -> Option<PathBuf> {
        self.0.lock().ok()?.take()
    }
}

/// Returns the program identity for the legal notice.
#[tauri::command]
#[must_use]
pub fn app_info() -> AppInfo {
    AppInfo::current()
}

/// Returns the list of the libraries this build contains as JSON, in the form
/// its generator defines, or nothing if the build carries no list.
#[tauri::command]
#[must_use]
pub fn library_notices() -> Option<&'static str> {
    notices::embedded()
}

/// Lets the user choose an invoice and checks it as the profile it suggests.
///
/// The dialog is given the window that asked as its parent. Windows and macOS
/// then keep it in front of that window, which takes no input until the dialog
/// is closed. The dialogs of GTK on Linux take no parent: there the window
/// stays usable, and a second request opens a second dialog. Nothing here
/// relies on the window being blocked.
#[tauri::command]
pub async fn check_invoice(app: AppHandle, window: WebviewWindow) -> CheckOutcome {
    off_the_event_loop(move || {
        let Some(chosen) = app
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter(INVOICE_FILE_KIND, &[INVOICE_EXTENSION])
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

/// Checks the file the program was started with, as the profile it suggests.
/// Answers with nothing if the program was started without a file, or if the
/// file was asked for before.
#[tauri::command]
pub async fn check_start_file(app: AppHandle) -> Option<CheckOutcome> {
    let file = app.state::<StartFile>().take()?;
    Some(
        off_the_event_loop(move || checking::check_file(&app.state::<ValidationService>(), &file))
            .await,
    )
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::StartFile;

    #[test]
    fn the_start_file_is_handed_out_once() {
        let file = PathBuf::from("invoice.xml");
        let start = StartFile::new(Some(file.clone()));

        assert_eq!(start.take(), Some(file));
        assert_eq!(start.take(), None);
    }

    #[test]
    fn without_a_start_file_there_is_nothing_to_hand_out() {
        assert_eq!(StartFile::new(None).take(), None);
        assert_eq!(StartFile::default().take(), None);
    }
}
