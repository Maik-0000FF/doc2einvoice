// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from '@tauri-apps/api/core';

import type { LibraryNotices } from './bindings/LibraryNotices.ts';

// Name of the command as registered by the application core.
const LIBRARY_NOTICES_COMMAND = 'library_notices';

/**
 * Asks the application core for the list of the libraries this build
 * contains. Resolves to `null` if the build carries none: the list is
 * generated when a package is built.
 *
 * The core hands the list on as its generator wrote it, and the type it is
 * read as here is derived from that generator.
 */
export async function fetchLibraryNotices(): Promise<LibraryNotices | null> {
  const list = await invoke<string | null>(LIBRARY_NOTICES_COMMAND);
  return list === null ? null : (JSON.parse(list) as LibraryNotices);
}
