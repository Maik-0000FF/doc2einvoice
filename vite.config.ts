// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import path from 'node:path';
import process from 'node:process';

import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

import manifest from './package.json' with { type: 'json' };
import { bundledPackages } from './tools/bundled-packages.ts';

// The desktop shell loads the dev server from a fixed address, so the port must
// match `build.devUrl` in src-tauri/tauri.conf.json (JSON cannot import it).
const DEV_SERVER_PORT = 1420;
const HMR_PORT = 1421;
const LOOPBACK_HOST = '127.0.0.1';

// Tests of the building blocks of the interface; they need a document.
const COMPONENT_TESTS = 'src/lib/components/**/*.test.svelte.ts';

// Where the build writes down the packages in the bundle of the window. The
// manifest states the place once, for this build and for the generator of the
// list of libraries, which reads the file.
const BUNDLED_PACKAGES_LIST = path.join(
  manifest.config.noticesDirectory,
  manifest.config.bundledPackagesList,
);

// Set by the Tauri CLI only when developing against a mobile device.
const remoteDevHost = process.env['TAURI_DEV_HOST'];

export default defineConfig({
  // The desktop shell has no server to render pages, so the frontend is built
  // as a static single-page application with index.html as the fallback.
  plugins: [
    sveltekit({ adapter: adapter({ fallback: 'index.html' }) }),
    bundledPackages(BUNDLED_PACKAGES_LIST),
  ],

  // Keep Rust compiler errors visible in the shared terminal.
  clearScreen: false,

  server: {
    port: DEV_SERVER_PORT,
    strictPort: true,
    host: remoteDevHost ?? LOOPBACK_HOST,
    ...(remoteDevHost === undefined
      ? {}
      : { hmr: { protocol: 'ws', host: remoteDevHost, port: HMR_PORT } }),
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },

  test: {
    projects: [
      {
        extends: true,
        test: {
          name: 'logic',
          include: ['src/**/*.test.ts', 'tools/**/*.test.ts'],
        },
      },
      {
        // Components are mounted into a document as the window would, with
        // the browser build of Svelte.
        extends: true,
        resolve: { conditions: ['browser'] },
        test: {
          name: 'components',
          include: [COMPONENT_TESTS],
          environment: 'jsdom',
        },
      },
    ],
  },
});
