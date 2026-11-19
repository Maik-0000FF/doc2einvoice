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

  let { info, open = $bindable(false) }: { info: AppInfo; open?: boolean } = $props();

  // Identifies the heading that names the dialog for assistive technology.
  const TITLE_ID = 'about-dialog-title';

  // The sentences of the disclaimer, in the order they are shown.
  const DISCLAIMER: readonly MessageKey[] = [
    'about.disclaimer.advice',
    'about.disclaimer.warranty',
    'about.disclaimer.responsibility',
    'about.disclaimer.liability',
  ];

  const publications = $derived([...info.ruleSets, ...info.xmlSchemas]);

  let dialog = $state<HTMLDialogElement>();

  // The dialog element is opened and closed by its own methods: only they make
  // it modal, keep the focus inside and close it on Escape.
  $effect(() => {
    if (dialog === undefined || dialog.open === open) {
      return;
    }
    if (open) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  });
</script>

<dialog
  bind:this={dialog}
  class="about"
  aria-labelledby={TITLE_ID}
  onclose={() => {
    open = false;
  }}
>
  <header class="about__header">
    <h2 id={TITLE_ID} class="about__title">{i18n.t('about.title', { name: info.name })}</h2>
    <p class="about__version">{i18n.t('about.version', { version: info.version })}</p>
  </header>

  <p class="about__legal">{formatLegalNotice(info)}</p>

  <dl class="about__facts">
    <dt>{i18n.t('about.license')}</dt>
    <dd>{info.license}</dd>
    <dt>{i18n.t('about.source')}</dt>
    <dd class="about__address">{info.repository}</dd>
  </dl>

  <section>
    <h3 class="about__heading">{i18n.t('about.publications.heading')}</h3>
    <p class="about__text">{i18n.t('about.publications.text')}</p>
    <table class="about__table">
      <thead>
        <tr>
          <th scope="col">{i18n.t('about.publications.title')}</th>
          <th scope="col">{i18n.t('about.publications.version')}</th>
          <th scope="col">{i18n.t('about.publications.published')}</th>
          <th scope="col">{i18n.t('about.license')}</th>
        </tr>
      </thead>
      <tbody>
        {#each publications as publication (publication.title)}
          <tr>
            <th scope="row">{publication.title}</th>
            <td>{publication.version ?? i18n.t('about.publications.notStated')}</td>
            <td>{i18n.date(publication.published)}</td>
            <td>{publication.license ?? i18n.t('about.publications.notStated')}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  <section>
    <h3 class="about__heading">{i18n.t('about.disclaimer.heading')}</h3>
    <p class="about__text about__text--strong">{i18n.t('about.disclaimer.risk')}</p>
    {#each DISCLAIMER as sentence (sentence)}
      <p class="about__text">{i18n.t(sentence)}</p>
    {/each}
  </section>

  <form method="dialog" class="about__actions">
    <button class="about__close">{i18n.t('about.close')}</button>
  </form>
</dialog>

<style>
  .about {
    width: min(var(--dialog-width), calc(100vw - 2 * var(--space-lg)));
    max-height: calc(100vh - 2 * var(--space-lg));
    padding: var(--space-lg);
    overflow-y: auto;
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
  }

  .about::backdrop {
    background: var(--color-backdrop);
  }

  .about__header {
    margin-bottom: var(--space-md);
  }

  .about__title {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .about__heading {
    margin-top: var(--space-lg);
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-bold);
  }

  .about__version,
  .about__legal {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .about__facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-md);
    margin: var(--space-md) 0 0;
    font-size: var(--font-size-sm);
  }

  .about__facts dt {
    color: var(--color-text-muted);
  }

  .about__facts dd {
    margin: 0;
  }

  /* An address is one word that may be longer than the dialog is wide. */
  .about__address {
    overflow-wrap: anywhere;
    user-select: all;
  }

  .about__text {
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-sm);
  }

  .about__text--strong {
    font-weight: var(--font-weight-bold);
  }

  .about__table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
    text-align: left;
  }

  .about__table th,
  .about__table td {
    padding: var(--space-xs) var(--space-sm) var(--space-xs) 0;
    border-bottom: var(--border-width) solid var(--color-border);
    font-weight: var(--font-weight-regular);
    vertical-align: top;
  }

  .about__table thead th {
    color: var(--color-text-muted);
  }

  .about__actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-lg);
  }

  .about__close {
    padding: var(--space-xs) var(--space-md);
    border: var(--border-width) solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    cursor: pointer;
  }
</style>
