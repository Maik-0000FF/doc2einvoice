// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Which predicates a rule file puts on a step after a search through all
//! descendants (`//`), as [`Schema::search_predicates`] reports them.
//!
//! Input table. Each row is one test below.
//!
//! | rule file                                             | reported                         |
//! |-------------------------------------------------------|----------------------------------|
//! | a test with `//a[k = '1']`                            | `k = '1'`                        |
//! | a search without a predicate, `//a`                   | nothing                          |
//! | two predicates on one step, `//a[b][c]`               | `b` and `c`                      |
//! | a predicate laid out over several lines               | with single spaces               |
//! | a search inside the predicate of another search       | both predicates                  |
//! | a search inside a function call, a condition, a loop  | its predicate                    |
//! | a search for attributes, `//@k[. = '1']`              | nothing: no child step           |
//! | a predicate on a step further on, `//a/b[c]`          | nothing: `b` follows no search   |
//! | a predicate on a step that follows no search, `a[b]`  | nothing                          |
//! | a rule context that is not anchored, `a[k = '1']`     | `k = '1'`: it is searched for    |
//! | a rule context anchored at the root, `/doc/a[b]`      | nothing                          |
//! | a variable and the value shown in an assertion text   | their predicates                 |
//! | the body of a function a test calls                   | its predicate                    |
//! | the same predicate in several places                  | once                             |

use doc2einvoice_schematron::{NoIncludes, Schema};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A rule file with one rule on `context` and what `rule_body` holds.
fn rules(context: &str, rule_body: &str) -> String {
    format!(
        r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
             <ns prefix="u" uri="utils"/>
             <xsl:function name="u:marked">
               <xsl:param name="node"/>
               <xsl:sequence select="exists($node//mark[@on = 'yes'])"/>
             </xsl:function>
             <pattern><rule context="{context}">{rule_body}</rule></pattern>
           </schema>"#
    )
}

/// A rule file with one assertion on `/doc`.
fn rules_with_test(test: &str) -> String {
    rules(
        "/doc",
        &format!(r#"<assert id="R-1" flag="fatal" test="{test}">text</assert>"#),
    )
}

fn reported(rules: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(Schema::compile(rules, &NoIncludes)?
        .search_predicates()?
        .into_iter()
        .collect())
}

#[test]
fn a_predicate_after_a_search_is_reported() -> TestResult {
    assert_eq!(reported(&rules_with_test("//a[k = '1']"))?, ["k = '1'"]);
    Ok(())
}

#[test]
fn a_search_without_a_predicate_reports_nothing() -> TestResult {
    assert!(reported(&rules_with_test("//a"))?.is_empty());
    Ok(())
}

#[test]
fn every_predicate_of_a_step_is_reported() -> TestResult {
    assert_eq!(reported(&rules_with_test("//a[b][c]"))?, ["b", "c"]);
    Ok(())
}

#[test]
fn white_space_in_a_predicate_is_collapsed() -> TestResult {
    let test = "//a[k\n                   =   '1']";

    assert_eq!(reported(&rules_with_test(test))?, ["k = '1'"]);
    Ok(())
}

#[test]
fn a_search_inside_a_predicate_is_reported_too() -> TestResult {
    assert_eq!(
        reported(&rules_with_test("//a[.//b[c] = '1']"))?,
        [".//b[c] = '1'", "c"]
    );
    Ok(())
}

#[test]
fn a_search_is_found_wherever_it_stands() -> TestResult {
    let tests = [
        ("count(//a[in-a-call]) = 0", "in-a-call"),
        ("if (x) then //a[in-a-branch] else y", "in-a-branch"),
        ("for $n in //a[in-a-loop] return $n", "in-a-loop"),
        (
            "some $n in x satisfies //a[in-a-condition]",
            "in-a-condition",
        ),
        ("let $n := //a[in-a-binding] return $n", "in-a-binding"),
        ("x ! //a[in-a-mapping]", "in-a-mapping"),
        ("(x, //a[in-a-sequence])", "in-a-sequence"),
        ("x = //a[in-a-comparison]", "in-a-comparison"),
        ("[//a[in-an-array]]", "in-an-array"),
        ("map { 'k': //a[in-a-map] }", "in-a-map"),
    ];

    for (test, predicate) in tests {
        assert_eq!(reported(&rules_with_test(test))?, [predicate], "{test}");
    }
    Ok(())
}

#[test]
fn a_predicate_that_follows_no_search_is_not_reported() -> TestResult {
    for test in ["//@k[. = '1']", "//a/b[c]", "a[b]", "/doc/a[b]"] {
        assert!(reported(&rules_with_test(test))?.is_empty(), "{test}");
    }
    Ok(())
}

#[test]
fn a_rule_context_that_is_searched_for_is_reported() -> TestResult {
    let assertion = r#"<assert id="R-1" flag="fatal" test="true()">text</assert>"#;

    assert_eq!(reported(&rules("a[k = '1']", assertion))?, ["k = '1'"]);
    assert!(reported(&rules("/doc/a[b]", assertion))?.is_empty());
    Ok(())
}

#[test]
fn variables_and_shown_values_are_looked_into() -> TestResult {
    let body = r#"<let name="marked" value="//a[in-a-variable]"/>
                  <assert id="R-1" flag="fatal" test="true()">
                    <value-of select="//a[in-a-text]"/>
                  </assert>"#;

    assert_eq!(
        reported(&rules("/doc", body))?,
        ["in-a-text", "in-a-variable"]
    );
    Ok(())
}

#[test]
fn the_body_of_a_called_function_is_looked_into() -> TestResult {
    assert_eq!(reported(&rules_with_test("u:marked(.)"))?, ["@on = 'yes'"]);
    Ok(())
}

#[test]
fn a_predicate_used_twice_is_reported_once() -> TestResult {
    assert_eq!(
        reported(&rules_with_test("//a[k = '1'] or //b[k = '1']"))?,
        ["k = '1'"]
    );
    Ok(())
}
