<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one heading of a section, below the heading of a view or a dialog.
  const { text }: { text: string } = $props();
</script>

<h3 class="heading">{text}</h3>

<style>
  .heading {
    margin-top: var(--space-lg);
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-bold);
  }
</style>
