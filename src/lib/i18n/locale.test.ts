// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { DEFAULT_LOCALE, resolveLocale } from './locale.ts';

describe('resolveLocale', () => {
  it.each([
    [[], DEFAULT_LOCALE],
    [['de'], 'de'],
    [['de-AT'], 'de'],
    [['de-Latn-DE'], 'de'],
    [['DE'], 'de'],
    [['De-at'], 'de'],
    [['fr', 'de'], 'de'],
    [['en-US', 'de'], 'en'],
    [['fr'], DEFAULT_LOCALE],
    [['dex'], DEFAULT_LOCALE],
    [['', 'de'], 'de'],
    [['not a tag', 'de'], 'de'],
    [['de_DE'], DEFAULT_LOCALE],
  ])('chooses for %j the language %s', (preferred, expected) => {
    expect(resolveLocale(preferred)).toBe(expected);
  });
});
