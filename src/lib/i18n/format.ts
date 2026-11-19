// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Locale } from './locale.ts';

/** Length of a date written as `YYYY-MM-DD`. */
const ISO_DATE_LENGTH = 10;

/** The time added to a date so that it is read as a day in universal time. */
const START_OF_DAY_UTC = 'T00:00:00Z';

const UTC = 'UTC';

/**
 * Writes a date of the form `YYYY-MM-DD` the way the language writes dates,
 * with the month spelled out.
 *
 * | Date               | Result                                           |
 * | ------------------ | ------------------------------------------------ |
 * | `2026-10-12`       | `October 12, 2026` or `12. Oktober 2026`         |
 * | `2024-02-29`       | written out: a leap day                          |
 * | `2026-02-30`       | unchanged: no such day, never moved to another   |
 * | `2026-13-01`       | unchanged                                        |
 * | `2026-10`          | unchanged: not a full date                       |
 * | `2026-10-12T10:00` | unchanged: more than a date                      |
 * | `12.10.2026`       | unchanged: another notation, never guessed       |
 * | `` (empty), `abc`  | unchanged                                        |
 *
 * What is not a date of that form is shown as it is, so that nothing is hidden
 * and nothing is invented.
 */
export function formatDate(isoDate: string, locale: Locale): string {
  const date = new Date(`${isoDate}${START_OF_DAY_UTC}`);
  if (Number.isNaN(date.getTime()) || isoDateOf(date) !== isoDate) {
    return isoDate;
  }
  return new Intl.DateTimeFormat(locale, { dateStyle: 'long', timeZone: UTC }).format(date);
}

/** The day of a point in time, in universal time, as `YYYY-MM-DD`. */
function isoDateOf(date: Date): string {
  return date.toISOString().slice(0, ISO_DATE_LENGTH);
}
