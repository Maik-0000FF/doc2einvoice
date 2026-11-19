<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Using doc2einvoice

What the application can do today is check an existing e-invoice. Creating
invoices is not possible yet.

- [What is checked](#what-is-checked)
- [Checking an invoice in the window](#checking-an-invoice-in-the-window)
- [Checking invoices on the command line](#checking-invoices-on-the-command-line)
- [Version and help](#version-and-help)
- [What a result means](#what-a-result-means)
- [Limits](#limits)

## What is checked

An invoice is an XML file in one of the two syntaxes of EN 16931: UBL 2.1 or
UN/CEFACT CII D16B. It is checked in two steps:

1. against the XML Schema of its syntax,
2. against the rules of a profile.

| Profile     | Rules                                                   |
| ----------- | ------------------------------------------------------- |
| `en16931`   | the rules of the European standard EN 16931             |
| `xrechnung` | the rules of EN 16931 and those of the German XRechnung |

The profile is chosen from what the invoice says about itself: its
specification identifier (BT-24). An invoice that names XRechnung is checked as
XRechnung, every other invoice as EN 16931. The profile can be changed.

The [README](../README.md#conformance) states which releases of the rule sets
are used.

## Checking an invoice in the window

1. Choose **Check an e-invoice** and select the file.
2. The result appears below the button.

The result shows:

- the verdict: whether the invoice conforms,
- the file, its syntax and the profile it was checked as,
- the specification the invoice names, and a note if its rules were not
  applied,
- whether the file is valid against the XML Schema,
- for every rule set its version and the date it was published, and the rules
  the invoice does not satisfy.

Each finding names the rule (for example `BR-CO-15`), how serious it is, the
place in the document and the wording of the rule as its publisher wrote it. An
error makes the invoice invalid; a warning or a note does not.

To check the same invoice as another profile, choose it in the list next to
**Checked as**.

A file that is not an invoice is not checked, and the reason is stated.

## Checking invoices on the command line

The same check runs without a window:

```sh
doc2einvoice check invoice.xml
doc2einvoice check first.xml second.xml invoices/
```

A directory stands for every file in it and below it whose name ends in `.xml`.

One line is printed per file, followed by the totals:

```text
invoice.xml: conforms (profile: xrechnung, schema violations: 0, errors: 0, warnings: 1, notes: 0)
order.xml: not checked: the document element `Order` is no invoice in UBL or CII
conform: 1, do not conform: 0, not checked: 1
```

| Option              | Effect                                                       |
| ------------------- | ------------------------------------------------------------ |
| `--profile PROFILE` | check every file as this profile instead of the one it names |
| `--findings`        | list the findings of each file below its line, one per line  |
| `--help`            | list the options                                             |

If the rules of the specification an invoice names were not applied, its line
ends with `; the rules of the specification it names were not applied` after
the closing parenthesis. That is the case for a specification whose rules are
not included, and for an XRechnung checked with `--profile en16931`.

With `--findings`, each finding is one line whose fields are separated by tabs:
the severity (or `schema`), the rule, the place in the document and the wording.
A rule that could not be evaluated has one more field, `not evaluated:` with the
error code.

The command ends with a status that scripts can use:

| Status | Meaning                                                                   |
| ------ | ------------------------------------------------------------------------- |
| `0`    | every path led to invoices, and every one of them conforms                |
| `1`    | an invoice does not conform, a path was not checked, or nothing was found |
| `2`    | the command line was not understood                                       |

## Version and help

```sh
doc2einvoice --version
doc2einvoice --help
```

`--version` (or `-V`) prints the name and the version of the program, `--help`
(or `-h`) the commands and options. Both answer without opening a window.
Started without any argument, the program opens its window.

## What a result means

A result is the outcome of a technical validation against the named releases of
the rule sets, and of nothing newer. An invoice that conforms can still be wrong
in content or fail the legal requirements of a country. See the
[disclaimer](../README.md#disclaimer).

## Limits

- Only XML files are read. An invoice embedded in a PDF (ZUGFeRD, Factur-X)
  cannot be checked yet.
- Only files in UTF-8 are read, up to 32 MiB, with elements nested up to 256
  levels deep.
- Only the rules of EN 16931 and of XRechnung are included. An invoice that
  names another specification, such as a profile of Factur-X or Peppol BIS
  Billing, is checked against the rules of EN 16931 only, and the result says
  that the rules of the specification it names were not applied. What that
  specification allows beyond EN 16931, as the EXTENDED profile of Factur-X
  does, is then reported as findings.
- On Windows the released program has no console. Called from a terminal it
  shows no output there; redirect the output to a file or a pipe.
