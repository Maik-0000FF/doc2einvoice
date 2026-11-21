// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The values a Cross Industry Invoice holds for each business term, read at
//! the paths the syntax crate gives them, each in the occurrence of the
//! repeated group it stands in.
//!
//! Only the outermost repeated group counts: a discount of the second line
//! stands in the second line, whichever discount of that line it is. The
//! values within one occurrence are compared as one set.

use std::collections::{BTreeMap, HashMap};

use doc2einvoice_model::{Occurrence, group_id, group_number, term_by_id};
use doc2einvoice_syntax::bindings::{PREFIXES, bindings};
use xee_xpath::context::StaticContextBuilder;
use xee_xpath::query::SequenceQuery;
use xee_xpath::{Documents, Item, Queries, Query};

/// The business groups an invoice may hold more than once, outside an
/// invoice line: notes, preceding invoices, credit transfers, document level
/// allowances and charges, the VAT breakdown, supporting documents and the
/// lines themselves.
const REPEATED_GROUPS: [u16; 8] = [1, 3, 17, 20, 21, 23, 24, LINE];

/// The business group of an invoice line.
const LINE: u16 = 25;

/// The business groups that stand within an invoice line: its period,
/// allowances, charges, price, VAT, item and item attributes.
const WITHIN_LINE: std::ops::RangeInclusive<u16> = 26..=32;

/// Repeated groups whose occurrence is the element of an enclosing group
/// that holds them, because some of their terms stand beside them in it: a
/// credit transfer (BG-17) is a payment means element (BG-16) with an
/// account, and its BIC (BT-86) stands beside the account, not in it.
const HELD_BY: [(u16, u16); 1] = [(17, 16)];

/// What separates the alternatives of a path.
const ALTERNATIVE: &str = " | ";

/// Turns the nodes a path selects into their values.
const VALUES: &str = "! normalize-space(string(.))";

/// Where a value stands: its business term, and the occurrence of the
/// outermost repeated group it stands in, if it stands in one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub term: u16,
    pub occurrence: Option<Occurrence>,
}

/// What an invoice holds for its business terms.
pub struct Values {
    /// The non-empty values at each place that has any.
    pub found: BTreeMap<Place, Vec<String>>,
    /// The terms whose path could not be evaluated on this invoice.
    pub unevaluated: Vec<u16>,
}

/// How the values of one business term are read.
enum Reading {
    /// From the root of the invoice.
    Whole(SequenceQuery),
    /// From each occurrence of a repeated group.
    Within { group: u16, query: SequenceQuery },
}

/// The queries for every business term and repeated group, compiled once.
pub struct Expected {
    terms: Vec<(u16, Reading)>,
    groups: HashMap<u16, SequenceQuery>,
}

/// The outermost repeated group a term of `group` stands in.
///
/// | Group of the term        | Outermost repeated group |
/// |--------------------------|--------------------------|
/// | none (the invoice)       | none                     |
/// | BG-4 (seller), BG-16     | none                     |
/// | BG-23 (VAT breakdown)    | BG-23                    |
/// | BG-25 (line)             | BG-25                    |
/// | BG-27 (line allowance), BG-32 (item attribute) | BG-25 |
fn outermost_repeated(group: Option<u16>) -> Option<u16> {
    let group = group?;
    if WITHIN_LINE.contains(&group) {
        Some(LINE)
    } else {
        REPEATED_GROUPS.contains(&group).then_some(group)
    }
}

/// `path` relative to the group at `group_path`: every alternative of it
/// starts below the group.
///
/// | Path                                 | Group path | Relative       |
/// |--------------------------------------|------------|----------------|
/// | `/a/g/b`                             | `/a/g`     | `b`            |
/// | `/a/g/b \| /a/g/c`                   | `/a/g`     | `b \| c`       |
/// | `/a/g/b \| /a/x/c`                   | `/a/g`     | none           |
/// | `/a/gh/b`                            | `/a/g`     | none           |
fn relative(path: &str, group_path: &str) -> Option<String> {
    let below = format!("{group_path}/");
    let alternatives: Option<Vec<&str>> = path
        .split(ALTERNATIVE)
        .map(|alternative| alternative.strip_prefix(below.as_str()))
        .collect();
    Some(alternatives?.join(ALTERNATIVE))
}

impl Expected {
    /// Compiles the queries.
    ///
    /// # Panics
    ///
    /// If a path of the table is no XPath, or a term of a repeated group has
    /// a path outside the group; a test compiles them all.
    pub fn new() -> Self {
        let mut context = StaticContextBuilder::default();
        context.namespaces(PREFIXES);
        let compiler = Queries::new(context);
        let compile = |expression: &str| {
            compiler
                .sequence(expression)
                .unwrap_or_else(|error| unreachable!("{expression}: {error}"))
        };
        let table = bindings();
        let cii_of_group: HashMap<u16, &str> = table
            .iter()
            .filter_map(|binding| Some((group_number(binding.id)?, binding.cii)))
            .collect();
        // For each repeated group: the path its terms are relative to, and
        // the path of its occurrences.
        let group_paths: HashMap<u16, (&str, String)> = REPEATED_GROUPS
            .iter()
            .map(|number| {
                let own = cii_of_group[number];
                let held = HELD_BY.iter().find(|(group, _)| group == number);
                let paths = match held {
                    None => (own, own.to_owned()),
                    Some((_, holder)) => {
                        let base = cii_of_group[holder];
                        let step = relative(own, base).unwrap_or_else(|| {
                            unreachable!("{} is not below {base}", group_id(*number))
                        });
                        (base, format!("{base}[{step}]"))
                    }
                };
                (*number, paths)
            })
            .collect();
        let groups = group_paths
            .iter()
            .map(|(number, (_, occurrences))| (*number, compile(occurrences)))
            .collect();
        let terms = table
            .iter()
            .filter_map(|binding| {
                let term = term_by_id(binding.id)?;
                let reading = match outermost_repeated(term.group) {
                    None => Reading::Whole(compile(&format!("({}) {VALUES}", binding.cii))),
                    Some(group) => {
                        let group_path = group_paths[&group].0;
                        let path = relative(binding.cii, group_path).unwrap_or_else(|| {
                            unreachable!("{} is not below {group_path}", binding.id)
                        });
                        Reading::Within {
                            group,
                            query: compile(&format!("({path}) {VALUES}")),
                        }
                    }
                };
                Some((term.number, reading))
            })
            .collect();
        Self { terms, groups }
    }

    /// The number of terms there is a query for.
    #[cfg(test)]
    pub fn terms(&self) -> usize {
        self.terms.len()
    }

    /// What `xml` holds for each business term; none if the XML cannot be
    /// read. A path that cannot be evaluated is counted, not fatal.
    pub fn values(&self, xml: &str) -> Option<Values> {
        let mut documents = Documents::new();
        let handle = documents.add_string_without_uri(xml).ok()?;
        let root = Item::Node(documents.document_node(handle)?);
        let mut occurrences: HashMap<u16, Option<Vec<Item>>> = HashMap::new();
        for (number, query) in &self.groups {
            let nodes = query.execute(&mut documents, &root).ok().map(|sequence| {
                sequence
                    .nodes()
                    .filter_map(Result::ok)
                    .map(Item::Node)
                    .collect()
            });
            occurrences.insert(*number, nodes);
        }
        let mut values = Values {
            found: BTreeMap::new(),
            unevaluated: Vec::new(),
        };
        for (term, reading) in &self.terms {
            let evaluated = match reading {
                Reading::Whole(query) => read(&mut documents, query, &root).map(|list| {
                    values.found_at(
                        Place {
                            term: *term,
                            occurrence: None,
                        },
                        list,
                    );
                }),
                Reading::Within { group, query } => occurrences[group].as_ref().and_then(|items| {
                    for (index, item) in items.iter().enumerate() {
                        let list = read(&mut documents, query, item)?;
                        let occurrence = Occurrence {
                            group: *group,
                            index,
                        };
                        values.found_at(
                            Place {
                                term: *term,
                                occurrence: Some(occurrence),
                            },
                            list,
                        );
                    }
                    Some(())
                }),
            };
            if evaluated.is_none() {
                values.unevaluated.push(*term);
            }
        }
        Some(values)
    }
}

impl Values {
    fn found_at(&mut self, place: Place, list: Vec<String>) {
        if !list.is_empty() {
            self.found.insert(place, list);
        }
    }
}

/// The non-empty values `query` gives from `item`; none if it fails.
fn read(documents: &mut Documents, query: &SequenceQuery, item: &Item) -> Option<Vec<String>> {
    let sequence = query.execute(documents, item).ok()?;
    Some(
        sequence
            .atomized(documents.xot())
            .filter_map(Result::ok)
            .filter_map(|atomic| String::try_from(atomic).ok())
            .filter(|value| !value.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use doc2einvoice_model::{Occurrence, TERMS};

    use super::{Expected, LINE, Place, outermost_repeated, relative};

    /// Two lines; a payment in cash and a credit transfer, whose BIC stands
    /// beside the account; and a tender reference (BT-17), which CII writes
    /// in the element a supporting document (BG-24) uses, but outside that
    /// group.
    const INVOICE: &str = r#"<rsm:CrossIndustryInvoice
        xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"
        xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100">
      <rsm:ExchangedDocument><ram:ID> RE-1 </ram:ID></rsm:ExchangedDocument>
      <rsm:SupplyChainTradeTransaction>
        <ram:IncludedSupplyChainTradeLineItem>
          <ram:SpecifiedTradeProduct><ram:Name>Apfel</ram:Name></ram:SpecifiedTradeProduct>
        </ram:IncludedSupplyChainTradeLineItem>
        <ram:IncludedSupplyChainTradeLineItem>
          <ram:SpecifiedTradeProduct><ram:Name>Birne</ram:Name></ram:SpecifiedTradeProduct>
        </ram:IncludedSupplyChainTradeLineItem>
        <ram:ApplicableHeaderTradeSettlement>
          <ram:SpecifiedTradeSettlementPaymentMeans>
            <ram:TypeCode>10</ram:TypeCode>
          </ram:SpecifiedTradeSettlementPaymentMeans>
          <ram:SpecifiedTradeSettlementPaymentMeans>
            <ram:TypeCode>58</ram:TypeCode>
            <ram:PayeePartyCreditorFinancialAccount>
              <ram:IBANID>DE02120300000000202051</ram:IBANID>
            </ram:PayeePartyCreditorFinancialAccount>
            <ram:PayeeSpecifiedCreditorFinancialInstitution>
              <ram:BICID>BYLADEM1001</ram:BICID>
            </ram:PayeeSpecifiedCreditorFinancialInstitution>
          </ram:SpecifiedTradeSettlementPaymentMeans>
        </ram:ApplicableHeaderTradeSettlement>
        <ram:ApplicableHeaderTradeAgreement>
          <ram:AdditionalReferencedDocument>
            <ram:IssuerAssignedID>V-7</ram:IssuerAssignedID>
            <ram:TypeCode>50</ram:TypeCode>
          </ram:AdditionalReferencedDocument>
        </ram:ApplicableHeaderTradeAgreement>
      </rsm:SupplyChainTradeTransaction>
    </rsm:CrossIndustryInvoice>"#;

    /// The business group of a credit transfer.
    const CREDIT_TRANSFER: u16 = 17;

    fn line(index: usize) -> Occurrence {
        Occurrence { group: LINE, index }
    }

    #[test]
    fn outermost_groups_follow_their_table() {
        assert_eq!(outermost_repeated(None), None);
        assert_eq!(outermost_repeated(Some(4)), None);
        assert_eq!(outermost_repeated(Some(16)), None);
        assert_eq!(outermost_repeated(Some(23)), Some(23));
        assert_eq!(outermost_repeated(Some(LINE)), Some(LINE));
        assert_eq!(outermost_repeated(Some(27)), Some(LINE));
        assert_eq!(outermost_repeated(Some(32)), Some(LINE));
    }

    #[test]
    fn relative_paths_follow_their_table() {
        assert_eq!(relative("/a/g/b", "/a/g").as_deref(), Some("b"));
        assert_eq!(
            relative("/a/g/b | /a/g/c", "/a/g").as_deref(),
            Some("b | c")
        );
        assert_eq!(relative("/a/g/b | /a/x/c", "/a/g"), None);
        assert_eq!(relative("/a/gh/b", "/a/g"), None);
    }

    #[test]
    fn every_term_has_a_query() {
        assert_eq!(Expected::new().terms(), TERMS.len());
    }

    #[test]
    fn values_stand_in_the_occurrence_of_their_group() {
        let values = Expected::new()
            .values(INVOICE)
            .unwrap_or_else(|| unreachable!("the invoice is well-formed"));
        let at = |term, occurrence| values.found.get(&Place { term, occurrence }).cloned();
        assert_eq!(at(1, None), Some(vec!["RE-1".to_owned()]));
        assert_eq!(at(153, Some(line(0))), Some(vec!["Apfel".to_owned()]));
        assert_eq!(at(153, Some(line(1))), Some(vec!["Birne".to_owned()]));
        assert_eq!(at(17, None), Some(vec!["V-7".to_owned()]));
        assert!(values.found.keys().all(|place| place.term != 122));
        let transfer = Some(Occurrence {
            group: CREDIT_TRANSFER,
            index: 0,
        });
        assert_eq!(
            at(84, transfer),
            Some(vec!["DE02120300000000202051".to_owned()])
        );
        assert_eq!(at(86, transfer), Some(vec!["BYLADEM1001".to_owned()]));
        assert!(values.unevaluated.is_empty());
    }
}
