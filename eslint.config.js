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
    // Every heading, control, table and list of facts is drawn by one building
    // block under components/base/, which sets its look and its behaviour. A
    // view that writes one of the elements below itself fails the check. Text
    // and the containers that lay a view out (p, section, article, footer)
    // stay free.
    files: ['src/**/*.svelte'],
    ignores: ['src/lib/components/base/**'],
    rules: {
      'svelte/no-restricted-html-elements': [
        'error',
        {
          elements: [
            'a',
            'button',
            'dd',
            'details',
            'dialog',
            'dl',
            'dt',
            'fieldset',
            'form',
            'h1',
            'h2',
            'h3',
            'h4',
            'h5',
            'h6',
            'input',
            'label',
            'optgroup',
            'option',
            'progress',
            'select',
            'summary',
            'table',
            'tbody',
            'td',
            'textarea',
            'th',
            'thead',
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
