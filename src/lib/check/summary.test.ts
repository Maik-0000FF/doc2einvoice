// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { LOCALES } from '#lib/i18n/locale.ts';
import { MESSAGES, placeholdersOf } from '#lib/i18n/translate.ts';
import type { InvoiceCheck } from '#lib/ipc/bindings/InvoiceCheck.ts';
import type { Refusal } from '#lib/ipc/bindings/Refusal.ts';
import type { RuleNote } from '#lib/ipc/bindings/RuleNote.ts';
import type { SeverityKind } from '#lib/ipc/bindings/SeverityKind.ts';

import {
  countBySeverity,
  PROFILE_KEYS,
  PROFILES,
  refusalMessage,
  SEVERITY_KEYS,
  SYNTAX_KEYS,
} from './summary.ts';

function finding(severity: SeverityKind): RuleNote {
  return { ruleId: 'BR-01', severity, message: '', location: '/Invoice[1]', evaluationError: null };
}

function check(severities: readonly (readonly SeverityKind[])[]): InvoiceCheck {
  return {
    fileName: 'invoice.xml',
    syntax: 'ubl',
    profile: 'en16931',
    specificationIdentifier: null,
    ruleCoverage: 'notNamed',
    conforms: false,
    schemaFindings: [],
    ruleSets: severities.map((ruleSet) => ({
      publication: {
        title: 'Rules',
        version: '1',
        published: '2026-01-01',
        license: null,
        source: 'https://example.org',
      },
      findings: ruleSet.map(finding),
    })),
  };
}

// One refusal of every kind. A kind that is added to the core and not here
// does not compile in `refusalMessage`.
const REFUSALS: readonly Refusal[] = [
  { reason: 'notAFile' },
  { reason: 'unreadable' },
  { reason: 'tooLarge', limitBytes: 32 * 1024 * 1024 },
  { reason: 'notUtf8' },
  { reason: 'malformed', detail: 'unexpected end' },
  { reason: 'tooDeeplyNested', limit: 256 },
  { reason: 'unsupportedDocument', element: 'Order' },
  { reason: 'nothingToCheck' },
  { reason: 'internal' },
];

describe('countBySeverity', () => {
  it('counts nothing in a check without findings', () => {
    expect(countBySeverity(check([[], []]))).toEqual({ fatal: 0, warning: 0, information: 0 });
  });

  it('counts the findings of every rule set', () => {
    const counts = countBySeverity(
      check([
        ['fatal', 'warning'],
        ['fatal', 'information', 'fatal'],
      ]),
    );

    expect(counts).toEqual({ fatal: 3, warning: 1, information: 1 });
  });
});

describe('refusalMessage', () => {
  it('states the size limit in mebibytes', () => {
    expect(refusalMessage({ reason: 'tooLarge', limitBytes: 32 * 1024 * 1024 }).values).toEqual({
      mebibytes: 32,
    });
  });

  it.each(REFUSALS)('gives $reason every value its text asks for', (refusal) => {
    const message = refusalMessage(refusal);
    const given = new Set(Object.keys(message.values ?? {}));

    for (const locale of LOCALES) {
      expect(placeholdersOf(MESSAGES[locale][message.name]), locale).toEqual(given);
    }
  });

  it('gives every refusal a text of its own', () => {
    const keys = REFUSALS.map((refusal) => refusalMessage(refusal).name);

    expect(new Set(keys).size).toBe(REFUSALS.length);
  });
});

describe('the names of profiles, syntaxes and severities', () => {
  it('offers every profile', () => {
    expect([...PROFILES].sort()).toEqual(Object.keys(PROFILE_KEYS).sort());
  });

  it('gives every one a text of its own', () => {
    for (const keys of [PROFILE_KEYS, SYNTAX_KEYS, SEVERITY_KEYS]) {
      const texts = Object.values(keys);

      expect(new Set(texts).size).toBe(texts.length);
    }
  });
});
