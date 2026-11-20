// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from '@tauri-apps/api/core';

import type { CheckOutcome } from './bindings/CheckOutcome.ts';
import type { ProfileKind } from './bindings/ProfileKind.ts';

// Names of the commands as registered by the application core.
const CHECK_INVOICE_COMMAND = 'check_invoice';
const CHECK_INVOICE_AGAIN_COMMAND = 'check_invoice_again';
const CHECK_START_FILE_COMMAND = 'check_start_file';

/**
 * Lets the user choose an invoice file and checks it. The application core
 * opens the file dialog; no path passes through the interface.
 */
export function checkInvoice(): Promise<CheckOutcome> {
  return invoke<CheckOutcome>(CHECK_INVOICE_COMMAND);
}

/**
 * Checks the file the program was started with. Resolves to `null` if it was
 * started without a file, or if the file was asked for before. The path stays
 * in the application core here as well.
 */
export function checkStartFile(): Promise<CheckOutcome | null> {
  return invoke<CheckOutcome | null>(CHECK_START_FILE_COMMAND);
}

/** Checks the invoice of the last check again, as the given profile. */
export function checkInvoiceAgain(profile: ProfileKind): Promise<CheckOutcome> {
  return invoke<CheckOutcome>(CHECK_INVOICE_AGAIN_COMMAND, { profile });
}
