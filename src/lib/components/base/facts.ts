// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// The shape of one line of the one list of facts of the interface
// (FactList.svelte).

import type { Snippet } from 'svelte';

export interface Fact {
  term: string;
  /** A text, or a control that shows the value and lets it be changed. */
  value: string | Snippet;
  /** Names the control in the value, so that the term is its label. */
  labelFor?: string;
  /** The value is one word that may be longer than the list is wide. */
  long?: boolean;
  /** A click selects the whole value, so that it can be copied at once. */
  selectable?: boolean;
}
