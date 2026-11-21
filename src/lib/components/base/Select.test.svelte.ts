// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// The choice list shows the value its owner holds. What the owner does with a
// choice, and what the list shows afterwards:
//
// | Owner, after the choice                        | Shown while waiting | Shown at the end |
// | ---------------------------------------------- | ------------------- | ---------------- |
// | takes it over                                  | -                   | the choice       |
// | does nothing (refuses it)                      | -                   | the old value    |
// | disables the list, enables it again unchanged  | the choice          | the old value    |
// | disables the list, takes it over, enables it   | the choice          | the choice       |
// | changes the value without a choice being made  | -                   | the new value    |
// | a value that is no option (only a script can   | -                   | not reported,    |
// | put one into the list)                         |                     | the old value    |

import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import Select from './Select.svelte';

// `mount` does not carry the type parameter of a component, so its values are
// plain strings here.
type Fruit = string;

const OPTIONS: readonly { value: Fruit; label: string }[] = [
  { value: 'apple', label: 'Apple' },
  { value: 'pear', label: 'Pear' },
];

const OLD: Fruit = 'apple';
const CHOICE: Fruit = 'pear';
const NO_OPTION = 'plum';

interface Owner {
  value: Fruit;
  disabled: boolean;
  /** Its answer to a choice, set by each case before the choice is made. */
  answer: (choice: Fruit) => void;
}

let target: HTMLElement;
let component: ReturnType<typeof mount> | undefined;
let reported: Fruit[];

// Draws the list for an owner whose value and state each case changes.
function setup(): { control: HTMLSelectElement; owner: Owner } {
  const owner: Owner = $state({ value: OLD, disabled: false, answer: () => undefined });
  component = mount(Select, {
    target,
    props: {
      id: 'fruit',
      options: OPTIONS,
      get value() {
        return owner.value;
      },
      get disabled() {
        return owner.disabled;
      },
      onchange: (choice: Fruit) => {
        reported.push(choice);
        owner.answer(choice);
      },
    },
  });
  flushSync();
  const control = target.querySelector('select');
  if (control === null) {
    throw new Error('the list was not drawn');
  }
  return { control, owner };
}

// Chooses as the user would, then waits until the list has settled.
async function choose(control: HTMLSelectElement, value: string): Promise<void> {
  control.value = value;
  // As in a browser, the event rises to the document, where Svelte listens.
  control.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
  await tick();
  flushSync();
}

beforeEach(() => {
  target = document.createElement('div');
  document.body.append(target);
  reported = [];
});

afterEach(() => {
  if (component !== undefined) {
    void unmount(component);
    component = undefined;
  }
  target.remove();
});

describe('Select', () => {
  it('shows a choice the owner takes over', async () => {
    const { control, owner } = setup();
    owner.answer = (choice) => {
      owner.value = choice;
    };
    await choose(control, CHOICE);
    expect(reported).toEqual([CHOICE]);
    expect(control.value).toBe(CHOICE);
  });

  it('withdraws a choice the owner refuses', async () => {
    const { control } = setup();
    await choose(control, CHOICE);
    expect(reported).toEqual([CHOICE]);
    expect(control.value).toBe(OLD);
  });

  it('keeps a choice while the owner acts on it, then withdraws it if not taken over', async () => {
    const { control, owner } = setup();
    owner.answer = () => {
      owner.disabled = true;
    };
    await choose(control, CHOICE);
    expect(control.disabled).toBe(true);
    expect(control.value).toBe(CHOICE);

    owner.disabled = false;
    flushSync();
    expect(control.value).toBe(OLD);
  });

  it('keeps a choice the owner takes over after acting on it', async () => {
    const { control, owner } = setup();
    owner.answer = () => {
      owner.disabled = true;
    };
    await choose(control, CHOICE);

    owner.value = CHOICE;
    owner.disabled = false;
    flushSync();
    expect(control.value).toBe(CHOICE);
  });

  it('shows a value the owner sets without a choice', () => {
    const { control, owner } = setup();
    owner.value = CHOICE;
    flushSync();
    expect(control.value).toBe(CHOICE);
  });

  it('does not report a value that is no option', async () => {
    const { control } = setup();
    await choose(control, NO_OPTION);
    expect(reported).toEqual([]);
    expect(control.value).toBe(OLD);
  });
});
