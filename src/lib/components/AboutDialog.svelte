<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // What the program is, under which terms it is distributed, which published
  // files its validation uses, which material of other authors and which
  // libraries it contains and what the author does not answer for.
  import { formatLegalNotice } from '#lib/app-info/legal-notice.ts';
  import { licenseTextSummary, licenseTextUses } from '#lib/app-info/library-notices.ts';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import type { MessageKey } from '#lib/i18n/translate.ts';
  import type { AppInfo } from '#lib/ipc/bindings/AppInfo.ts';
  import type { Library } from '#lib/ipc/bindings/Library.ts';
  import type { LibraryNotices } from '#lib/ipc/bindings/LibraryNotices.ts';
  import type { Material } from '#lib/ipc/bindings/Material.ts';
  import type { Publication } from '#lib/ipc/bindings/Publication.ts';

  import Dialog from './base/Dialog.svelte';
  import FactList from './base/FactList.svelte';
  import type { Fact } from './base/facts.ts';
  import FoldedText from './base/FoldedText.svelte';
  import Note from './base/Note.svelte';
  import SectionHeading from './base/SectionHeading.svelte';
  import SubHeading from './base/SubHeading.svelte';
  import Table from './base/Table.svelte';
  import { byPosition, type TableColumn } from './base/table.ts';

  let {
    info,
    libraries,
    open = $bindable(false),
  }: {
    info: AppInfo;
    /**
     * The list of the libraries the build contains, once it is asked for;
     * `null` for a build that carries none.
     */
    libraries?: Promise<LibraryNotices | null> | undefined;
    open?: boolean;
  } = $props();

  // The sentences of the disclaimer, in the order they are shown.
  const DISCLAIMER: readonly MessageKey[] = [
    'about.disclaimer.advice',
    'about.disclaimer.warranty',
    'about.disclaimer.responsibility',
    'about.disclaimer.liability',
  ];

  const publications = $derived([...info.ruleSets, ...info.xmlSchemas]);

  const facts: readonly Fact[] = $derived([
    { term: i18n.t('about.license'), value: info.license },
    { term: i18n.t('about.source'), value: info.repository, long: true, selectable: true },
  ]);

  // What the licence of a material asks to be stated, and what was changed
  // from the original where something was.
  function materialFacts(material: Material): readonly Fact[] {
    const changes: readonly Fact[] =
      material.changes === null
        ? []
        : [{ term: i18n.t('about.materials.changes'), value: material.changes }];
    return [
      { term: i18n.t('about.materials.copyright'), value: material.copyright, long: true },
      { term: i18n.t('about.license'), value: material.license },
      { term: i18n.t('about.materials.source'), value: material.source, long: true },
      ...changes,
    ];
  }

  const libraryColumns: readonly TableColumn<Library>[] = $derived([
    { heading: i18n.t('about.libraries.name'), cell: libraryNameCell, wrap: 'anywhere' },
    { heading: i18n.t('about.publications.version'), cell: libraryVersionCell },
    { heading: i18n.t('about.license'), cell: libraryLicenseCell },
  ]);

  // What was changed in the libraries the program contains as a changed copy.
  function changedLibraries(notices: LibraryNotices): readonly Fact[] {
    return notices.libraries.flatMap((library) =>
      library.changes === null ? [] : [{ term: library.name, value: library.changes }],
    );
  }

  const columns: readonly TableColumn<Publication>[] = $derived([
    { heading: i18n.t('about.publications.title'), cell: titleCell },
    { heading: i18n.t('about.publications.version'), cell: versionCell },
    { heading: i18n.t('about.publications.published'), cell: publishedCell },
    { heading: i18n.t('about.license'), cell: licenseCell },
  ]);
</script>

{#snippet libraryNameCell(library: Library)}{library.name}{/snippet}
{#snippet libraryVersionCell(library: Library)}{library.version}{/snippet}
{#snippet libraryLicenseCell(library: Library)}{library.license}{/snippet}
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
  <Note>{formatLegalNotice(info)}</Note>

  <FactList {facts} />

  <section>
    <SectionHeading text={i18n.t('about.publications.heading')} />
    <p class="about__text">{i18n.t('about.publications.text')}</p>
    <!-- A rule set and a schema may share a title. -->
    <Table {columns} rows={publications} key={byPosition} />
  </section>

  <section>
    <SectionHeading text={i18n.t('about.materials.heading')} />
    <p class="about__text">{i18n.t('about.materials.text')}</p>
    {#each info.materials as material (material.title)}
      <SubHeading text={material.title} />
      <FactList facts={materialFacts(material)} />
      <FoldedText summary={i18n.t('about.materials.licenseText')} text={material.licenseText} />
    {/each}
  </section>

  <section>
    <SectionHeading text={i18n.t('about.libraries.heading')} />
    <p class="about__text">{i18n.t('about.libraries.text')}</p>
    {#if libraries !== undefined}
      {#await libraries then notices}
        {#if notices === null}
          <Note spaced>{i18n.t('about.libraries.missing')}</Note>
        {:else}
          {@const changed = changedLibraries(notices)}
          <Table columns={libraryColumns} rows={notices.libraries} key={byPosition} />
          {#if changed.length > 0}
            <SubHeading text={i18n.t('about.libraries.changed')} />
            <FactList facts={changed} />
          {/if}
          <SubHeading text={i18n.t('about.libraries.texts')} />
          <!-- A text has nothing to tell it from the others but its place. -->
          {#each licenseTextUses(notices) as use, index (index)}
            <FoldedText summary={licenseTextSummary(use)} text={use.text} />
          {/each}
        {/if}
      {:catch error}
        <Note tone="danger" role="alert">{String(error)}</Note>
      {/await}
    {/if}
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
  .about__text {
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-sm);
  }

  .about__text--strong {
    font-weight: var(--font-weight-bold);
  }
</style>
