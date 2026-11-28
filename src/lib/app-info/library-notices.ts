// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import type { Library } from '#lib/ipc/bindings/Library.ts';
import type { LibraryNotices } from '#lib/ipc/bindings/LibraryNotices.ts';

/**
 * A text that is passed on with libraries, as a licence or a notice is, with
 * the libraries it belongs to.
 */
export interface TextUse {
  text: string;
  /** The libraries, each as its name and version, in the order of the list. */
  libraries: string[];
}

/** A library as it is named beside a text: its name and its version. */
export function libraryLabel(library: Library): string {
  return `${library.name} ${library.version}`;
}

/**
 * The texts of a list, each with the libraries that refer to it.
 *
 * | List                                         | Result                           |
 * | -------------------------------------------- | -------------------------------- |
 * | two libraries refer to one text              | the text, with both libraries    |
 * | one library refers to two texts              | both texts, each with it         |
 * | a text no library refers to                  | the text, with no library        |
 * | a library refers to a text that is not there | the reference is left out        |
 * | no texts                                     | nothing                          |
 */
export function textUses(notices: LibraryNotices): TextUse[] {
  const uses: TextUse[] = notices.texts.map((text) => ({ text, libraries: [] }));
  for (const library of notices.libraries) {
    for (const place of library.texts) {
      uses[place]?.libraries.push(libraryLabel(library));
    }
  }
  return uses;
}

/** The line that names a text: the libraries it belongs to. */
export function textSummary(use: TextUse): string {
  return use.libraries.join(', ');
}
