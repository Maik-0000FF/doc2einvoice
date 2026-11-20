// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Searching through all descendants (`//`) and putting nodes in document
//! order.
//!
//! The XPath engine evaluates `//name` as one descent where that selects the
//! same nodes, and leaves nodes alone that already stand in document order.
//! Both only change how a result is reached. These tests pin the results,
//! above all where a predicate depends on the position of a node: there the
//! search must not be turned into one descent.
//!
//! The document, with the `id` of every element:
//!
//! ```text
//! doc
//! ├── a1
//! │   ├── b1 (k=1)
//! │   │   └── c1
//! │   └── b2 (k=2)
//! └── a2
//!     ├── b3 (k=1)
//!     │   ├── b4 (k=3)
//!     │   └── b5 (k=1)
//!     └── c2
//! ```
//!
//! Input table. Each row is one entry of a list below; the selected elements
//! are named by their `id`, in document order.
//!
//! | expression                          | selects        | note                                 |
//! |-------------------------------------|----------------|--------------------------------------|
//! | `//b`                               | b1 b2 b3 b4 b5 | no predicate                         |
//! | `//*[@k = '3']`                     | b4             | any name                             |
//! | `//b[@k = '1']`                     | b1 b3 b5       | a comparison                         |
//! | `//b[@k = '1' and not(c)]`          | b3 b5          | `and`, a function                    |
//! | `//b[c]`                            | b1             | a path                               |
//! | `//b[not(c)]`                       | b2 b3 b4 b5    | a function that returns a boolean    |
//! | `//b[b[1]/@k = '3']`                | b3             | a position further inside            |
//! | `//b[@k = /doc/a[1]/b[2]/@k]`       | b2             | a path from the root inside          |
//! | `//b/c`                             | c1             | a step after the search              |
//! | `/doc//b[@k = '1']`                 | b1 b3 b5       | a search in the middle of a path     |
//! | `//b//b`                            | b4 b5          | two searches, results overlap        |
//! | `//b[1]`                            | b1 b3 b4       | first among the children of a parent |
//! | `//b[position() = 2]`               | b2 b5          | second among its siblings            |
//! | `//b[last()]`                       | b2 b3 b5       | last among its siblings              |
//! | `//b[count(b) + 1]`                 | b1 b4          | a number that is computed            |
//! | `//b[$second]`                      | b2 b5          | a number from a variable             |
//! | `//b[@k = '1'][1]`                  | b1 b3 b5       | a position after a comparison        |
//! | `//b[if (c) then 1 else 2]`         | b1 b2 b5       | a shape that is not looked into      |
//! | `(/doc/a[2]/b, /doc/a[1]/b)/@k/..`  | b1 b2 b3       | nodes given out of order             |
//! | `(//b[@k = '3'] \| //b[@k = '2'])`   | b2 b4          | a union                              |
//!
//! A search for attributes is checked by its count: `count(//@k)` is 5.
//!
//! A predicate could also reach the position without naming it, through
//! `function-lookup`. The engine cannot call a function it looked up that way:
//! the call fails, in a search and in its written out form alike. The test
//! pins that failure. It fails as soon as an engine can make the call, and
//! then the two forms have to be compared by what they select.

use doc2einvoice_schematron::{Finding, NoIncludes, Outcome, Schema};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DOCUMENT: &str = r#"<doc>
  <a id="a1"><b id="b1" k="1"><c id="c1"/></b><b id="b2" k="2"/></a>
  <a id="a2"><b id="b3" k="1"><b id="b4" k="3"/><b id="b5" k="1"/></b><c id="c2"/></a>
</doc>"#;

/// The search as it is written in a rule file.
const SEARCH: &str = "//";
/// The same search written out, with a predicate that is always true on the
/// first step. The predicate changes nothing in the result, but an engine
/// that turns `//` into one descent leaves this form alone.
const SEARCH_WRITTEN_OUT: &str = "/descendant-or-self::node()[true()]/";

/// Expressions and the elements they select.
const SELECTIONS: [(&str, &str); 20] = [
    ("//b", "b1 b2 b3 b4 b5"),
    ("//*[@k = '3']", "b4"),
    ("//b[@k = '1']", "b1 b3 b5"),
    ("//b[@k = '1' and not(c)]", "b3 b5"),
    ("//b[c]", "b1"),
    ("//b[not(c)]", "b2 b3 b4 b5"),
    ("//b[b[1]/@k = '3']", "b3"),
    ("//b[@k = /doc/a[1]/b[2]/@k]", "b2"),
    ("//b/c", "c1"),
    ("/doc//b[@k = '1']", "b1 b3 b5"),
    ("//b//b", "b4 b5"),
    ("//b[1]", "b1 b3 b4"),
    ("//b[position() = 2]", "b2 b5"),
    ("//b[last()]", "b2 b3 b5"),
    ("//b[count(b) + 1]", "b1 b4"),
    ("//b[$second]", "b2 b5"),
    ("//b[@k = '1'][1]", "b1 b3 b5"),
    ("//b[if (c) then 1 else 2]", "b1 b2 b5"),
    ("(/doc/a[2]/b, /doc/a[1]/b)/@k/..", "b1 b2 b3"),
    ("(//b[@k = '3'] | //b[@k = '2'])", "b2 b4"),
];

/// A rule file with one assertion on `/doc`, the variable `$second` and
/// `$none`, an empty list of arguments.
fn rules(test: &str) -> String {
    format!(
        r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
             <pattern>
               <rule context="/doc">
                 <let name="second" value="2"/>
                 <let name="none" value="[]"/>
                 <assert id="R-1" flag="fatal" test="{test}">text</assert>
               </rule>
             </pattern>
           </schema>"#
    )
}

/// Whether `test` holds on the document.
fn holds(test: &str) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(Schema::compile(&rules(test), &NoIncludes)?
        .validate(DOCUMENT)?
        .is_clean())
}

/// A test that compares the `id`s an expression selects with a text.
fn selects(expression: &str, ids: &str) -> String {
    format!("string-join(({expression})/@id, ' ') = '{ids}'")
}

#[test]
fn a_search_selects_what_the_table_states() -> TestResult {
    for (expression, ids) in SELECTIONS {
        assert!(holds(&selects(expression, ids))?, "{expression}");
    }
    Ok(())
}

#[test]
fn a_search_selects_what_its_written_out_form_selects() -> TestResult {
    for (expression, ids) in SELECTIONS {
        let written_out = expression.replace(SEARCH, SEARCH_WRITTEN_OUT);

        assert!(holds(&selects(&written_out, ids))?, "{written_out}");
    }
    Ok(())
}

#[test]
fn a_wrong_selection_is_noticed() -> TestResult {
    // Guards the two tests above: a comparison that could not fail would
    // prove nothing.
    assert!(!holds(&selects("//b", "b1"))?);
    Ok(())
}

#[test]
fn a_position_asked_for_indirectly_cannot_be_evaluated() -> TestResult {
    const PREDICATE: &str = "b[apply(function-lookup(xs:QName('fn:position'), 0), $none) = 2]";
    /// The error the engine raises when the looked up function is called.
    const CALL_FAILS: &str = "FORG0001";

    for search in [SEARCH, SEARCH_WRITTEN_OUT] {
        let test = selects(&format!("{search}{PREDICATE}"), "b2 b5");
        let report = Schema::compile(&rules(&test), &NoIncludes)?.validate(DOCUMENT)?;

        assert!(
            matches!(
                report.findings(),
                [Finding { outcome: Outcome::EvaluationError { error }, .. }] if error == CALL_FAILS
            ),
            "{search}"
        );
    }
    Ok(())
}

#[test]
fn a_search_for_attributes_finds_them_all() -> TestResult {
    assert!(holds("count(//@k) = 5")?);
    Ok(())
}

#[test]
fn a_rule_context_is_searched_in_the_whole_document() -> TestResult {
    // A relative context is searched with `//`. The rule sees every `b`, and
    // each is asked for its own first child.
    let rules = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
                     <pattern>
                       <rule context="b[@k = '1']">
                         <assert id="R-1" flag="fatal" test="@id = 'b1'">text</assert>
                       </rule>
                     </pattern>
                   </schema>"#;
    let report = Schema::compile(rules, &NoIncludes)?.validate(DOCUMENT)?;
    let places: Vec<&str> = report
        .findings()
        .iter()
        .map(|finding| finding.location.as_str())
        .collect();

    assert_eq!(places, ["/doc[1]/a[2]/b[1]", "/doc[1]/a[2]/b[1]/b[2]"]);
    Ok(())
}
