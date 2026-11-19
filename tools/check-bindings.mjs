// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

// Fails if the TypeScript bindings generated from the Rust types differ from
// what is committed: a changed, a missing or a new, not yet committed file.
// Run after the Rust tests, which regenerate the bindings.

import { execFileSync } from 'node:child_process';
import process from 'node:process';

const BINDINGS_DIRECTORY = 'src/lib/ipc/bindings';

// Lists changed and untracked files and changes nothing in the repository.
const differences = execFileSync(
  'git',
  ['status', '--porcelain', '--untracked-files=all', '--', BINDINGS_DIRECTORY],
  { encoding: 'utf8' },
).trim();

if (differences !== '') {
  console.error(`The generated bindings differ from the committed ones:\n${differences}`);
  process.exit(1);
}
