// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// Writes down which packages the build put into the interface: the
// directories of the packages whose code is part of the bundle the window
// loads. `cargo xtask notices` reads the list and names those packages, with
// their licence texts, among the libraries the application contains.
//
// What a package only contributes to the build, or to the pages rendered
// while building, is not in the bundle and not in the list.

import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import type { Plugin } from 'vite';

// The directory packages are installed in.
const PACKAGES_DIRECTORY = 'node_modules';
// The part of the build whose bundle the window loads.
const WINDOW_ENVIRONMENT = 'client';
// What the name of a package starts with when it belongs to a scope and so
// takes two directories.
const SCOPE_MARK = '@';
// What the name of a module starts with when a plugin made it up: such a
// module is no file, even where a path follows.
const MADE_UP_MARK = '\0';

/**
 * The directory of the package a module belongs to, relative to the root and
 * with forward slashes, or nothing for a module that is no file of a package.
 *
 * | Module                                              | Directory                       |
 * | --------------------------------------------------- | ------------------------------- |
 * | `/r/node_modules/svelte/src/index.js`               | `node_modules/svelte`           |
 * | `/r/node_modules/@scope/name/index.js`              | `node_modules/@scope/name`      |
 * | `/r/node_modules/a/node_modules/b/index.js`         | `node_modules/a/node_modules/b` |
 * | `/r/node_modules/svelte/src/index.js?v=1`           | `node_modules/svelte`           |
 * | `/r/src/lib/app.ts`                                 | nothing                         |
 * | `\0virtual:module`                                  | nothing: made up by a plugin    |
 * | `\0/r/node_modules/svelte/src/index.js`             | nothing: made up by a plugin    |
 * | `/elsewhere/node_modules/svelte/index.js`           | nothing: outside the root       |
 * | `/r/node_modules/@scope`                            | nothing: no package named       |
 * | `/r/node_modules`                                   | nothing: no package named       |
 * | empty                                               | nothing                         |
 */
export function packageDirectory(module: string, root: string): string | undefined {
  if (module === '' || module.startsWith(MADE_UP_MARK)) {
    return undefined;
  }
  const relative = path.relative(root, module);
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    return undefined;
  }
  const parts = relative.split(path.sep);
  const start = parts.lastIndexOf(PACKAGES_DIRECTORY);
  if (start === -1) {
    return undefined;
  }
  const first = parts[start + 1];
  const length = first?.startsWith(SCOPE_MARK) === true ? 2 : 1;
  const named = parts.slice(start + 1, start + 1 + length);
  // A file directly in the directory of the packages, or in that of a scope,
  // belongs to no package.
  if (named.length < length || parts.length <= start + 1 + length) {
    return undefined;
  }
  return [...parts.slice(0, start + 1), ...named].join('/');
}

/**
 * Writes the list of the packages in the bundle of the window when it is
 * built. `list` is the file the list is written to, relative to the root.
 *
 * The list is written once the bundle is: a build that fails leaves the list
 * of the last one that succeeded.
 */
export function bundledPackages(list: string): Plugin {
  let root = '';
  return {
    name: 'bundled-packages',
    apply: 'build',
    configResolved(config) {
      root = config.root;
    },
    writeBundle(_options, bundle) {
      if (this.environment.name !== WINDOW_ENVIRONMENT) {
        return;
      }
      const directories = new Set<string>();
      for (const chunk of Object.values(bundle)) {
        if (chunk.type !== 'chunk') {
          continue;
        }
        for (const [module, rendered] of Object.entries(chunk.modules)) {
          // A module of which nothing is left in the bundle is not contained.
          const directory =
            rendered.renderedLength > 0 ? packageDirectory(module, root) : undefined;
          if (directory !== undefined) {
            directories.add(directory);
          }
        }
      }
      const file = path.join(root, list);
      mkdirSync(path.dirname(file), { recursive: true });
      writeFileSync(file, `${JSON.stringify([...directories].sort())}\n`);
    },
  };
}
