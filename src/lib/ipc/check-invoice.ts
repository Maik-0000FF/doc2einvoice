// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from '@tauri-apps/api/core';

import type { CheckOutcome } from './bindings/CheckOutcome.ts';
import type { ProfileKind } from './bindings/ProfileKind.ts';

// Names of the commands as registered by the application core.
const CHECK_INVOICE_COMMAND = 'check_invoice';
const CHECK_INVOICE_AGAIN_COMMAND = 'check_invoice_again';

/**
 * Lets the user choose an invoice file and checks it. The application core
 * opens the file dialog; no path passes through the interface.
 */
export function checkInvoice(): Promise<CheckOutcome> {
  return invoke<CheckOutcome>(CHECK_INVOICE_COMMAND);
}

/** Checks the invoice of the last check again, as the given profile. */
export function checkInvoiceAgain(profile: ProfileKind): Promise<CheckOutcome> {
  return invoke<CheckOutcome>(CHECK_INVOICE_AGAIN_COMMAND, { profile });
}
