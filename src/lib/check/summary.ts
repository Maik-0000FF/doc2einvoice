// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// What the interface says about the result of a check: which text names a
// profile, a syntax, a severity or a refusal, and how many findings there are.
// The texts themselves are in the language files.

import type { MessageKey, MessageValues } from '#lib/i18n/translate.ts';
import type { InvoiceCheck } from '#lib/ipc/bindings/InvoiceCheck.ts';
import type { ProfileKind } from '#lib/ipc/bindings/ProfileKind.ts';
import type { Refusal } from '#lib/ipc/bindings/Refusal.ts';
import type { SeverityKind } from '#lib/ipc/bindings/SeverityKind.ts';
import type { SyntaxKind } from '#lib/ipc/bindings/SyntaxKind.ts';

/** The text that names each profile. A profile without a text does not compile. */
export const PROFILE_KEYS: Readonly<Record<ProfileKind, MessageKey>> = {
  en16931: 'profile.en16931',
  xrechnung: 'profile.xrechnung',
};

/** Every profile an invoice can be checked as, in the order they are offered. */
export const PROFILES = Object.keys(PROFILE_KEYS) as readonly ProfileKind[];

/** The text that names each syntax. */
export const SYNTAX_KEYS: Readonly<Record<SyntaxKind, MessageKey>> = {
  ubl: 'syntax.ubl',
  cii: 'syntax.cii',
};

/** The text that names each severity. */
export const SEVERITY_KEYS: Readonly<Record<SeverityKind, MessageKey>> = {
  fatal: 'severity.fatal',
  warning: 'severity.warning',
  information: 'severity.information',
};

/** The severities, the most serious first. */
export const SEVERITIES = Object.keys(SEVERITY_KEYS) as readonly SeverityKind[];

/** Sizes of files are stated in mebibytes. */
const BYTES_PER_MEBIBYTE = 1024 * 1024;

/** A text of the language files with the values for its placeholders. */
export interface Message {
  /** The name of the text. */
  readonly name: MessageKey;
  readonly values?: MessageValues;
}

/** How many rules of each severity the invoice does not satisfy. */
export function countBySeverity(check: InvoiceCheck): Record<SeverityKind, number> {
  const counts: Record<SeverityKind, number> = { fatal: 0, warning: 0, information: 0 };
  for (const ruleSet of check.ruleSets) {
    for (const finding of ruleSet.findings) {
      counts[finding.severity] += 1;
    }
  }
  return counts;
}

/** The text that tells the user why a file was not checked. */
export function refusalMessage(refusal: Refusal): Message {
  switch (refusal.reason) {
    case 'notAFile':
      return { name: 'refusal.notAFile' };
    case 'unreadable':
      return { name: 'refusal.unreadable' };
    case 'tooLarge':
      return {
        name: 'refusal.tooLarge',
        values: { mebibytes: refusal.limitBytes / BYTES_PER_MEBIBYTE },
      };
    case 'notUtf8':
      return { name: 'refusal.notUtf8' };
    case 'malformed':
      return { name: 'refusal.malformed', values: { detail: refusal.detail } };
    case 'unsupportedDocument':
      return { name: 'refusal.unsupportedDocument', values: { element: refusal.element } };
    case 'nothingToCheck':
      return { name: 'refusal.nothingToCheck' };
    case 'internal':
      return { name: 'refusal.internal' };
  }
}
