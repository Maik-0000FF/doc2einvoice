// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import type { Library } from '#lib/ipc/bindings/Library.ts';
import type { LicenseText } from '#lib/ipc/bindings/LicenseText.ts';

import { licenseTextSummary, licenseTextUses } from './library-notices.ts';

const FIRST: LicenseText = { license: 'MIT', text: 'The first sample terms.' };
const SECOND: LicenseText = { license: 'Zlib', text: 'The second sample terms.' };

function library(name: string, texts: number[]): Library {
  return { name, version: '1.0.0', license: 'MIT', texts, changes: null };
}

describe('licenseTextUses', () => {
  it('names both libraries that refer to one text', () => {
    const uses = licenseTextUses({
      texts: [FIRST],
      libraries: [library('alpha', [0]), library('beta', [0])],
    });

    expect(uses).toEqual([{ ...FIRST, libraries: ['alpha 1.0.0', 'beta 1.0.0'] }]);
  });

  it('names a library with each of its texts', () => {
    const uses = licenseTextUses({ texts: [FIRST, SECOND], libraries: [library('alpha', [0, 1])] });

    expect(uses).toEqual([
      { ...FIRST, libraries: ['alpha 1.0.0'] },
      { ...SECOND, libraries: ['alpha 1.0.0'] },
    ]);
  });

  it('keeps a text no library refers to', () => {
    const uses = licenseTextUses({ texts: [FIRST, SECOND], libraries: [library('alpha', [1])] });

    expect(uses[0]).toEqual({ ...FIRST, libraries: [] });
  });

  it('leaves out a reference to a text that is not there', () => {
    const uses = licenseTextUses({ texts: [FIRST], libraries: [library('alpha', [0, 7, -1])] });

    expect(uses).toEqual([{ ...FIRST, libraries: ['alpha 1.0.0'] }]);
  });

  it('gives nothing for a list without texts', () => {
    expect(licenseTextUses({ texts: [], libraries: [library('alpha', [])] })).toEqual([]);
  });
});

describe('licenseTextSummary', () => {
  it('names the licence and the libraries of a text', () => {
    const summary = licenseTextSummary({ ...FIRST, libraries: ['alpha 1.0.0', 'beta 1.0.0'] });

    expect(summary).toBe('MIT: alpha 1.0.0, beta 1.0.0');
  });
});
