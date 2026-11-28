<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  import { formatLegalNotice } from '#lib/app-info/legal-notice.ts';
  import AboutDialog from '#lib/components/AboutDialog.svelte';
  import Button from '#lib/components/base/Button.svelte';
  import Note from '#lib/components/base/Note.svelte';
  import PageTitle from '#lib/components/base/PageTitle.svelte';
  import InvoiceCheck from '#lib/components/InvoiceCheck.svelte';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import { fetchAppInfo } from '#lib/ipc/app-info.ts';
  import type { LibraryNotices } from '#lib/ipc/bindings/LibraryNotices.ts';
  import { fetchLibraryNotices } from '#lib/ipc/library-notices.ts';

  let documentTitle = $state('');
  let aboutOpen = $state(false);
  // The list of the contained libraries is long and is asked for once, when
  // the dialog that shows it is opened for the first time.
  let libraries = $state<Promise<LibraryNotices | null>>();

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
    <PageTitle text={info.name} />
    <Note>{info.version}</Note>
    <InvoiceCheck />
    <Button
      onclick={() => {
        aboutOpen = true;
        libraries ??= fetchLibraryNotices();
      }}
    >
      {i18n.t('about.open')}
    </Button>
    <footer class="shell__legal">
      <Note>{formatLegalNotice(info)}</Note>
    </footer>
  </main>
  <AboutDialog {info} {libraries} bind:open={aboutOpen} />
{:catch error}
  <main class="shell">
    <Note tone="danger" size="body" role="alert">{String(error)}</Note>
  </main>
{/await}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm);
    min-height: 100vh;
    padding: var(--space-lg);
  }

  .shell__legal {
    margin-top: var(--space-lg);
    text-align: center;
  }
</style>
