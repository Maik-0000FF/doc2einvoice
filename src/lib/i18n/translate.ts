// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Locale } from './locale.ts';
import de from './messages/de.json';
import en from './messages/en.json';

/** The name of a text of the interface. The English file defines the names. */
export type MessageKey = keyof typeof en;

/** The values put into the placeholders of a text. */
export type MessageValues = Readonly<Record<string, string | number>>;

/**
 * Every text in every language. A language file that lacks a text does not
 * compile.
 */
export const MESSAGES: Readonly<Record<Locale, Readonly<Record<MessageKey, string>>>> = { en, de };

/** A placeholder in a text: a name in braces, for example `{name}`. */
const PLACEHOLDER = /\{([A-Za-z][A-Za-z0-9]*)\}/g;

/** The names of the placeholders a text contains. */
export function placeholdersOf(text: string): Set<string> {
  return new Set(Array.from(text.matchAll(PLACEHOLDER), ([, name]) => name ?? ''));
}

/**
 * The text of the given name in the given language, with its placeholders
 * replaced.
 *
 * | Text              | Values           | Result                              |
 * | ----------------- | ---------------- | ----------------------------------- |
 * | `Close`           | none             | `Close`                             |
 * | `About {name}`    | `name: "x"`      | `About x`                           |
 * | `{a} and {a}`     | `a: 1`           | `1 and 1`                           |
 * | `About {name}`    | none             | `About {name}`: a missing value is  |
 * |                   |                  | visible, never an empty gap         |
 * | `About {name}`    | `name: "{name}"` | `About {name}`: a value is text and |
 * |                   |                  | is not searched for placeholders    |
 * | `About {name}`    | `name: "$&"`     | `About $&`: a value is taken as is  |
 * | `Close`           | `name: "x"`      | `Close`: an unused value is ignored |
 * | `{}`, `{ name }`  | any              | unchanged: not a placeholder        |
 */
export function translate(locale: Locale, key: MessageKey, values: MessageValues = {}): string {
  return fill(MESSAGES[locale][key], values);
}

/** Replaces the placeholders of a text by their values. */
export function fill(text: string, values: MessageValues): string {
  return text.replace(PLACEHOLDER, (placeholder, name: string) =>
    Object.hasOwn(values, name) ? String(values[name]) : placeholder,
  );
}
