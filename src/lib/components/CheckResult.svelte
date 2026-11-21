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
  import type { SchemaNote } from '#lib/ipc/bindings/SchemaNote.ts';

  import FactList from './base/FactList.svelte';
  import type { Fact } from './base/facts.ts';
  import Note from './base/Note.svelte';
  import SectionHeading from './base/SectionHeading.svelte';
  import Select from './base/Select.svelte';
  import SubHeading from './base/SubHeading.svelte';
  import Table from './base/Table.svelte';
  import { byPosition, type TableColumn } from './base/table.ts';
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
  const profileId = $props.id();

  const counts = $derived(countBySeverity(check));

  const profiles = $derived(
    PROFILES.map((profile) => ({ value: profile, label: i18n.t(PROFILE_KEYS[profile]) })),
  );

  const schemaColumns: readonly TableColumn<SchemaNote>[] = $derived([
    { heading: i18n.t('check.schema.code'), cell: codeCell },
    { heading: i18n.t('check.findings.location'), cell: locationCell, wrap: 'anywhere' },
    { heading: i18n.t('check.schema.text'), cell: textCell },
  ]);

  const facts: readonly Fact[] = $derived([
    { term: i18n.t('check.file'), value: check.fileName, long: true },
    { term: i18n.t('check.syntax'), value: i18n.t(SYNTAX_KEYS[check.syntax]) },
    { term: i18n.t('check.profile'), value: profileControl, labelFor: profileId },
  ]);
</script>

{#snippet codeCell(finding: SchemaNote)}{finding.code}{/snippet}
{#snippet locationCell(finding: SchemaNote)}{finding.location}{/snippet}
{#snippet textCell(finding: SchemaNote)}{finding.message}{/snippet}

<!-- The control states what the result below it was checked as. A choice that
     led to no new result (the check failed, or was refused) is withdrawn once
     no check is running. -->
{#snippet profileControl()}
  <Select
    id={profileId}
    value={check.profile}
    options={profiles}
    disabled={busy}
    onchange={onprofile}
  />
{/snippet}

<section class="result">
  <p class="result__verdict" class:result__verdict--fails={!check.conforms} role="status">
    {i18n.t(check.conforms ? 'check.verdict.conforms' : 'check.verdict.fails')}
  </p>
  <Note spaced>{i18n.t('check.verdict.scope')}</Note>

  <FactList {facts} />

  <Note spaced long>
    {check.specificationIdentifier === null
      ? i18n.t('check.identifier.none')
      : i18n.t('check.identifier.stated', { identifier: check.specificationIdentifier })}
  </Note>
  {#if check.ruleCoverage === 'notApplied'}
    <Note tone="warning" spaced>
      {i18n.t('check.identifier.notApplied', { profile: i18n.t(PROFILE_KEYS[check.profile]) })}
    </Note>
  {/if}

  <SectionHeading text={i18n.t('check.schema.heading')} />
  {#if check.schemaFindings.length === 0}
    <Note spaced>{i18n.t('check.schema.valid')}</Note>
  {:else}
    <Note tone="danger" spaced>{i18n.t('check.schema.invalid')}</Note>
    <Table columns={schemaColumns} rows={check.schemaFindings} key={byPosition} />
  {/if}

  <SectionHeading text={i18n.t('check.ruleSets.heading')} />
  <Note spaced>{i18n.t('check.counts', counts)}</Note>
  {#each check.ruleSets as ruleSet (ruleSet.publication.title)}
    <article class="result__rule-set">
      <SubHeading text={ruleSet.publication.title} />
      {#if ruleSet.publication.version !== null}
        <Note spaced>
          {i18n.t('check.ruleSets.release', {
            version: ruleSet.publication.version,
            published: i18n.date(ruleSet.publication.published),
          })}
        </Note>
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

  .result__rule-set {
    margin-top: var(--space-md);
  }
</style>
