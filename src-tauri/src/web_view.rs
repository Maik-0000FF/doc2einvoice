// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Watching the web view the user interface is drawn in.
//!
//! The web view runs in a process of its own. If that process ends, the
//! window stays open and shows nothing: without a working graphics driver,
//! for instance, it ends before it has drawn anything. The application
//! cannot be used from then on, so it is ended, and the caller learns why.
//!
//! This holds wherever the process ends, also in the middle of a session. That
//! is a decision: nothing here can bring the user interface back, and a window
//! that silently shows nothing is the worse failure.
//!
//! The web view reports the loss of its process on Linux only. On the other
//! platforms nothing is watched.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::AppHandle;

/// Status the event loop is asked to end with when the web view was lost. The
/// caller decides by [`Watch::web_view_lost`], not by this value.
#[cfg(target_os = "linux")]
const LOST_EXIT_CODE: i32 = 1;

/// Remembers whether the web view was lost.
#[derive(Debug, Clone, Default)]
pub(crate) struct Watch {
    lost: Arc<AtomicBool>,
}

impl Watch {
    /// Whether the process of the web view ended while the application ran.
    pub(crate) fn web_view_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }

    /// Notes the loss and ends the application.
    ///
    /// | The process of the web view                    | Result           |
    /// |------------------------------------------------|------------------|
    /// | crashed                                        | noted, app ended |
    /// | was ended for using too much memory            | noted, app ended |
    /// | was ended on request                           | noted, app ended |
    /// | ended for a reason this program does not know  | noted, app ended |
    ///
    /// The application never ends the process itself, so every reason is a
    /// loss.
    #[cfg(target_os = "linux")]
    fn note_loss(&self, app: &AppHandle) {
        self.lost.store(true, Ordering::SeqCst);
        app.exit(LOST_EXIT_CODE);
    }

    /// Starts watching the web view of every window that exists.
    ///
    /// # Errors
    ///
    /// Returns the Tauri error if a web view cannot be reached. Called from
    /// the setup of the application, such an error does not reach the caller
    /// of [`crate::run`]: the framework turns a failed setup into a panic.
    #[cfg(target_os = "linux")]
    pub(crate) fn start(&self, app: &AppHandle) -> tauri::Result<()> {
        use tauri::Manager;
        use webkit2gtk::WebViewExt;

        for window in app.webview_windows().values() {
            let watch = self.clone();
            let app = app.clone();
            window.with_webview(move |web_view| {
                web_view
                    .inner()
                    .connect_web_process_terminated(move |_, _| watch.note_loss(&app));
            })?;
        }
        Ok(())
    }

    /// Nothing is watched on this platform.
    ///
    /// # Errors
    ///
    /// Never fails; the signature is that of the platform that watches.
    #[cfg(not(target_os = "linux"))]
    pub(crate) fn start(&self, _: &AppHandle) -> tauri::Result<()> {
        Ok(())
    }
}
