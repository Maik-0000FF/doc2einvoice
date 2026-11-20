<!--
SPDX-FileCopyrightText: Maik-0000FF
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Patched copies of the XPath engine

The rules are evaluated with the XPath engine [Xee](https://github.com/Paligo/xee)
(MIT). Two of its crates are used as patched copies. Two faults of the published
versions make the official rule files report wrong results, and two ways the
published versions evaluate a path make a large invoice take very long to check.

Each copy is the published crate with exactly one patch file applied.
`tools/verify-vendor.sh` proves that: it downloads the published crate, checks
its checksum, applies the patch and compares the result with the copy.

| Crate              | Version | Patch file                               | Changes                                                       |
| ------------------ | ------- | ---------------------------------------- | ------------------------------------------------------------- |
| xee-interpreter    | 0.2.0   | `patches/xee-interpreter-0.2.0.patch`    | predicate with several matching nodes, nodes already in order |
| xee-xpath-compiler | 0.1.5   | `patches/xee-xpath-compiler-0.1.5.patch` | evaluation of `and` and `or`, a search as one descent         |

The first change of each crate corrects a result. The second changes only how a
result is reached.

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

## Nodes that are already in document order

Every step of a path ends by putting its nodes in document order, each once.
The engine did that with a set and a sort that looked the order of a node up
again for every comparison, also when the nodes already stood in order, which
is what one step from one node yields.

The patch leaves nodes that already stand in order as they are, and where a
sort is needed looks the order of each node up once.

## A search through all descendants as one descent

`//name` is short for `/descendant-or-self::node()/child::name`. The engine
evaluated it as written: it visited every node below the context and asked each
for its children of that name. The rule files search the whole invoice in about
a hundred rule contexts and as many assertions, so an invoice of a thousand
lines was walked through a few hundred times, one step per node.

The patch evaluates such a search as `descendant::name`, one walk, where that
selects the same nodes. The two differ only in what a predicate sees as its
position, so the steps are merged only where no predicate can depend on a
position: its value is never a number and nothing in it asks for a position or a
size. That is decided on the shape of the predicate; every shape that is not
known to be safe is evaluated as before. `src/descent.rs` of the copy lists the
shapes.

`crates/doc2einvoice-schematron/tests/descendant_search.rs` pins the results,
above all for predicates that depend on a position.

## Removing a patch

A change is removed when a published version of the crate contains it: bump the
version in the workspace manifest, drop the change from the copy and from its
patch file, and run the conformance tests. They fail if the published version
still has one of the two faults. The two other changes cannot make them fail;
without them the tests only take longer.
