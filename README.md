<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->

# doc2einvoice | Documents to EN 16931 e-invoices, on your own machine

<p align="center"><img height="128" src="src-tauri/icons/128x128@2x.png" alt="doc2einvoice"></p>

[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![CI](https://github.com/Maik-0000FF/doc2einvoice/actions/workflows/ci.yml/badge.svg)](https://github.com/Maik-0000FF/doc2einvoice/actions/workflows/ci.yml)
![status: early development](https://img.shields.io/badge/status-early%20development-yellow)
![EN 16931](https://img.shields.io/badge/EN%2016931-validation%20core-0b7285)
![Windows: tested](https://img.shields.io/badge/Windows-tested-555)
![macOS: tested](https://img.shields.io/badge/macOS-tested-555)
![Linux: tested](https://img.shields.io/badge/Linux-tested-555)
[![Sponsor](https://img.shields.io/badge/Sponsor-%E2%9D%A4-pink?logo=github)](https://github.com/sponsors/Maik-0000FF)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20me-ff5e5b?logo=ko-fi&logoColor=white)](https://ko-fi.com/maik0000ff)

Desktop application that turns a document, starting with PDF, into an electronic
invoice that conforms to the European standard EN 16931.

**Status: early development. The application cannot create invoices yet.** What
it can do today is check an existing e-invoice: an XML file in UBL or CII is
validated against EN 16931 or XRechnung, and every finding is listed with its
rule. The validation is tested against the published rule sets.

## Principles

- **Local only.** Every document is processed on the local machine. The
  application opens no network connection, sends no telemetry and uses no cloud
  service.
- **The user verifies everything.** Data read from a document is only a
  suggestion. Every field must be reviewed and confirmed by the user before an
  invoice can be exported.
- **Validated output.** An invoice is exported only after it passes the
  validation rules of EN 16931.

## Planned output formats

- ZUGFeRD / Factur-X (PDF/A-3 with embedded XML)
- XRechnung (UBL and CII)
- Peppol BIS Billing 3.0

## Conformance

Every conformance statement in this project refers to named versions of
published rule sets on a stated date, and to nothing else.

Development targets as of 2026-11-19:

| Rule set                      | Version                                         |
| ----------------------------- | ----------------------------------------------- |
| EN 16931 validation artefacts | 1.3.17 (UBL and CII)                            |
| XML Schema                    | UBL 2.1, CII D16B                               |
| XRechnung                     | 3.0.2, Schematron rules 2.6.0                   |
| Peppol BIS Billing 3.0        | validation artefacts as published on 2026-11-04 |

- A statement about one version says nothing about earlier or later versions.
  Rule sets and legal requirements change.
- Passing these rules is a technical result. It is not a certification, not an
  approval by an authority or a standards body, and not a statement that an
  invoice is legally or fiscally valid in any country.
- This project is not affiliated with, or endorsed by, the organisations that
  publish these standards and rule sets. Their names are used only to identify
  the specifications.

## How validation works

- **Two steps, as the specifications require.** A document is validated
  against the XML Schema of its syntax (UBL 2.1 or CII D16B) and against the
  Schematron rules of EN 16931 and XRechnung.
- **The published files, unchanged.** Schemas and rules are embedded exactly as
  their publishers released them. Each file is checked against its checksum
  before it is used, and `tools/verify-sources.sh` compares every such file,
  and every test document, with the archive its publisher released. Nothing is
  read from the file system or fetched from a network while validating.
- **An interpreter, no XSLT processor.** The rules are evaluated directly by a
  Schematron interpreter written for this project. No Java is needed.
- **Measured against the reference.** For every document of the conformance
  corpus, each rule set must report the findings that Saxon (Java) reports
  when it runs the same published rules: the same rules, on the same nodes, the
  same number of times. The corpus consists of the test documents the
  publishers ship: 1 288 documents, evaluated against two rule sets each. A
  single difference fails the build. The text of a finding is not compared. For
  two documents the reference stops at a rule it cannot evaluate; there the
  rules the interpreter reports as not evaluable are pinned and checked by hand.
- **The XML Schema step is measured too.** For 8 723 documents, the corpus
  and variants of its valid documents that differ in one small way, the verdict
  must be the verdict of the XML Schema processor of the JDK.
  Compared is the verdict, valid or invalid, not the wording of what is
  reported.
- **Patched XPath engine.** The rules are evaluated with the XPath engine
  [Xee](https://github.com/Paligo/xee). Two faults of its published version
  make the official rules report wrong results, so two of its crates are used
  as patched copies. [vendor/PATCHES.md](vendor/PATCHES.md) describes both
  patches, and `tools/verify-vendor.sh` proves that each copy is the published
  crate plus exactly that patch.

The rule files of Peppol BIS Billing 3.0 are not part of this repository.

## Checking invoices on the command line

The same check runs without a window, for one file, many files or whole
directories:

```sh
doc2einvoice check invoice.xml
doc2einvoice check --findings --profile xrechnung invoices/
```

One line is printed per file, followed by the totals. The command ends
successfully only if every path led to invoices and every one of them conforms,
so it can be used in scripts. `doc2einvoice check --help` lists the options.

## References

The specifications and rule sets this project works against:

- [EN 16931 validation artefacts](https://github.com/ConnectingEurope/eInvoicing-EN16931)
  (CEN/TC 434), licensed under EUPL 1.2
- [XRechnung](https://xeinkauf.de/xrechnung/) and its
  [Schematron rules](https://github.com/itplr-kosit/xrechnung-schematron)
  (KoSIT), licensed under Apache-2.0
- [Peppol BIS Billing 3.0](https://docs.peppol.eu/poacc/billing/3.0/) (OpenPeppol)
- [UBL 2.1](http://docs.oasis-open.org/ubl/os-UBL-2.1/) XML Schema (OASIS),
  distributed unmodified under the notice of the specification
- Cross Industry Invoice D16B XML Schema (UN/CEFACT), distributed under the
  notice the files carry, as bundled with the
  [XRechnung validator configuration](https://github.com/itplr-kosit/validator-configuration-xrechnung)
- [ZUGFeRD](https://www.ferd-net.de/) / Factur-X, based on PDF/A-3
  (ISO 19005-3)

The text of the standard EN 16931 itself is not part of this repository.

## Downloads

There are no releases yet.

Release builds will not be code-signed at first, because signing requires paid
certificates. Expect a warning when you start the application:

- **Windows:** SmartScreen shows "Windows protected your PC". Choose
  "More info", then "Run anyway".
- **macOS:** Gatekeeper refuses to open the application on a double click.
  Open it once from the context menu ("Open"), or allow it under
  System Settings, Privacy & Security.

These warnings say that the publisher is not verified. They say nothing about
what the program does. Only download builds from this repository, and build
from source if you want to be certain.

## Disclaimer

**Use this software entirely at your own risk.**

doc2einvoice is a technical tool. It does not provide legal, tax or accounting
advice, and nothing in this repository or in the application is such advice.

- The software is provided "as is", without warranty of any kind, express or
  implied, including the warranties of merchantability, fitness for a particular
  purpose, correctness and legal or tax compliance.
- Data extracted from a document can be wrong or incomplete. A file that passes
  technical validation can still be wrong in content or fail the legal
  requirements of a particular country.
- You alone are responsible for checking every invoice before you use or send
  it, and for meeting all legal, tax and retention obligations that apply to
  you. If in doubt, consult a qualified adviser.
- To the maximum extent permitted by applicable law, the author is not liable
  for any damage, loss or claim arising from the use of, or the inability to
  use, this software. This includes rejected or invalid invoices, tax
  disadvantages, fines, lost data and lost profit.

The full terms are in sections 15 to 17 of the
[GNU General Public License](LICENSE).

## Development

Requirements: Node.js, a Rust toolchain and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) of the platform.
With Nix, `nix develop` provides all of them on Linux; inside that shell use
`cargo tauri` in place of `npm run tauri`.

```sh
npm install
npm run tauri dev   # run the application
npm run verify      # every quality gate: format, lint, types, tests, licences
```

The Rust code is a workspace:

| Path                             | Purpose                                                      |
| -------------------------------- | ------------------------------------------------------------ |
| `crates/doc2einvoice-schematron` | Schematron interpreter, knows nothing about invoices         |
| `crates/doc2einvoice-validation` | rule sets with version and date, profiles, validation report |
| `src-tauri`                      | the desktop application                                      |
| `rulesets`                       | the published rule files, unchanged                          |
| `schemas`                        | the published XML Schema files, unchanged                    |
| `tests/conformance`              | published test documents and the reference results           |
| `vendor`                         | patched copies of two crates of the XPath engine             |

Two further checks need a network connection and are run on demand:

```sh
npm run vendor:check             # the copies under vendor/ against the published crates
npm run sources:check            # rule files, schemas and test documents against their publishers
npm run conformance:regenerate   # the reference results, with Saxon (Java)
```

## Support

doc2einvoice is free software. If it is useful to you, you can support its
development:

- [GitHub Sponsors](https://github.com/sponsors/Maik-0000FF)
- [Ko-fi](https://ko-fi.com/maik0000ff)

## License

Copyright (C) Maik-0000FF

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version. See [LICENSE](LICENSE).

Copyright and licence notices must be preserved in copies and in modified
versions.

The files of other publishers keep their own terms, stated in
[REUSE.toml](REUSE.toml) and in full under [LICENSES](LICENSES). That includes
the typeface of the interface, [Inter](https://rsms.me/inter/), licensed under
the SIL Open Font License 1.1.
