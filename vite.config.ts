// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import process from 'node:process';

import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

// The desktop shell loads the dev server from a fixed address, so the port must
// match `build.devUrl` in src-tauri/tauri.conf.json (JSON cannot import it).
const DEV_SERVER_PORT = 1420;
const HMR_PORT = 1421;
const LOOPBACK_HOST = '127.0.0.1';

// Tests of the building blocks of the interface; they need a document.
const COMPONENT_TESTS = 'src/lib/components/**/*.test.svelte.ts';

// Set by the Tauri CLI only when developing against a mobile device.
const remoteDevHost = process.env['TAURI_DEV_HOST'];

export default defineConfig({
  // The desktop shell has no server to render pages, so the frontend is built
  // as a static single-page application with index.html as the fallback.
  plugins: [sveltekit({ adapter: adapter({ fallback: 'index.html' }) })],

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
          include: ['src/**/*.test.ts'],
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
