<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  import '#lib/styles/tokens.css';
  import '#lib/styles/base.css';

  import type { Snippet } from 'svelte';

  import { i18n } from '#lib/i18n/i18n.svelte.ts';

  const { children }: { children: Snippet } = $props();

  // Tells assistive technology and hyphenation which language the page is in.
  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
</script>

{@render children()}
