// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { formatDate } from './format.ts';

describe('formatDate', () => {
  it('writes a date the way the language does', () => {
    expect(formatDate('2026-10-12', 'en')).toBe('October 12, 2026');
    expect(formatDate('2026-10-12', 'de')).toBe('12. Oktober 2026');
  });

  it('writes a leap day', () => {
    expect(formatDate('2024-02-29', 'en')).toBe('February 29, 2024');
  });

  it.each([
    '2026-02-30',
    '2025-02-29',
    '2026-13-01',
    '2026-10',
    '2026-10-12T10:00',
    '12.10.2026',
    '',
    'abc',
    ' 2026-10-12',
  ])('leaves %j unchanged', (text) => {
    expect(formatDate(text, 'en')).toBe(text);
  });
});
