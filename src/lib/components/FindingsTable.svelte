<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // The rules an invoice does not satisfy: the identifier of each rule, how
  // serious it is, where in the document it was checked, and the wording of
  // the rule as its publisher wrote it.
  import { SEVERITY_KEYS } from '#lib/check/summary.ts';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import type { RuleNote } from '#lib/ipc/bindings/RuleNote.ts';

  import Note from './base/Note.svelte';
  import Table from './base/Table.svelte';
  import { byPosition, type TableColumn } from './base/table.ts';

  const { findings }: { findings: readonly RuleNote[] } = $props();

  const columns: readonly TableColumn<RuleNote>[] = $derived([
    { heading: i18n.t('check.findings.rule'), cell: ruleCell, wrap: 'never' },
    { heading: i18n.t('check.findings.severity'), cell: severityCell },
    { heading: i18n.t('check.findings.location'), cell: locationCell, wrap: 'anywhere' },
    { heading: i18n.t('check.findings.text'), cell: textCell },
  ]);
</script>

{#snippet ruleCell(finding: RuleNote)}{finding.ruleId}{/snippet}
{#snippet severityCell(finding: RuleNote)}
  <span class="findings__severity--{finding.severity}">
    {i18n.t(SEVERITY_KEYS[finding.severity])}
  </span>
{/snippet}
{#snippet locationCell(finding: RuleNote)}{finding.location}{/snippet}
{#snippet textCell(finding: RuleNote)}
  {finding.message}
  {#if finding.evaluationError !== null}
    <span class="findings__error">
      {i18n.t('check.findings.notEvaluated', { code: finding.evaluationError })}
    </span>
  {/if}
{/snippet}

{#if findings.length === 0}
  <Note>{i18n.t('check.findings.none')}</Note>
{:else}
  <!-- A rule can be broken on several nodes, and on one node several times. -->
  <Table {columns} rows={findings} key={byPosition} />
{/if}

<style>
  .findings__severity--fatal {
    color: var(--color-danger);
  }

  .findings__severity--warning {
    color: var(--color-warning);
  }

  .findings__error {
    display: block;
    color: var(--color-danger);
  }
</style>
