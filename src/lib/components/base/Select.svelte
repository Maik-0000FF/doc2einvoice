<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts" generics="Value extends string">
  // The one choice list of the interface. It shows the value its owner holds:
  // a choice is reported, and it stays shown only if the owner takes it over.
  // An owner that disables the control instead is acting on the choice; once
  // it enables the control again, a choice it did not take over is withdrawn.
  import { tick } from 'svelte';

  const {
    id,
    value,
    options,
    disabled = false,
    onchange,
  }: {
    /** Lets a label name the control. */
    id: string;
    value: Value;
    options: readonly { value: Value; label: string }[];
    disabled?: boolean;
    onchange: (value: Value) => void;
  } = $props();

  let control = $state<HTMLSelectElement>();

  function showValue(): void {
    if (control !== undefined && !disabled) {
      control.value = value;
    }
  }

  // Waits until the owner has answered the choice, by taking it over or by
  // disabling the control; one that did neither has refused it.
  async function settle(chosen: Value): Promise<void> {
    onchange(chosen);
    await tick();
    showValue();
  }

  $effect(showValue);
</script>

<select
  bind:this={control}
  {id}
  {value}
  {disabled}
  onchange={(event) => {
    // Only a script can put a value that is no option into the element.
    const chosen = options.find((option) => option.value === event.currentTarget.value);
    if (chosen === undefined) {
      showValue();
      return;
    }
    void settle(chosen.value);
  }}
>
  {#each options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
