// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { type ModuleOrigin, moduleOrigin } from './bundled-packages.ts';

const ROOT = '/r';
const OWN: ModuleOrigin = { kind: 'own' };
const UNKNOWN: ModuleOrigin = { kind: 'unknown' };
// The made-up modules known in these tests.
const MADE_UP = { 'helper/': 'helper-package', 'tool:': '@scope/tool' };

function inPackage(directory: string): ModuleOrigin {
  return { kind: 'package', directory };
}

describe('where a module of the bundle comes from', () => {
  it.each([
    ['/r/node_modules/svelte/src/index.js', inPackage('node_modules/svelte')],
    ['/r/node_modules/@scope/name/index.js', inPackage('node_modules/@scope/name')],
    ['/r/node_modules/a/node_modules/b/index.js', inPackage('node_modules/a/node_modules/b')],
    ['/r/node_modules/svelte/src/index.js?v=1', inPackage('node_modules/svelte')],
    ['/r/src/lib/app.ts', OWN],
    ['/elsewhere/node_modules/svelte/index.js', OWN],
    ['/r/node_modules/@scope', OWN],
    ['/r/node_modules', OWN],
    ['', OWN],
    ['\0helper/runtime.js', inPackage('node_modules/helper-package')],
    ['\0tool:part', inPackage('node_modules/@scope/tool')],
    ['\0virtual:module', UNKNOWN],
    ['\0', UNKNOWN],
    ['\0/r/node_modules/svelte/src/index.js', UNKNOWN],
  ])('of %j', (module, expected) => {
    expect(moduleOrigin(module, ROOT, MADE_UP)).toEqual(expected);
  });

  it('knows the modules the tools of this build make up', () => {
    expect(moduleOrigin('\0vite/preload-helper.js', ROOT)).toEqual(inPackage('node_modules/vite'));
    expect(moduleOrigin('\0rolldown/runtime.js', ROOT)).toEqual(inPackage('node_modules/rolldown'));
  });
});
