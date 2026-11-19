// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { DEFAULT_LOCALE, LOCALES } from './locale.ts';
import { fill, MESSAGES, placeholdersOf, translate } from './translate.ts';
import type { MessageKey } from './translate.ts';

describe('fill', () => {
  it.each([
    ['Close', {}, 'Close'],
    ['About {name}', { name: 'x' }, 'About x'],
    ['{a} and {a}', { a: 1 }, '1 and 1'],
    ['About {name}', {}, 'About {name}'],
    ['About {name}', { name: '{name}' }, 'About {name}'],
    ['About {name}', { name: '$&' }, 'About $&'],
    ['Close', { name: 'x' }, 'Close'],
    ['{}', { name: 'x' }, '{}'],
    ['{ name }', { name: 'x' }, '{ name }'],
    ['{toString}', {}, '{toString}'],
  ])('turns %j with %j into %j', (text, values, expected) => {
    expect(fill(text, values)).toBe(expected);
  });
});

describe('translate', () => {
  it('returns the text of the language with its values', () => {
    expect(translate('en', 'about.title', { name: 'x' })).toBe('About x');
    expect(translate('de', 'about.title', { name: 'x' })).toBe('Über x');
  });
});

describe('the language files', () => {
  const keys = Object.keys(MESSAGES[DEFAULT_LOCALE]) as MessageKey[];

  it.each(LOCALES)('%s contains exactly the texts of the default language', (locale) => {
    expect(Object.keys(MESSAGES[locale]).sort()).toEqual([...keys].sort());
  });

  it.each(LOCALES)('%s leaves no text empty', (locale) => {
    for (const key of keys) {
      expect(MESSAGES[locale][key].trim(), key).not.toBe('');
    }
  });

  it.each(LOCALES)('%s uses the placeholders of the default language', (locale) => {
    for (const key of keys) {
      expect(placeholdersOf(MESSAGES[locale][key]), key).toEqual(
        placeholdersOf(MESSAGES[DEFAULT_LOCALE][key]),
      );
    }
  });
});
