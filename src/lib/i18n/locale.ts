// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

/** The languages the interface is translated into. */
export const LOCALES = ['en', 'de'] as const;

/** A language of the interface. */
export type Locale = (typeof LOCALES)[number];

/** The language used when none of the preferred ones is available. */
export const DEFAULT_LOCALE: Locale = 'en';

/**
 * Chooses the interface language from the languages the user prefers, in the
 * order the operating system states them.
 *
 * | Preferred                 | Result                                      |
 * | ------------------------- | ------------------------------------------- |
 * | none                      | the default                                 |
 * | `de`                      | `de`                                        |
 * | `de-AT`, `de-Latn-DE`     | `de`: region and script do not matter       |
 * | `DE`, `De-at`             | `de`: case does not matter                  |
 * | `fr`, `de`                | `de`: the first one that is available       |
 * | `en-US`, `de`             | `en`: the first one that is available       |
 * | `fr`                      | the default                                 |
 * | `dex`                     | the default: a different language           |
 * | `` (empty), `de`          | `de`: what is no language tag is skipped    |
 * | `not a tag`, `de`         | `de`                                        |
 * | `de_DE`                   | the default: no language tag, never guessed |
 */
export function resolveLocale(preferred: readonly string[]): Locale {
  for (const tag of preferred) {
    const language = languageOf(tag);
    const locale = LOCALES.find((candidate) => candidate === language);
    if (locale !== undefined) {
      return locale;
    }
  }
  return DEFAULT_LOCALE;
}

/** The language of a language tag, or nothing if the text is no such tag. */
function languageOf(tag: string): string | undefined {
  try {
    return new Intl.Locale(tag).language;
  } catch {
    return undefined;
  }
}
