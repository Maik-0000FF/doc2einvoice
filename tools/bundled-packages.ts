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
// What the name of a module starts with when a tool of the build made it up.
// Such a module is no file, even where its name looks like a path.
const MADE_UP_MARK = '\0';

/**
 * The modules the tools of the build make up and put into the bundle, by what
 * their name starts with after the mark, each with the package whose code it
 * is. A made-up module that is not named here stops the build: its code would
 * be in the bundle without its package being in the list.
 */
const MADE_UP_MODULES: Readonly<Record<string, string>> = {
  // The helpers the bundler adds to the code it joins.
  'rolldown/': 'rolldown',
  // The helper that loads the parts of the bundle a page needs.
  'vite/': 'vite',
};

/** A module of the bundle as the list sees it. */
export type ModuleOrigin =
  | { kind: 'package'; directory: string }
  /** Code of this repository, or of nothing that has a package. */
  | { kind: 'own' }
  /** Made up by a tool that is not known here. */
  | { kind: 'unknown' };

/**
 * Where a module of the bundle comes from. The directory of a package is
 * relative to the root and written with forward slashes.
 *
 * | Module                                        | Origin                                   |
 * | --------------------------------------------- | ---------------------------------------- |
 * | `/r/node_modules/svelte/src/index.js`         | package `node_modules/svelte`            |
 * | `/r/node_modules/@scope/name/index.js`        | package `node_modules/@scope/name`       |
 * | `/r/node_modules/a/node_modules/b/index.js`   | package `node_modules/a/node_modules/b`  |
 * | `/r/node_modules/svelte/src/index.js?v=1`     | package `node_modules/svelte`            |
 * | `/r/src/lib/app.ts`                           | own                                      |
 * | `/elsewhere/node_modules/svelte/index.js`     | own: outside the root, no package of it  |
 * | `/r/node_modules/@scope`                      | own: no package named                    |
 * | `/r/node_modules`                             | own: no package named                    |
 * | empty                                         | own                                      |
 * | made up, its start named in the table         | the package named there                  |
 * | made up, not named in the table               | unknown                                  |
 * | made up, with the path of a package after it  | unknown: the path is not asked           |
 */
export function moduleOrigin(
  module: string,
  root: string,
  madeUp: Readonly<Record<string, string>> = MADE_UP_MODULES,
): ModuleOrigin {
  if (module.startsWith(MADE_UP_MARK)) {
    const name = module.slice(MADE_UP_MARK.length);
    const start = Object.keys(madeUp).find((known) => name.startsWith(known));
    const packageName = start === undefined ? undefined : madeUp[start];
    return packageName === undefined
      ? { kind: 'unknown' }
      : { kind: 'package', directory: `${PACKAGES_DIRECTORY}/${packageName}` };
  }
  const relative = path.relative(root, module);
  if (module === '' || relative.startsWith('..') || path.isAbsolute(relative)) {
    return { kind: 'own' };
  }
  const parts = relative.split(path.sep);
  const start = parts.lastIndexOf(PACKAGES_DIRECTORY);
  if (start === -1) {
    return { kind: 'own' };
  }
  const first = parts[start + 1];
  const length = first?.startsWith(SCOPE_MARK) === true ? 2 : 1;
  const named = parts.slice(start + 1, start + 1 + length);
  // A file directly in the directory of the packages, or in that of a scope,
  // belongs to no package.
  if (named.length < length || parts.length <= start + 1 + length) {
    return { kind: 'own' };
  }
  return { kind: 'package', directory: [...parts.slice(0, start + 1), ...named].join('/') };
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
      const unknown = new Set<string>();
      for (const chunk of Object.values(bundle)) {
        if (chunk.type !== 'chunk') {
          continue;
        }
        for (const [module, rendered] of Object.entries(chunk.modules)) {
          // A module of which nothing is left in the bundle is not contained.
          if (rendered.renderedLength === 0) {
            continue;
          }
          const origin = moduleOrigin(module, root);
          if (origin.kind === 'package') {
            directories.add(origin.directory);
          } else if (origin.kind === 'unknown') {
            unknown.add(JSON.stringify(module));
          }
        }
      }
      if (unknown.size > 0) {
        this.error(
          `The bundle holds code of modules a tool made up, and no package is named for them: ${[...unknown].sort().join(', ')}. Name the package in tools/bundled-packages.ts.`,
        );
      }
      const file = path.join(root, list);
      mkdirSync(path.dirname(file), { recursive: true });
      writeFileSync(file, `${JSON.stringify([...directories].sort())}\n`);
    },
  };
}
