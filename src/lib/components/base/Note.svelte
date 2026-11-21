<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one note of the interface: a line beside the content that explains,
  // warns or reports a failure. Its tone says which.
  import type { Snippet } from 'svelte';

  const {
    children,
    tone = 'muted',
    size = 'small',
    spaced = false,
    long = false,
    role,
  }: {
    children: Snippet;
    tone?: 'muted' | 'warning' | 'danger';
    /** A small note accompanies the content; a body-sized one stands for it. */
    size?: 'small' | 'body';
    /** Keeps a gap to what follows. */
    spaced?: boolean;
    /** The note holds a word that may be longer than the space it is shown in. */
    long?: boolean;
    /** Announces the note to assistive technology when it appears. */
    role?: 'status' | 'alert';
  } = $props();
</script>

<p class="note--{tone} note--{size}" class:note--spaced={spaced} class:long-words={long} {role}>
  {@render children()}
</p>

<style>
  .note--muted {
    color: var(--color-text-muted);
  }

  .note--warning {
    color: var(--color-warning);
  }

  .note--danger {
    color: var(--color-danger);
  }

  .note--small {
    font-size: var(--font-size-sm);
  }

  .note--spaced {
    margin-bottom: var(--space-sm);
  }
</style>
