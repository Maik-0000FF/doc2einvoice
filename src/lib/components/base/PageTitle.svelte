<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one heading of the window, above every other.
  const { text }: { text: string } = $props();
</script>

<h1 class="title">{text}</h1>

<style>
  .title {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }
</style>
