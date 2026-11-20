// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Watching the web view the user interface is drawn in.
//!
//! The web view runs in a process of its own. If that process ends, the
//! window stays open and shows nothing: without a working graphics driver,
//! for instance, it ends before it has drawn anything. The application
//! cannot be used from then on. The reason is written to stderr at once, a
//! dialog of the operating system says so as well, and when the dialog is
//! closed the application ends. The dialog is needed because nobody may be
//! reading stderr: a program started from a menu would otherwise just vanish.
//!
//! The application waits for the dialog to be closed, however long that
//! takes. That is a decision: a window that is gone when its user comes back
//! is what the dialog is there to prevent. A start nobody watches, from a
//! script, therefore does not end by itself; the line on stderr is what tells
//! such a caller what happened.
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

#[cfg(target_os = "linux")]
use crate::texts::{self, Text};

/// What stderr says when the web view was lost.
#[cfg(target_os = "linux")]
const LOST_ON_STDERR: &str = "the web view that draws the window ended unexpectedly, \
    the user interface cannot be shown; a graphics driver that does not work is a common cause";

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

    /// Notes the loss, writes the reason to stderr, says so in a dialog and
    /// ends the application when the dialog is closed.
    ///
    /// | The process of the web view                    | Result                 |
    /// |------------------------------------------------|------------------------|
    /// | crashed                                        | noted, dialog, the end |
    /// | was ended for using too much memory            | noted, dialog, the end |
    /// | was ended on request                           | noted, dialog, the end |
    /// | ended for a reason this program does not know  | noted, dialog, the end |
    /// | of another window is lost while the dialog is  | nothing more: one      |
    /// | open                                           | dialog, one end        |
    ///
    /// The application never ends the process itself, so every reason is a
    /// loss.
    ///
    /// The dialog belongs to the window whose web view was lost, which keeps
    /// it in front of that window. If the operating system cannot show the
    /// dialog, nothing reports that here, and the application stays open with
    /// its empty window until it is closed. That is a known limit.
    #[cfg(target_os = "linux")]
    fn note_loss(&self, app: &AppHandle, window: &tauri::WebviewWindow) {
        use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

        if self.lost.swap(true, Ordering::SeqCst) {
            return;
        }
        // At once, not when the dialog is closed: whoever reads stderr
        // learns the reason while the application still waits.
        eprintln!("{}: {LOST_ON_STDERR}", env!("CARGO_PKG_NAME"));
        let ending = app.clone();
        app.dialog()
            .message(texts::text(Text::WebViewLostText))
            .title(texts::text(Text::WebViewLostTitle))
            .kind(MessageDialogKind::Error)
            .parent(window)
            .show(move |_| ending.exit(LOST_EXIT_CODE));
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
            let owner = window.clone();
            window.with_webview(move |web_view| {
                web_view
                    .inner()
                    .connect_web_process_terminated(move |_, _| watch.note_loss(&app, &owner));
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
