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
//
// and the libraries the build contains:
//
// | List of libraries         | Shown                                              |
// | ------------------------- | -------------------------------------------------- |
// | not asked for yet         | the section, without a list                        |
// | the build carries none    | that it carries none                               |
// | a list                    | every library with version and licence, and every  |
// |                           | text they pass on folded, named by its libraries   |
// | a list with a changed one | the same, and what was changed                     |

import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { i18n } from '#lib/i18n/i18n.svelte.ts';
import type { AppInfo } from '#lib/ipc/bindings/AppInfo.ts';
import type { LibraryNotices } from '#lib/ipc/bindings/LibraryNotices.ts';
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

const LIBRARIES: LibraryNotices = {
  texts: ['The terms of the sample libraries.', 'The notice of the other sample library.'],
  libraries: [
    { name: 'sample-reader', version: '1.0.0', license: 'MIT', texts: [0], changes: null },
    {
      name: 'sample-writer',
      version: '2.1.0',
      license: 'MIT OR Zlib',
      texts: [0, 1],
      changes: null,
    },
  ],
};

const CHANGED_LIBRARY = {
  name: 'sample-copy',
  version: '0.3.0',
  license: 'MIT',
  texts: [0],
  changes: 'Reads one more form.',
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

// Draws the dialog with the answer to its question for the libraries and
// gives everything it says once the answer is shown.
async function shownWithLibraries(libraries: LibraryNotices | null): Promise<string> {
  component = mount(AboutDialog, {
    target,
    props: { info: info([]), libraries: Promise.resolve(libraries) },
  });
  await tick();
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

  it('shows the section of the libraries without a list before it is asked for', () => {
    const text = shown([]);

    expect(text).toContain(i18n.t('about.libraries.heading'));
    expect(text).not.toContain(i18n.t('about.libraries.missing'));
    expect(text).not.toContain(i18n.t('about.libraries.texts'));
  });

  it('says that a build carries no list of its libraries', async () => {
    const text = await shownWithLibraries(null);

    expect(text).toContain(i18n.t('about.libraries.missing'));
    expect(text).not.toContain(i18n.t('about.libraries.texts'));
  });

  it('names every library with its version and its licence', async () => {
    const text = await shownWithLibraries(LIBRARIES);

    for (const library of LIBRARIES.libraries) {
      expect(text).toContain(library.name);
      expect(text).toContain(library.version);
      expect(text).toContain(library.license);
    }
    expect(text).not.toContain(i18n.t('about.libraries.missing'));
  });

  it('holds every text the libraries pass on folded, named by its libraries', async () => {
    await shownWithLibraries(LIBRARIES);
    const folds = [...target.querySelectorAll('details')];

    expect(folds.map((fold) => fold.querySelector('summary')?.textContent)).toEqual([
      'sample-reader 1.0.0, sample-writer 2.1.0',
      'sample-writer 2.1.0',
    ]);
    expect(folds.map((fold) => fold.querySelector('pre')?.textContent)).toEqual(LIBRARIES.texts);
    expect(folds.every((fold) => !fold.open)).toBe(true);
  });

  it('says what was changed in a library where something was', async () => {
    const text = await shownWithLibraries({
      texts: LIBRARIES.texts,
      libraries: [...LIBRARIES.libraries, CHANGED_LIBRARY],
    });

    expect(text).toContain(i18n.t('about.libraries.changed'));
    expect(text).toContain(CHANGED_LIBRARY.changes);
  });

  it('says nothing of changed libraries where none was changed', async () => {
    const text = await shownWithLibraries(LIBRARIES);

    expect(text).not.toContain(i18n.t('about.libraries.changed'));
  });

  it('shows the section without an entry where there is no material', () => {
    const text = shown([]);

    expect(text).toContain(i18n.t('about.materials.heading'));
    expect(target.querySelectorAll('details')).toHaveLength(0);
  });
});
