<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one folded text of the interface: a line that names a long text, and
  // the text itself, shown when the line is opened. The text keeps its lines
  // and paragraphs as they are written and scrolls within its own height.
  const { summary, text }: { summary: string; text: string } = $props();
</script>

<details class="folded">
  <summary class="folded__summary">{summary}</summary>
  <!-- Reachable with the keyboard, so that it can be scrolled without a mouse. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <pre class="folded__text long-words" tabindex="0">{text}</pre>
</details>

<style>
  .folded {
    margin-bottom: var(--space-md);
    font-size: var(--font-size-sm);
  }

  .folded__summary {
    cursor: pointer;
  }

  .folded__text {
    max-height: var(--folded-text-height);
    margin: var(--space-sm) 0 0;
    padding: var(--space-sm);
    overflow-y: auto;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    font-family: inherit;
    white-space: pre-wrap;
  }
</style>
