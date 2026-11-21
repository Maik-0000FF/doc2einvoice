<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one modal dialog of the interface: a title, an optional line below it,
  // the content, and a button that closes it. Escape closes it as well.
  import type { Snippet } from 'svelte';

  import Button from './Button.svelte';
  import Note from './Note.svelte';

  let {
    open = $bindable(false),
    title,
    subtitle,
    closeLabel,
    children,
  }: {
    open?: boolean;
    title: string;
    subtitle?: string;
    closeLabel: string;
    children: Snippet;
  } = $props();

  // Lets a script focus an element that the Tab key does not stop at.
  const NOT_IN_TAB_ORDER = -1;

  // Identifies the heading that names the dialog for assistive technology.
  const titleId = $props.id();

  let dialog = $state<HTMLDialogElement>();

  // The dialog element is opened and closed by its own methods: only they make
  // it modal, keep the focus inside and close it on Escape. Opening would put
  // the focus on the first control, the button at the very end, and scroll
  // there; the dialog takes the focus itself and starts at its top.
  $effect(() => {
    if (dialog === undefined || dialog.open === open) {
      return;
    }
    if (open) {
      dialog.showModal();
      dialog.scrollTop = 0;
      dialog.focus();
    } else {
      dialog.close();
    }
  });
</script>

<dialog
  bind:this={dialog}
  class="dialog"
  tabindex={NOT_IN_TAB_ORDER}
  aria-labelledby={titleId}
  onclose={() => {
    open = false;
  }}
>
  <header class="dialog__header">
    <h2 id={titleId} class="dialog__title">{title}</h2>
    {#if subtitle !== undefined}
      <Note>{subtitle}</Note>
    {/if}
  </header>

  {@render children()}

  <form method="dialog" class="dialog__actions">
    <Button type="submit">{closeLabel}</Button>
  </form>
</dialog>

<style>
  .dialog {
    width: min(var(--dialog-width), calc(100vw - 2 * var(--space-lg)));
    max-height: calc(100vh - 2 * var(--space-lg));
    padding: var(--space-lg);
    overflow-y: auto;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
  }

  /* The dialog takes the focus only so that reading starts at its top; it is
     no control, so it is not marked as one. */
  .dialog:focus-visible {
    outline: none;
  }

  .dialog::backdrop {
    background: var(--color-backdrop);
  }

  .dialog__header {
    margin-bottom: var(--space-md);
  }

  .dialog__title {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .dialog__actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-lg);
  }
</style>
