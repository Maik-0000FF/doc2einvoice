<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one list of facts of the interface: a term and its value per line,
  // the terms in a column of their own.
  import type { Fact } from './facts.ts';

  const { facts }: { facts: readonly Fact[] } = $props();
</script>

<dl class="facts">
  <!-- Terms are translated texts, and a translation may word two of them alike. -->
  {#each facts as fact, index (index)}
    <dt>
      {#if fact.labelFor === undefined}
        {fact.term}
      {:else}
        <label for={fact.labelFor}>{fact.term}</label>
      {/if}
    </dt>
    <dd class:long-words={fact.long} class:facts__value--selectable={fact.selectable}>
      {#if typeof fact.value === 'string'}
        {fact.value}
      {:else}
        {@render fact.value()}
      {/if}
    </dd>
  {/each}
</dl>

<style>
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-md);
    margin: var(--space-md) 0;
    font-size: var(--font-size-sm);
  }

  .facts dt {
    color: var(--color-text-muted);
  }

  .facts dd {
    margin: 0;
  }

  .facts__value--selectable {
    user-select: all;
  }
</style>
