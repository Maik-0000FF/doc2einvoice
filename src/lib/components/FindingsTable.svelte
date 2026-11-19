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

  const { findings }: { findings: readonly RuleNote[] } = $props();
</script>

{#if findings.length === 0}
  <p class="findings__none">{i18n.t('check.findings.none')}</p>
{:else}
  <table class="findings">
    <thead>
      <tr>
        <th scope="col">{i18n.t('check.findings.rule')}</th>
        <th scope="col">{i18n.t('check.findings.severity')}</th>
        <th scope="col">{i18n.t('check.findings.location')}</th>
        <th scope="col">{i18n.t('check.findings.text')}</th>
      </tr>
    </thead>
    <tbody>
      <!-- A rule can be broken on several nodes, and on one node several times. -->
      {#each findings as finding, index (index)}
        <tr class="findings__row findings__row--{finding.severity}">
          <th scope="row" class="findings__rule">{finding.ruleId}</th>
          <td class="findings__severity">{i18n.t(SEVERITY_KEYS[finding.severity])}</td>
          <td class="findings__location">{finding.location}</td>
          <td>
            {finding.message}
            {#if finding.evaluationError !== null}
              <span class="findings__error">
                {i18n.t('check.findings.notEvaluated', { code: finding.evaluationError })}
              </span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .findings {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-sm);
    text-align: left;
  }

  .findings th,
  .findings td {
    padding: var(--space-xs) var(--space-sm) var(--space-xs) 0;
    border-bottom: var(--border-width) solid var(--color-border);
    font-weight: var(--font-weight-regular);
    vertical-align: top;
  }

  .findings thead th,
  .findings__none {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .findings__rule {
    white-space: nowrap;
  }

  .findings__row--fatal .findings__severity {
    color: var(--color-danger);
  }

  .findings__row--warning .findings__severity {
    color: var(--color-warning);
  }

  /* A path is one word that may be longer than its column is wide. */
  .findings__location {
    overflow-wrap: anywhere;
  }

  .findings__error {
    display: block;
    color: var(--color-danger);
  }
</style>
