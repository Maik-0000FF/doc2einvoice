<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one button of the interface: every button is this element, so its
  // look and its behaviour are set here and nowhere else.
  import type { Snippet } from 'svelte';

  const {
    children,
    emphasis = 'secondary',
    type = 'button',
    disabled = false,
    busy = false,
    onclick,
  }: {
    children: Snippet;
    /** A primary button is the main action of its view; there is one at most. */
    emphasis?: 'primary' | 'secondary';
    /** A submit button closes the dialog whose form it is in. */
    type?: 'button' | 'submit';
    disabled?: boolean;
    /** The action of the button is running; it cannot be pressed meanwhile. */
    busy?: boolean;
    onclick?: () => void;
  } = $props();
</script>

<button
  class="button button--{emphasis}"
  class:button--busy={busy}
  {type}
  disabled={disabled || busy}
  {onclick}
>
  {@render children()}
</button>

<style>
  .button {
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    cursor: pointer;
  }

  .button--primary {
    padding: var(--space-sm) var(--space-lg);
    font-weight: var(--font-weight-bold);
  }

  .button--secondary {
    padding: var(--space-xs) var(--space-md);
  }

  .button:disabled {
    color: var(--color-text-muted);
    cursor: not-allowed;
  }

  .button--busy:disabled {
    cursor: progress;
  }
</style>
