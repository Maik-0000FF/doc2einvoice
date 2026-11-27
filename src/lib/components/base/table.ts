// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// The shape of a column of the one table of the interface (Table.svelte).

import type { Snippet } from 'svelte';

/**
 * How a long value wraps: as text does, anywhere (a path or an identifier is
 * one word that may be longer than its column is wide), or not at all.
 */
export type CellWrap = 'normal' | 'anywhere' | 'never';

/** Tells the rows of a table apart, so that a changed list redraws only what changed. */
export type RowKey<Row> = (row: Row, index: number) => string | number;

/** For rows that have nothing to tell them apart but their place. */
export const byPosition: RowKey<unknown> = (_, index) => index;

export interface TableColumn<Row> {
  heading: string;
  /** Draws the value of this column for one row. */
  cell: Snippet<[Row]>;
  wrap?: CellWrap;
}
