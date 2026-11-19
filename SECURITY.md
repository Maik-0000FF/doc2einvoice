<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Security policy

## Supported versions

There is no release yet. Fixes go into the development branch `dev` and from
there into `main`.

## Reporting a vulnerability

Please report a vulnerability privately, not in a public issue:

1. Open the **Security** tab of this repository.
2. Choose **Report a vulnerability**.

A useful report contains:

- the commit or version, and the operating system
- what you did, what happened and what you expected
- the smallest document that shows the problem

Do not attach a real invoice. Replace names, addresses, tax numbers and bank
details with invented data before you send a document.

## What counts as a vulnerability

Besides the usual (code execution, reading or writing files the user did not
choose), these are treated as security problems in this project:

- a document that is invalid but is reported as conforming
- a network connection the user did not explicitly allow
- a document that makes the application crash or hang
- a schema or rule file that is used although its checksum does not match

## What to expect

I read every report and answer as soon as I can. Once a fix is published, the
report is credited to you unless you prefer otherwise. There is no bug bounty.
