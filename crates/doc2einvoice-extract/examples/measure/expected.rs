// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The values a Cross Industry Invoice holds for each business term, read at
//! the paths the syntax crate gives them.

use doc2einvoice_syntax::bindings::bindings;
use doc2einvoice_syntax::namespaces::{CII_AGGREGATE, CII_INVOICE, CII_QUALIFIED, CII_UNQUALIFIED};
use xee_xpath::context::StaticContextBuilder;
use xee_xpath::query::SequenceQuery;
use xee_xpath::{Documents, Item, Queries, Query};

/// The prefixes the paths of a Cross Industry Invoice use.
const NAMESPACES: [(&str, &str); 4] = [
    ("rsm", CII_INVOICE),
    ("ram", CII_AGGREGATE),
    ("udt", CII_UNQUALIFIED),
    ("qdt", CII_QUALIFIED),
];

/// The prefix of the identifier of a business term, as in `BT-27`.
const TERM_PREFIX: &str = "BT-";

/// What an invoice holds for its business terms.
pub struct Values {
    /// The non-empty values of each term that has any, by number of the term.
    pub found: Vec<(u16, Vec<String>)>,
    /// The terms whose path could not be evaluated on this invoice.
    pub unevaluated: Vec<u16>,
}

/// The queries for every business term, compiled once.
pub struct Expected {
    queries: Vec<(u16, SequenceQuery)>,
}

impl Expected {
    pub fn new() -> Self {
        let mut context = StaticContextBuilder::default();
        context.namespaces(NAMESPACES);
        let compiler = Queries::new(context);
        let queries = bindings()
            .into_iter()
            .filter_map(|binding| {
                let number = binding.id.strip_prefix(TERM_PREFIX)?.parse().ok()?;
                let expression = format!("({}) ! normalize-space(string(.))", binding.cii);
                let query = compiler
                    .sequence(&expression)
                    .unwrap_or_else(|error| unreachable!("{}: {error}", binding.id));
                Some((number, query))
            })
            .collect();
        Self { queries }
    }

    /// What `xml` holds for each business term; none if the XML cannot be
    /// read. A path that cannot be evaluated is counted, not fatal.
    pub fn values(&self, xml: &str) -> Option<Values> {
        let mut documents = Documents::new();
        let handle = documents.add_string_without_uri(xml).ok()?;
        let root = Item::Node(documents.document_node(handle)?);
        let mut values = Values {
            found: Vec::new(),
            unevaluated: Vec::new(),
        };
        for (number, query) in &self.queries {
            let Ok(sequence) = query.execute(&mut documents, &root) else {
                values.unevaluated.push(*number);
                continue;
            };
            let list: Vec<String> = sequence
                .atomized(documents.xot())
                .filter_map(Result::ok)
                .filter_map(|atomic| String::try_from(atomic).ok())
                .filter(|value| !value.is_empty())
                .collect();
            if !list.is_empty() {
                values.found.push((*number, list));
            }
        }
        Some(values)
    }
}
