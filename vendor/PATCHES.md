<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Patched copies of the XPath engine

The rules are evaluated with the XPath engine [Xee](https://github.com/Paligo/xee)
(MIT). Two of its crates are used as patched copies, because two faults of the
published versions make the official rule files report wrong results.

Each copy is the published crate with exactly one patch applied.
`tools/verify-vendor.sh` proves that: it downloads the published crate, checks
its checksum, applies the patch and compares the result with the copy.

| Crate              | Version | Patch                                                       |
| ------------------ | ------- | ----------------------------------------------------------- |
| xee-interpreter    | 0.2.0   | `patches/xee-interpreter-0.2.0_predicate_multi_item.patch`  |
| xee-xpath-compiler | 0.1.5   | `patches/xee-xpath-compiler-0.1.5_short_circuit.patch`      |

## Predicate with several matching nodes

`a[b]` raised the type error XPTY0004 when `a` has more than one child `b`. The
engine tested whether the predicate value is a single number by atomizing the
whole sequence, which fails for more than one item. The XPath specification
defines such a predicate by its effective boolean value, which is true for a
sequence that starts with a node.

The patch makes the test answer "not a number" for an empty or multi-item
sequence instead of raising an error. Upstream: issue 147, pull request 148.

## Evaluation of `and` and `or`

Both operands were always evaluated, so an error in the right operand surfaced
even when the left operand already decided the result. The rule files rely on
the left operand guarding the right one, as in
`not(amount) or xs:decimal(amount) > 0`, and the reference processors evaluate
them that way.

The patch lowers `and` and `or` to a conditional: the right operand is evaluated
only if the left operand does not decide the result.

## Removing a patch

A patch is removed when a published version of the crate contains the fix: bump
the version in the workspace manifest, drop the copy and its patch, and run the
conformance tests. They fail if the published version still has the fault.
