// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import type { AppInfo } from '#lib/ipc/bindings/AppInfo.ts';

/**
 * The legal notice shown in the user interface: copyright holder and licence.
 *
 * The wording is a legal notice, not interface text, and is therefore the same
 * in every language.
 */
export function formatLegalNotice(info: AppInfo): string {
  return `Copyright (C) ${info.copyrightHolder}. Licensed under ${info.license}.`;
}
