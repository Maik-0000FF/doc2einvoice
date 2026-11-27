// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// The folded text names a long text in one line and holds the text itself,
// closed until the line is opened:
//
// | Text                         | Shown                                      |
// | ---------------------------- | ------------------------------------------ |
// | two paragraphs               | both, the empty line between them kept     |
// | markup, as `<b>bold</b>`     | as it is written, not as an element        |
// | nothing                      | the line that names it, and an empty text  |

import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import FoldedText from './FoldedText.svelte';

const SUMMARY = 'Licence text';
const PARAGRAPHS = 'First paragraph.\n\nSecond paragraph.';
const MARKUP = '<b>bold</b>';

let target: HTMLElement;
let component: ReturnType<typeof mount> | undefined;

// Draws the folded text and gives what it is made of.
function setup(text: string): {
  fold: HTMLDetailsElement;
  summary: Element;
  shown: HTMLPreElement;
} {
  component = mount(FoldedText, { target, props: { summary: SUMMARY, text } });
  flushSync();
  const fold = target.querySelector('details');
  const summary = fold?.querySelector('summary');
  const shown = fold?.querySelector('pre');
  if (fold === null || summary == null || shown == null) {
    throw new Error('the folded text was not drawn');
  }
  return { fold, summary, shown };
}

beforeEach(() => {
  target = document.createElement('div');
  document.body.append(target);
});

afterEach(() => {
  if (component !== undefined) {
    void unmount(component);
    component = undefined;
  }
  target.remove();
});

describe('FoldedText', () => {
  it('is closed until it is opened, and named by its line', () => {
    const { fold, summary } = setup(PARAGRAPHS);

    expect(fold.open).toBe(false);
    expect(summary.textContent).toBe(SUMMARY);
  });

  it('keeps the lines and paragraphs of the text', () => {
    expect(setup(PARAGRAPHS).shown.textContent).toBe(PARAGRAPHS);
  });

  it('shows markup as it is written', () => {
    const { shown } = setup(MARKUP);

    expect(shown.textContent).toBe(MARKUP);
    expect(shown.children).toHaveLength(0);
  });

  it('shows an empty text as nothing', () => {
    expect(setup('').shown.textContent).toBe('');
  });

  it('lets the keyboard reach the text, to scroll it', () => {
    expect(setup(PARAGRAPHS).shown.tabIndex).toBe(0);
  });
});
