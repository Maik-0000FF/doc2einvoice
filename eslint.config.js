// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import svelte from 'eslint-plugin-svelte';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';

export default defineConfig(
  {
    ignores: [
      'build/',
      '.svelte-kit/',
      'coverage/',
      'node_modules/',
      'target/',
      'vendor/',
      'rulesets/',
      'schemas/',
      'tests/conformance/',
      'src-tauri/gen/',
      'src/lib/ipc/bindings/',
      '.direnv/',
      'private/',
      '.private/',
    ],
  },
  js.configs.recommended,
  ts.configs.strictTypeChecked,
  ts.configs.stylisticTypeChecked,
  svelte.configs.recommended,
  prettier,
  svelte.configs.prettier,
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: ts.parser,
      },
    },
  },
  {
    // Every control, table, list of facts and section heading is drawn by one
    // building block under
    // components/base/, which sets its look and its behaviour. A view that
    // writes such an element itself fails the check.
    files: ['src/**/*.svelte'],
    ignores: ['src/lib/components/base/**'],
    rules: {
      'svelte/no-restricted-html-elements': [
        'error',
        {
          elements: [
            'button',
            'dd',
            'dialog',
            'dl',
            'dt',
            'h3',
            'input',
            'option',
            'select',
            'table',
            'td',
            'textarea',
            'th',
            'tr',
          ],
          message: 'Use the building block under src/lib/components/base/ instead.',
        },
      ],
    },
  },
  {
    // The root-level JavaScript configuration files and the scripts under
    // tools/ are outside the TypeScript project, so the rules that need type
    // information do not apply to them.
    files: ['*.js', 'tools/**/*.mjs'],
    extends: [ts.configs.disableTypeChecked],
  },
);
