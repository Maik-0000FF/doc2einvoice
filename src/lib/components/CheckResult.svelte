<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The result of checking one invoice: the verdict, what the invoice was
  // checked as and against which releases, and what it does not satisfy.
  import { countBySeverity, PROFILE_KEYS, PROFILES, SYNTAX_KEYS } from '#lib/check/summary.ts';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import type { InvoiceCheck } from '#lib/ipc/bindings/InvoiceCheck.ts';
  import type { ProfileKind } from '#lib/ipc/bindings/ProfileKind.ts';

  import FindingsTable from './FindingsTable.svelte';

  const {
    check,
    busy,
    onprofile,
  }: {
    check: InvoiceCheck;
    /** Whether a check is running; the profile cannot be changed meanwhile. */
    busy: boolean;
    /** Asks for the invoice to be checked again as another profile. */
    onprofile: (profile: ProfileKind) => void;
  } = $props();

  // Identifies the control its label belongs to.
  const PROFILE_ID = 'check-result-profile';

  const counts = $derived(countBySeverity(check));

  let profileControl = $state<HTMLSelectElement>();

  // The control states what the result below it was checked as. A choice that
  // led to no new result (the check failed, or was refused) must not stay in
  // it, so once no check is running it is set to the profile of the result.
  $effect(() => {
    if (profileControl !== undefined && !busy) {
      profileControl.value = check.profile;
    }
  });
</script>

<section class="result">
  <p class="result__verdict" class:result__verdict--fails={!check.conforms} role="status">
    {i18n.t(check.conforms ? 'check.verdict.conforms' : 'check.verdict.fails')}
  </p>
  <p class="result__note">{i18n.t('check.verdict.scope')}</p>

  <dl class="result__facts">
    <dt>{i18n.t('check.file')}</dt>
    <dd class="result__file">{check.fileName}</dd>
    <dt>{i18n.t('check.syntax')}</dt>
    <dd>{i18n.t(SYNTAX_KEYS[check.syntax])}</dd>
    <dt><label for={PROFILE_ID}>{i18n.t('check.profile')}</label></dt>
    <dd>
      <select
        bind:this={profileControl}
        id={PROFILE_ID}
        value={check.profile}
        disabled={busy}
        onchange={(event) => {
          onprofile(event.currentTarget.value as ProfileKind);
        }}
      >
        {#each PROFILES as profile (profile)}
          <option value={profile}>{i18n.t(PROFILE_KEYS[profile])}</option>
        {/each}
      </select>
    </dd>
  </dl>

  <p class="result__note result__identifier">
    {check.specificationIdentifier === null
      ? i18n.t('check.identifier.none')
      : i18n.t('check.identifier.stated', { identifier: check.specificationIdentifier })}
  </p>
  {#if check.ruleCoverage === 'notApplied'}
    <p class="result__note result__note--warning">
      {i18n.t('check.identifier.notApplied', { profile: i18n.t(PROFILE_KEYS[check.profile]) })}
    </p>
  {/if}

  <h3 class="result__heading">{i18n.t('check.schema.heading')}</h3>
  {#if check.schemaFindings.length === 0}
    <p class="result__note">{i18n.t('check.schema.valid')}</p>
  {:else}
    <p class="result__note result__note--danger">{i18n.t('check.schema.invalid')}</p>
    <table class="result__schema">
      <thead>
        <tr>
          <th scope="col">{i18n.t('check.schema.code')}</th>
          <th scope="col">{i18n.t('check.findings.location')}</th>
          <th scope="col">{i18n.t('check.schema.text')}</th>
        </tr>
      </thead>
      <tbody>
        {#each check.schemaFindings as finding, index (index)}
          <tr>
            <th scope="row">{finding.code}</th>
            <td class="result__location">{finding.location}</td>
            <td>{finding.message}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h3 class="result__heading">{i18n.t('check.ruleSets.heading')}</h3>
  <p class="result__note">{i18n.t('check.counts', counts)}</p>
  {#each check.ruleSets as ruleSet (ruleSet.publication.title)}
    <article class="result__rule-set">
      <h4 class="result__title">{ruleSet.publication.title}</h4>
      {#if ruleSet.publication.version !== null}
        <p class="result__note">
          {i18n.t('check.ruleSets.release', {
            version: ruleSet.publication.version,
            published: i18n.date(ruleSet.publication.published),
          })}
        </p>
      {/if}
      <FindingsTable findings={ruleSet.findings} />
    </article>
  {/each}
</section>

<style>
  .result {
    width: 100%;
    text-align: left;
  }

  .result__verdict {
    color: var(--color-success);
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .result__verdict--fails {
    color: var(--color-danger);
  }

  .result__note {
    margin-bottom: var(--space-sm);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .result__note--danger {
    color: var(--color-danger);
  }

  .result__note--warning {
    color: var(--color-warning);
  }

  .result__facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-xs) var(--space-md);
    margin: var(--space-md) 0;
    font-size: var(--font-size-sm);
  }

  .result__facts dt {
    color: var(--color-text-muted);
  }

  .result__facts dd {
    margin: 0;
  }

  /* A file name or an identifier is one word that may be longer than the
     window is wide. */
  .result__file,
  .result__identifier,
  .result__location {
    overflow-wrap: anywhere;
  }

  .result__heading {
    margin-top: var(--space-lg);
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-bold);
  }

  .result__rule-set {
    margin-top: var(--space-md);
  }

  .result__title {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
  }

  .result__schema {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
    text-align: left;
  }

  .result__schema th,
  .result__schema td {
    padding: var(--space-xs) var(--space-sm) var(--space-xs) 0;
    border-bottom: var(--border-width) solid var(--color-border);
    font-weight: var(--font-weight-regular);
    vertical-align: top;
  }

  .result__schema thead th {
    color: var(--color-text-muted);
  }
</style>
