// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from '@tauri-apps/api/core';

import type { AppInfo } from './bindings/AppInfo.ts';

// Name of the command as registered by the application core.
const APP_INFO_COMMAND = 'app_info';

/** Asks the application core for the program identity and legal notice data. */
export function fetchAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>(APP_INFO_COMMAND);
}
