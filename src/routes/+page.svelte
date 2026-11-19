<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  import { formatLegalNotice } from '#lib/app-info/legal-notice.ts';
  import AboutDialog from '#lib/components/AboutDialog.svelte';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import { fetchAppInfo } from '#lib/ipc/app-info.ts';

  let documentTitle = $state('');
  let aboutOpen = $state(false);

  const appInfo = fetchAppInfo().then((info) => {
    documentTitle = info.name;
    return info;
  });
</script>

<svelte:head>
  <title>{documentTitle}</title>
</svelte:head>

{#await appInfo then info}
  <main class="shell">
    <h1 class="shell__name">{info.name}</h1>
    <p class="shell__version">{info.version}</p>
    <button
      class="shell__about"
      onclick={() => {
        aboutOpen = true;
      }}
    >
      {i18n.t('about.open')}
    </button>
    <footer class="shell__legal">{formatLegalNotice(info)}</footer>
  </main>
  <AboutDialog {info} bind:open={aboutOpen} />
{:catch error}
  <main class="shell">
    <p class="shell__error" role="alert">{String(error)}</p>
  </main>
{/await}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-sm);
    min-height: 100vh;
    padding: var(--space-lg);
  }

  .shell__name {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  .shell__version,
  .shell__legal {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .shell__about {
    padding: var(--space-xs) var(--space-md);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    cursor: pointer;
  }

  .shell__legal {
    margin-top: var(--space-lg);
    text-align: center;
  }

  .shell__error {
    color: var(--color-danger);
  }
</style>
