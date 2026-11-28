// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import type { Library } from '#lib/ipc/bindings/Library.ts';

import { textSummary, textUses } from './library-notices.ts';

const FIRST = 'The first sample terms.';
const SECOND = 'The second sample terms.';

function library(name: string, texts: number[]): Library {
  return { name, version: '1.0.0', license: 'MIT', texts, changes: null };
}

describe('textUses', () => {
  it('names both libraries that refer to one text', () => {
    const uses = textUses({
      texts: [FIRST],
      libraries: [library('alpha', [0]), library('beta', [0])],
    });

    expect(uses).toEqual([{ text: FIRST, libraries: ['alpha 1.0.0', 'beta 1.0.0'] }]);
  });

  it('names a library with each of its texts', () => {
    const uses = textUses({ texts: [FIRST, SECOND], libraries: [library('alpha', [0, 1])] });

    expect(uses).toEqual([
      { text: FIRST, libraries: ['alpha 1.0.0'] },
      { text: SECOND, libraries: ['alpha 1.0.0'] },
    ]);
  });

  it('keeps a text no library refers to', () => {
    const uses = textUses({ texts: [FIRST, SECOND], libraries: [library('alpha', [1])] });

    expect(uses[0]).toEqual({ text: FIRST, libraries: [] });
  });

  it('leaves out a reference to a text that is not there', () => {
    const uses = textUses({ texts: [FIRST], libraries: [library('alpha', [0, 7, -1])] });

    expect(uses).toEqual([{ text: FIRST, libraries: ['alpha 1.0.0'] }]);
  });

  it('gives nothing for a list without texts', () => {
    expect(textUses({ texts: [], libraries: [library('alpha', [])] })).toEqual([]);
  });
});

describe('textSummary', () => {
  it('names the libraries of a text', () => {
    expect(textSummary({ text: FIRST, libraries: ['alpha 1.0.0', 'beta 1.0.0'] })).toBe(
      'alpha 1.0.0, beta 1.0.0',
    );
  });
});
