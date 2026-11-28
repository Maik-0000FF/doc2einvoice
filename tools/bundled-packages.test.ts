// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { packageDirectory } from './bundled-packages.ts';

const ROOT = '/r';

describe('the directory of the package a module belongs to', () => {
  it.each([
    ['/r/node_modules/svelte/src/index.js', 'node_modules/svelte'],
    ['/r/node_modules/@scope/name/index.js', 'node_modules/@scope/name'],
    ['/r/node_modules/a/node_modules/b/index.js', 'node_modules/a/node_modules/b'],
    ['/r/node_modules/svelte/src/index.js?v=1', 'node_modules/svelte'],
    ['/r/src/lib/app.ts', undefined],
    ['\0virtual:module', undefined],
    ['/elsewhere/node_modules/svelte/index.js', undefined],
    ['/r/node_modules/@scope', undefined],
    ['/r/node_modules', undefined],
  ])('of %s is %s', (module, expected) => {
    expect(packageDirectory(module, ROOT)).toBe(expected);
  });
});
