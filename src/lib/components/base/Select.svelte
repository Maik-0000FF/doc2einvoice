<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->
<script lang="ts" generics="Value extends string">
  // The one choice list of the interface. It shows the value its owner holds:
  // a choice is reported, and it stays shown only if the owner takes it over.
  // While the control is disabled the choice is being acted on; once it is
  // enabled again, a choice the owner did not take over is withdrawn.

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

  $effect(() => {
    if (control !== undefined && !disabled) {
      control.value = value;
    }
  });
</script>

<select
  bind:this={control}
  {id}
  {value}
  {disabled}
  onchange={(event) => {
    const chosen = options.find((option) => option.value === event.currentTarget.value);
    if (chosen !== undefined) {
      onchange(chosen.value);
    }
  }}
>
  {#each options as option (option.value)}
    <option value={option.value}>{option.label}</option>
  {/each}
</select>
