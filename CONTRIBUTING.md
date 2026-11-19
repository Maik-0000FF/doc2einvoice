<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Contributing

Thank you for considering a contribution. For anything larger than a small fix,
please open an issue first, so that the approach can be agreed before work is
spent on it.

A security problem is not reported in an issue; see [SECURITY.md](SECURITY.md).

## Setup

Requirements: Node.js, a Rust toolchain (the version is pinned in
`rust-toolchain.toml`) and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) of your platform.
With Nix, `nix develop` provides all of them on Linux.

```sh
npm install
npm run verify      # every quality gate: format, lint, types, tests, licences
```

`npm run verify` must pass before a pull request is opened. The README
describes the layout of the repository and the further checks.

## Branches

Pull requests target `dev`. `main` receives the state of `dev` once it is
verified.

## Rules

- **Files of other publishers stay unchanged.** The rule files under
  `rulesets`, the schemas under `schemas` and the documents under
  `tests/conformance/corpus` are kept byte for byte as published;
  `npm run sources:check` verifies that. The copies under `vendor` differ from
  the published crates only by the patches documented in `vendor/PATCHES.md`.
- **Conformance is complete agreement.** The conformance tests compare the
  validation with the reference implementations. A change that makes one
  document differ is not merged; the expected results are regenerated with
  `npm run conformance:regenerate`, never edited by hand.
- **Code that checks input starts with a table.** Write down the inputs,
  including the malformed and the hostile ones, with the expected outcome, next
  to the function. Each row becomes a test.
- **A construct that is not supported is refused.** Nothing is skipped
  silently: a rule that is never checked is worse than a rule file that is
  rejected.
- **Every value has one place.** No number, path or name with a meaning is
  repeated; it is defined once and used from there.
- **No real data in tests.** Test documents contain invented names, addresses
  and identifiers only.
- **Every file states its licence.** New files carry an SPDX header;
  `npm run license:check` verifies the whole repository.

## Licence of contributions

The project is licensed under GPL-3.0-or-later. By contributing you agree that
your contribution is licensed under the same terms. Copyright notices in
existing files are kept.
