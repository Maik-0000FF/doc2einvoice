<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts" generics="Row">
  // The one table of the interface: a heading per column and a line per row.
  // The first column names its row for assistive technology.
  import type { RowKey, TableColumn } from './table.ts';

  const {
    columns,
    rows,
    key,
  }: {
    columns: readonly TableColumn<Row>[];
    rows: readonly Row[];
    key: RowKey<Row>;
  } = $props();
</script>

<table class="table">
  <thead>
    <tr>
      <!-- The columns never change order; the key is the one the lint rule requires. -->
      {#each columns as column, index (index)}
        <th scope="col">{column.heading}</th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each rows as row, rowIndex (key(row, rowIndex))}
      <tr>
        <!-- The columns never change order; the key is the one the lint rule requires. -->
        {#each columns as column, index (index)}
          {#if index === 0}
            <th
              scope="row"
              class:long-words={column.wrap === 'anywhere'}
              class:table__cell--never={column.wrap === 'never'}
            >
              {@render column.cell(row)}
            </th>
          {:else}
            <td
              class:long-words={column.wrap === 'anywhere'}
              class:table__cell--never={column.wrap === 'never'}
            >
              {@render column.cell(row)}
            </td>
          {/if}
        {/each}
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
    text-align: left;
  }

  .table th,
  .table td {
    padding: var(--space-xs) var(--space-sm) var(--space-xs) 0;
    border-bottom: var(--border-width) solid var(--color-border);
    font-weight: var(--font-weight-regular);
    vertical-align: top;
  }

  .table thead th {
    color: var(--color-text-muted);
  }

  .table__cell--never {
    white-space: nowrap;
  }
</style>
