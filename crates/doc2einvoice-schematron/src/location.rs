// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Describing where in a document a node sits and what it is called.

use xot::{Node, Xot};

const STEP_SEPARATOR: char = '/';
const ATTRIBUTE_MARKER: char = '@';
/// Stands for a node that has no name, such as a text node.
const UNNAMED_NODE: &str = "node()";

/// The path of a node from the document element, with the position of every
/// element among its siblings of the same name:
/// `/Invoice[1]/InvoiceLine[2]/@currencyID`.
///
/// Names are local names. The path is meant for people locating a finding, not
/// for evaluation.
pub(crate) fn path_of(xot: &Xot, node: Node) -> String {
    let mut steps = Vec::new();
    let mut current = Some(node);

    while let Some(step) = current {
        if xot.is_document(step) {
            break;
        }
        let name = local_name_of(xot, step);
        steps.push(if xot.is_attribute_node(step) {
            // An element has at most one attribute of a name.
            format!("{ATTRIBUTE_MARKER}{name}")
        } else {
            format!("{name}[{}]", position_of(xot, step))
        });
        current = xot.parent(step);
    }
    if steps.is_empty() {
        return STEP_SEPARATOR.to_string();
    }
    steps.iter().rev().fold(String::new(), |mut path, step| {
        path.push(STEP_SEPARATOR);
        path.push_str(step);
        path
    })
}

/// The name of a node as the document writes it, with its prefix:
/// `cbc:ID`. Empty for a node without a name.
pub(crate) fn name_of(xot: &Xot, node: Node) -> String {
    xot.node_name(node)
        .map(|name| {
            xot.full_name(node, name)
                .unwrap_or_else(|_| xot.local_name_str(name).to_owned())
        })
        .unwrap_or_default()
}

fn local_name_of(xot: &Xot, node: Node) -> &str {
    xot.node_name(node)
        .map_or(UNNAMED_NODE, |name| xot.local_name_str(name))
}

/// The one-based position of a node among the siblings that share its name.
fn position_of(xot: &Xot, node: Node) -> usize {
    let name = xot.node_name(node);
    let Some(parent) = xot.parent(node) else {
        return 1;
    };
    xot.children(parent)
        .filter(|sibling| xot.node_name(*sibling) == name)
        .position(|sibling| sibling == node)
        .map_or(1, |index| index + 1)
}
