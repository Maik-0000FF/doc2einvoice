<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The one heading of a part of a section, below a section heading.
  const { text }: { text: string } = $props();
</script>

<h4 class="heading">{text}</h4>

<style>
  .heading {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
  }
</style>
