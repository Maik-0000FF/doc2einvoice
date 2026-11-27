// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// The about dialog names the material of other authors the program contains,
// each with what its licence asks for:
//
// | Material                  | Shown                                              |
// | ------------------------- | -------------------------------------------------- |
// | unchanged                 | title, copyright notice, licence, source, its text |
// | changed from the original | the same, and what was changed                     |
// | none                      | the section, without an entry                      |

import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '#lib/i18n/i18n.svelte.ts';
import type { AppInfo } from '#lib/ipc/bindings/AppInfo.ts';
import type { Material } from '#lib/ipc/bindings/Material.ts';

import AboutDialog from './AboutDialog.svelte';

const UNCHANGED: Material = {
  title: 'Sample Typeface',
  copyright: 'Copyright (c) 2001 The Sample Authors',
  license: 'OFL-1.1',
  licenseText: 'The terms of the sample typeface.',
  source: 'https://example.org/typeface',
  changes: null,
};

const CHANGED: Material = {
  title: 'Sample Table',
  copyright: 'Copyright (c) 2002 The Table Authors',
  license: 'CC-BY-4.0',
  licenseText: 'The terms of the sample table.',
  source: 'https://example.org/table',
  changes: 'Reduced to two columns.',
};

function info(materials: Material[]): AppInfo {
  return {
    name: 'sample',
    version: '1.2.3',
    copyrightHolder: 'Sample Holder',
    license: 'GPL-3.0-or-later',
    repository: 'https://example.org/sample',
    ruleSets: [],
    xmlSchemas: [],
    materials,
  };
}

let target: HTMLElement;
let component: ReturnType<typeof mount> | undefined;

// Draws the dialog and gives everything it says.
function shown(materials: Material[]): string {
  component = mount(AboutDialog, { target, props: { info: info(materials) } });
  flushSync();
  return target.textContent;
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

describe('AboutDialog', () => {
  it('names a material with what its licence asks for', () => {
    const text = shown([UNCHANGED]);

    for (const stated of [
      UNCHANGED.title,
      UNCHANGED.copyright,
      UNCHANGED.license,
      UNCHANGED.source,
      UNCHANGED.licenseText,
    ]) {
      expect(text).toContain(stated);
    }
  });

  it('says what was changed where something was', () => {
    const text = shown([CHANGED]);

    expect(text).toContain(i18n.t('about.materials.changes'));
    expect(text).toContain(CHANGED.changes);
  });

  it('says nothing of changes where nothing was changed', () => {
    expect(shown([UNCHANGED])).not.toContain(i18n.t('about.materials.changes'));
  });

  it('holds the licence text of every material folded', () => {
    shown([UNCHANGED, CHANGED]);
    const folds = [...target.querySelectorAll('details')];

    expect(folds).toHaveLength(2);
    expect(folds.every((fold) => !fold.open)).toBe(true);
  });

  it('shows the section without an entry where there is no material', () => {
    const text = shown([]);

    expect(text).toContain(i18n.t('about.materials.heading'));
    expect(target.querySelectorAll('details')).toHaveLength(0);
  });
});
