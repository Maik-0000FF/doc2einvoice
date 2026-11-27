<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts">
  // Checking an e-invoice: the user chooses a file, the application core
  // validates it, and the result or the reason for a refusal is shown. A file
  // the program was started with is checked without being chosen.
  import { refusalMessage } from '#lib/check/summary.ts';
  import { i18n } from '#lib/i18n/i18n.svelte.ts';
  import type { CheckOutcome } from '#lib/ipc/bindings/CheckOutcome.ts';
  import type { InvoiceCheck } from '#lib/ipc/bindings/InvoiceCheck.ts';
  import type { ProfileKind } from '#lib/ipc/bindings/ProfileKind.ts';
  import type { Refusal } from '#lib/ipc/bindings/Refusal.ts';
  import { checkInvoice, checkInvoiceAgain, checkStartFile } from '#lib/ipc/check-invoice.ts';

  import Button from './base/Button.svelte';
  import Note from './base/Note.svelte';
  import CheckResult from './CheckResult.svelte';

  // While a check runs, nothing starts another one. The file dialog does not
  // block the window on every system, so the interface does it itself.
  let busy = $state(false);
  let check = $state<InvoiceCheck>();
  let refusal = $state<Refusal>();
  // Set if the core could not be asked at all; the text is its own.
  let failure = $state<string>();

  const refused = $derived(refusal === undefined ? undefined : refusalMessage(refusal));

  // A request that resolves to nothing had nothing to check and changes
  // nothing.
  async function run(request: () => Promise<CheckOutcome | null>): Promise<void> {
    if (busy) {
      return;
    }
    busy = true;
    try {
      const outcome = await request();
      if (outcome !== null) {
        show(outcome);
      }
    } catch (error) {
      failure = String(error);
    } finally {
      busy = false;
    }
  }

  // A cancelled dialog changes nothing. A refused file is explained, and the
  // result of the invoice checked before stays: the core keeps that invoice.
  function show(outcome: CheckOutcome): void {
    failure = undefined;
    switch (outcome.kind) {
      case 'cancelled':
        return;
      case 'checked':
        check = outcome.check;
        refusal = undefined;
        return;
      case 'refused':
        refusal = outcome.refusal;
    }
  }

  function again(profile: ProfileKind): void {
    void run(() => checkInvoiceAgain(profile));
  }

  // Asked once, when the window comes up.
  void run(checkStartFile);
</script>

<section class="check" aria-busy={busy}>
  <Button
    emphasis="primary"
    {busy}
    onclick={() => {
      void run(checkInvoice);
    }}
  >
    {i18n.t(busy ? 'check.running' : 'check.open')}
  </Button>

  {#if failure !== undefined}
    <Note tone="danger" size="body" long role="alert">
      {i18n.t('check.failed', { detail: failure })}
    </Note>
  {:else if refused !== undefined}
    <Note tone="danger" size="body" long role="alert">{i18n.t(refused.name, refused.values)}</Note>
  {/if}

  {#if check !== undefined}
    <CheckResult {check} {busy} onprofile={again} />
  {/if}
</section>

<style>
  .check {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    width: min(var(--content-width), 100%);
  }
</style>
