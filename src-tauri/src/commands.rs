// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The IPC boundary: every function the user interface may call.
//!
//! Commands only translate between the interface and the core. Each one must be
//! registered in [`crate::run`] to be reachable.

use crate::app_info::AppInfo;

/// Returns the program identity for the legal notice.
#[tauri::command]
#[must_use]
pub fn app_info() -> AppInfo {
    AppInfo::current()
}
