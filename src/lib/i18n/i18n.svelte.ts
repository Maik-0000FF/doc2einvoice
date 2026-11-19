// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { formatDate } from './format.ts';
import { resolveLocale } from './locale.ts';
import type { Locale } from './locale.ts';
import { translate } from './translate.ts';
import type { MessageKey, MessageValues } from './translate.ts';

// The language of the interface. It starts as the first available one of the
// languages the operating system reports; whatever reads it is updated when it
// changes.
let locale = $state<Locale>(resolveLocale(navigator.languages));

/**
 * The texts and notations of the interface in its current language. Every
 * visible text comes from here; components contain none of their own.
 */
export const i18n = {
  get locale(): Locale {
    return locale;
  },
  set locale(value: Locale) {
    locale = value;
  },
  /** The text of the given name, with its placeholders replaced. */
  t(key: MessageKey, values?: MessageValues): string {
    return translate(locale, key, values);
  },
  /** A date of the form `YYYY-MM-DD`, written out. */
  date(isoDate: string): string {
    return formatDate(isoDate, locale);
  },
};
