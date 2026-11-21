<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // What the program is, under which terms it is distributed, which published
  // files its validation uses and what the author does not answer for.
  import { formatLegalNotice } from '#lib/app-info/legal-notice.ts';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import type { MessageKey } from '#lib/i18n/translate.ts';
  import type { AppInfo } from '#lib/ipc/bindings/AppInfo.ts';
  import type { Publication } from '#lib/ipc/bindings/Publication.ts';

  import Dialog from './base/Dialog.svelte';
  import FactList from './base/FactList.svelte';
  import type { Fact } from './base/facts.ts';
  import SectionHeading from './base/SectionHeading.svelte';
  import Table from './base/Table.svelte';
  import type { RowKey, TableColumn } from './base/table.ts';

  let { info, open = $bindable(false) }: { info: AppInfo; open?: boolean } = $props();

  // The sentences of the disclaimer, in the order they are shown.
  const DISCLAIMER: readonly MessageKey[] = [
    'about.disclaimer.advice',
    'about.disclaimer.warranty',
    'about.disclaimer.responsibility',
    'about.disclaimer.liability',
  ];

  const publications = $derived([...info.ruleSets, ...info.xmlSchemas]);

  const byTitle: RowKey<Publication> = (publication) => publication.title;

  const facts: readonly Fact[] = $derived([
    { term: i18n.t('about.license'), value: info.license },
    { term: i18n.t('about.source'), value: info.repository, long: true, selectable: true },
  ]);

  const columns: readonly TableColumn<Publication>[] = $derived([
    { heading: i18n.t('about.publications.title'), cell: titleCell },
    { heading: i18n.t('about.publications.version'), cell: versionCell },
    { heading: i18n.t('about.publications.published'), cell: publishedCell },
    { heading: i18n.t('about.license'), cell: licenseCell },
  ]);
</script>

{#snippet titleCell(publication: Publication)}{publication.title}{/snippet}
{#snippet versionCell(publication: Publication)}
  {publication.version ?? i18n.t('about.publications.notNumbered')}
{/snippet}
{#snippet publishedCell(publication: Publication)}{i18n.date(publication.published)}{/snippet}
{#snippet licenseCell(publication: Publication)}
  {publication.license ?? i18n.t('about.publications.ownTerms')}
{/snippet}

<Dialog
  bind:open
  title={i18n.t('about.title', { name: info.name })}
  subtitle={i18n.t('about.version', { version: info.version })}
  closeLabel={i18n.t('about.close')}
>
  <p class="about__legal">{formatLegalNotice(info)}</p>

  <FactList {facts} />

  <section>
    <SectionHeading text={i18n.t('about.publications.heading')} />
    <p class="about__text">{i18n.t('about.publications.text')}</p>
    <Table {columns} rows={publications} key={byTitle} />
  </section>

  <section>
    <SectionHeading text={i18n.t('about.disclaimer.heading')} />
    <p class="about__text about__text--strong">{i18n.t('about.disclaimer.risk')}</p>
    {#each DISCLAIMER as sentence (sentence)}
      <p class="about__text">{i18n.t(sentence)}</p>
    {/each}
  </section>
</Dialog>

<style>
  .about__legal {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .about__text {
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-sm);
  }

  .about__text--strong {
    font-weight: var(--font-weight-bold);
  }
</style>
