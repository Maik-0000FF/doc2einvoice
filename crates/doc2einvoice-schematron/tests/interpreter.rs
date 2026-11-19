// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Behaviour of the interpreter on small rule files.
//!
//! Input table. Each row is one test below.
//!
//! | rule file / document                                   | expected                               |
//! |--------------------------------------------------------|----------------------------------------|
//! | assertion holds                                        | no finding                             |
//! | assertion is false                                     | `Violated`, with id, flag, text, place |
//! | assertion false on the 2nd of two equal siblings       | location names position 2              |
//! | two rules of one pattern match the same node           | only the first rule checks it          |
//! | the same node in two patterns                          | both patterns check it                 |
//! | schema, pattern and rule variables                     | all readable in the test               |
//! | user-defined function, also recursive                  | evaluated                              |
//! | function reads a global variable                       | evaluated                              |
//! | included file declares a variable                      | readable                               |
//! | test raises an error (text where a number is needed)   | `EvaluationError`, counted as finding  |
//! | variable cannot be evaluated, assertion reads it       | `EvaluationError`                      |
//! | variable cannot be evaluated, assertion ignores it     | no finding                             |
//! | a variable derived from a failed one, read by a test   | `EvaluationError`                      |
//! | a rule variable derived from a failed global one       | `EvaluationError`                      |
//! | a rule declares a variable named like a global one     | `SchemaError::DuplicateVariable`       |
//! | two patterns each declare a variable of the same name  | `SchemaError::DuplicateVariable`       |
//! | a comment with an apostrophe before a function call    | the call is still evaluated            |
//! | a rule context that reads a failed variable            | every assertion of the rule reported   |
//! | a rule context that raises an error itself             | every assertion of the rule reported   |
//! | test reads `$brokenIsDE`, only `$broken` failed        | evaluated, no false finding            |
//! | test binds `$broken` itself (`for $broken in`)         | evaluated, no false finding            |
//! | test reads two failed variables                        | error of the one declared first        |
//! | test calls a function that reads a failed variable     | `EvaluationError`                      |
//! | text of an assertion with `name` and `value-of`        | filled in for the node                 |
//! | context is a group of absolute paths with a predicate  | selects the nodes once                 |
//! | rule file with a `report`                              | `SchemaError::UnsupportedElement`      |
//! | filter matches several nodes (engine patch 1)          | true, no error                         |
//! | left operand decides `or` / `and` (engine patch 2)     | right operand's error not raised       |
//! | left operand does not decide                           | right operand's error is raised        |
//! | argument-less `normalize-space()` / `string-length()`  | evaluated on the context node          |
//! | document is not well-formed                            | `DocumentError::Xml`                   |
//! | rule file with an expression that is not XPath         | `SchemaError::Expression`, names the id|

use doc2einvoice_schematron::{
    DocumentError, Finding, IncludedFiles, NoIncludes, Outcome, Schema, SchemaError,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CONVERSION_ERROR: &str = "FORG0001";

/// Wraps patterns and declarations into a schema with the prefix `u` for
/// user-defined functions.
fn schema_with(body: &str) -> String {
    format!(
        r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
             <ns prefix="u" uri="utils"/>
             {body}
           </schema>"#
    )
}

/// A schema with one rule on `/doc` holding one assertion `R-1`.
fn single_assertion(test: &str) -> String {
    schema_with(&format!(
        r#"<pattern><rule context="/doc"><assert id="R-1" flag="fatal" test="{test}">text</assert></rule></pattern>"#
    ))
}

fn findings(rules: &str, document: &str) -> Result<Vec<Finding>, Box<dyn std::error::Error>> {
    Ok(Schema::compile(rules, &NoIncludes)?
        .validate(document)?
        .findings()
        .to_vec())
}

fn ids(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.assertion_id.as_str())
        .collect()
}

#[test]
fn an_assertion_that_holds_yields_no_finding() -> TestResult {
    let schema = Schema::compile(&single_assertion("total"), &NoIncludes)?;
    let report = schema.validate("<doc><total>1</total></doc>")?;

    assert!(report.is_clean());
    Ok(())
}

#[test]
fn a_false_assertion_is_reported_with_its_details() -> TestResult {
    let found = findings(&single_assertion("total"), "<doc/>")?;

    assert_eq!(
        found,
        vec![Finding {
            assertion_id: "R-1".to_owned(),
            flag: Some("fatal".to_owned()),
            message: "text".to_owned(),
            location: "/doc[1]".to_owned(),
            outcome: Outcome::Violated,
        }]
    );
    Ok(())
}

#[test]
fn the_location_names_the_position_among_equal_siblings() -> TestResult {
    let rules = schema_with(
        r#"<pattern><rule context="line"><assert id="R-1" test="amount">text</assert></rule></pattern>"#,
    );
    let found = findings(&rules, "<doc><line><amount/></line><other/><line/></doc>")?;

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].location, "/doc[1]/line[2]");
    Ok(())
}

#[test]
fn within_a_pattern_only_the_first_matching_rule_checks_a_node() -> TestResult {
    let rules = schema_with(
        r#"<pattern>
             <rule context="/doc"><assert id="FIRST" test="false()">text</assert></rule>
             <rule context="doc"><assert id="SECOND" test="false()">text</assert></rule>
           </pattern>"#,
    );

    assert_eq!(ids(&findings(&rules, "<doc/>")?), ["FIRST"]);
    Ok(())
}

#[test]
fn every_pattern_checks_a_node_on_its_own() -> TestResult {
    let rules = schema_with(
        r#"<pattern><rule context="/doc"><assert id="ONE" test="false()">text</assert></rule></pattern>
           <pattern><rule context="/doc"><assert id="TWO" test="false()">text</assert></rule></pattern>"#,
    );

    assert_eq!(ids(&findings(&rules, "<doc/>")?), ["ONE", "TWO"]);
    Ok(())
}

#[test]
fn variables_of_every_level_are_readable() -> TestResult {
    let rules = schema_with(
        r#"<let name="top" value="1"/>
           <pattern><let name="shared" value="$top + 1"/></pattern>
           <pattern>
             <rule context="/doc">
               <let name="local" value="$shared + number(value)"/>
               <assert id="R-1" test="$top = 1 and $shared = 2 and $local = 5">text</assert>
             </rule>
           </pattern>"#,
    );

    assert!(findings(&rules, "<doc><value>3</value></doc>")?.is_empty());
    assert_eq!(
        ids(&findings(&rules, "<doc><value>4</value></doc>")?),
        ["R-1"]
    );
    Ok(())
}

#[test]
fn user_defined_functions_are_evaluated() -> TestResult {
    let rules = schema_with(
        r#"<xsl:function name="u:double"><xsl:param name="v" as="xs:decimal"/><xsl:sequence select="$v * 2"/></xsl:function>
           <xsl:function name="u:sum-to"><xsl:param name="n"/><xsl:sequence select="if ($n le 0) then 0 else $n + u:sum-to($n - 1)"/></xsl:function>
           <pattern><rule context="/doc">
             <assert id="DOUBLE" test="u:double(value) = 6">text</assert>
             <assert id="RECURSIVE" test="u:sum-to(4) = 10">text</assert>
           </rule></pattern>"#,
    );

    assert!(findings(&rules, "<doc><value>3</value></doc>")?.is_empty());
    assert_eq!(
        ids(&findings(&rules, "<doc><value>4</value></doc>")?),
        ["DOUBLE"]
    );
    Ok(())
}

#[test]
fn a_function_may_read_a_global_variable() -> TestResult {
    let rules = schema_with(
        r#"<xsl:function name="u:allowed"><xsl:param name="v"/><xsl:sequence select="matches($v, $PATTERN)"/></xsl:function>
           <pattern><let name="PATTERN" value="'^[0-9]+$'"/></pattern>
           <pattern><rule context="/doc"><assert id="R-1" test="u:allowed(string(value))">text</assert></rule></pattern>"#,
    );

    assert!(findings(&rules, "<doc><value>42</value></doc>")?.is_empty());
    assert_eq!(
        ids(&findings(&rules, "<doc><value>x</value></doc>")?),
        ["R-1"]
    );
    Ok(())
}

#[test]
fn an_included_file_becomes_part_of_the_rules() -> TestResult {
    let rules = schema_with(
        r#"<include href="common.sch"/>
           <pattern><rule context="/doc"><assert id="R-1" test="$limit = 7">text</assert></rule></pattern>"#,
    );
    let files = [(
        "common.sch",
        r#"<?xml version="1.0"?><pattern xmlns="http://purl.oclc.org/dsdl/schematron"><let name="limit" value="7"/></pattern>"#,
    )];

    let report = Schema::compile(&rules, &IncludedFiles(&files))?.validate("<doc/>")?;

    assert!(report.is_clean());
    Ok(())
}

#[test]
fn a_test_that_cannot_be_evaluated_counts_as_a_finding() -> TestResult {
    let found = findings(
        &single_assertion("amount &lt; 10"),
        "<doc><amount>abc</amount></doc>",
    )?;

    assert_eq!(ids(&found), ["R-1"]);
    assert_eq!(
        found[0].outcome,
        Outcome::EvaluationError {
            error: CONVERSION_ERROR.to_owned()
        }
    );
    Ok(())
}

#[test]
fn an_assertion_reading_an_unusable_variable_is_reported() -> TestResult {
    let rules = schema_with(
        r#"<let name="broken" value="xs:integer(/doc/amount)"/>
           <pattern><rule context="/doc">
             <assert id="READS" test="$broken = 1">text</assert>
             <assert id="IGNORES" test="amount">text</assert>
           </rule></pattern>"#,
    );
    let found = findings(&rules, "<doc><amount>abc</amount></doc>")?;

    assert_eq!(ids(&found), ["READS"]);
    assert_eq!(
        found[0].outcome,
        Outcome::EvaluationError {
            error: CONVERSION_ERROR.to_owned()
        }
    );
    Ok(())
}

#[test]
fn a_filter_that_matches_several_nodes_is_true() -> TestResult {
    let rules = single_assertion("count(group[item]) = 1");

    assert!(findings(&rules, "<doc><group><item/><item/><item/></group></doc>")?.is_empty());
    Ok(())
}

#[test]
fn the_left_operand_alone_decides_or_and_and() -> TestResult {
    let document = "<doc><amount>abc</amount></doc>";

    assert!(
        findings(
            &single_assertion("not(missing) or amount &lt; 10"),
            document
        )?
        .is_empty()
    );
    assert!(
        findings(
            &single_assertion("not(missing and amount &lt; 10)"),
            document
        )?
        .is_empty()
    );
    Ok(())
}

#[test]
fn the_right_operand_is_evaluated_when_the_left_does_not_decide() -> TestResult {
    let document = "<doc><amount>abc</amount></doc>";
    let expected = Outcome::EvaluationError {
        error: CONVERSION_ERROR.to_owned(),
    };

    assert_eq!(
        findings(&single_assertion("missing or amount &lt; 10"), document)?[0].outcome,
        expected
    );
    assert_eq!(
        findings(&single_assertion("amount and amount &lt; 10"), document)?[0].outcome,
        expected
    );
    Ok(())
}

#[test]
fn logical_operators_keep_their_meaning() -> TestResult {
    let document = "<doc><a>1</a><b>2</b></doc>";

    for holds in [
        "a or missing",
        "missing or a",
        "a and b",
        "not(a and missing)",
        "not(missing or missing)",
    ] {
        assert!(
            findings(&single_assertion(holds), document)?.is_empty(),
            "`{holds}` must hold"
        );
    }
    for fails in ["a and missing", "missing and a", "missing or missing"] {
        assert_eq!(
            ids(&findings(&single_assertion(fails), document)?),
            ["R-1"],
            "`{fails}` must not hold"
        );
    }
    Ok(())
}

#[test]
fn argument_less_string_functions_read_the_context_node() -> TestResult {
    let rules = schema_with(
        r#"<pattern><rule context="code">
             <assert id="R-1" test="string-length() = 2 and normalize-space() = 'DE'">text</assert>
           </rule></pattern>"#,
    );

    assert!(findings(&rules, "<doc><code>DE</code></doc>")?.is_empty());
    assert_eq!(
        ids(&findings(&rules, "<doc><code>DEU</code></doc>")?),
        ["R-1"]
    );
    Ok(())
}

#[test]
fn a_malformed_document_is_refused() -> TestResult {
    let schema = Schema::compile(&single_assertion("total"), &NoIncludes)?;

    assert!(matches!(
        schema.validate("<doc>"),
        Err(DocumentError::Xml(_))
    ));
    Ok(())
}

#[test]
fn a_rule_file_with_an_invalid_expression_is_refused_as_a_whole() {
    let result = Schema::compile(&single_assertion("total +"), &NoIncludes);

    assert!(matches!(
        result,
        Err(SchemaError::Expression { subject, .. }) if subject.contains("R-1")
    ));
}

#[test]
fn the_assertion_ids_are_listed_in_document_order() -> TestResult {
    let rules = schema_with(
        r#"<pattern><rule context="/doc">
             <assert id="B" test="true()">text</assert>
             <assert id="A" test="true()">text</assert>
           </rule></pattern>"#,
    );
    let schema = Schema::compile(&rules, &NoIncludes)?;

    assert_eq!(schema.assertion_ids().collect::<Vec<_>>(), ["B", "A"]);
    Ok(())
}

const DIVISION_ERROR: &str = "FOAR0001";

/// A variable that cannot be evaluated: the amount is not a number.
const BROKEN: &str = r#"<let name="broken" value="xs:integer(/doc/amount)"/>"#;
const DOCUMENT_WITH_TEXT_AMOUNT: &str = "<doc><amount>abc</amount></doc>";

fn evaluation_error(error: &str) -> Outcome {
    Outcome::EvaluationError {
        error: error.to_owned(),
    }
}

#[test]
fn a_failure_is_inherited_by_derived_variables() -> TestResult {
    let rules = schema_with(&format!(
        r#"{BROKEN}
           <let name="derived" value="exists($broken)"/>
           <let name="further" value="not($derived)"/>
           <pattern><rule context="/doc">
             <assert id="DERIVED" test="not($derived)">text</assert>
             <assert id="FURTHER" test="$further or true()">text</assert>
             <assert id="UNRELATED" test="amount">text</assert>
           </rule></pattern>"#
    ));
    let found = findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?;

    assert_eq!(ids(&found), ["DERIVED", "FURTHER"]);
    assert!(
        found
            .iter()
            .all(|finding| finding.outcome == evaluation_error(CONVERSION_ERROR))
    );
    Ok(())
}

#[test]
fn a_failure_is_inherited_by_rule_variables() -> TestResult {
    let rules = schema_with(&format!(
        r#"{BROKEN}
           <pattern><rule context="/doc">
             <let name="local" value="$broken + 1"/>
             <let name="other" value="1"/>
             <assert id="LOCAL" test="$local = 2">text</assert>
             <assert id="OTHER" test="$other = 1">text</assert>
           </rule></pattern>"#
    ));
    let found = findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?;

    assert_eq!(ids(&found), ["LOCAL"]);
    assert_eq!(found[0].outcome, evaluation_error(CONVERSION_ERROR));
    Ok(())
}

#[test]
fn a_rule_whose_context_reads_a_failed_variable_is_reported_as_a_whole() -> TestResult {
    let rules = schema_with(&format!(
        r#"{BROKEN}
           <pattern>
             <rule context="/doc[not($broken = 1)]">
               <assert id="ONE" test="true()">The element <name/> exists.</assert>
               <assert id="TWO" test="true()">text</assert>
             </rule>
             <rule context="/doc"><assert id="OTHER-RULE" test="amount">text</assert></rule>
           </pattern>"#
    ));
    let found = findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?;

    assert_eq!(ids(&found), ["ONE", "TWO"]);
    assert!(found.iter().all(|finding| finding.location == "/"));
    assert!(
        found
            .iter()
            .all(|finding| finding.outcome == evaluation_error(CONVERSION_ERROR))
    );
    assert_eq!(found[0].message, "The element exists.");
    Ok(())
}

#[test]
fn a_rule_whose_context_raises_an_error_is_reported_as_a_whole() -> TestResult {
    let rules = schema_with(
        r#"<pattern><rule context="/doc[xs:integer(amount) = 1]">
             <assert id="R-1" test="true()">text</assert>
           </rule></pattern>"#,
    );
    let found = findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?;

    assert_eq!(ids(&found), ["R-1"]);
    assert_eq!(found[0].outcome, evaluation_error(CONVERSION_ERROR));
    Ok(())
}

#[test]
fn a_name_that_only_starts_like_a_failed_variable_is_not_affected() -> TestResult {
    let rules = schema_with(&format!(
        r#"{BROKEN}
           <let name="brokenIsDE" value="true()"/>
           <pattern><rule context="/doc">
             <assert id="R-1" test="$brokenIsDE">text</assert>
           </rule></pattern>"#
    ));

    assert!(findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?.is_empty());
    Ok(())
}

#[test]
fn a_name_the_test_binds_itself_is_not_the_failed_variable() -> TestResult {
    let rules = schema_with(&format!(
        r#"{BROKEN}
           <pattern><rule context="/doc">
             <assert id="R-1" test="every $broken in (1, 2) satisfies $broken gt 0">text</assert>
           </rule></pattern>"#
    ));

    assert!(findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?.is_empty());
    Ok(())
}

#[test]
fn of_two_failed_variables_the_one_declared_first_is_named() -> TestResult {
    let rules = schema_with(
        r#"<let name="first" value="xs:integer(/doc/amount)"/>
           <let name="second" value="1 idiv 0"/>
           <pattern><rule context="/doc">
             <assert id="BOTH" test="$second = 1 or $first = 1">text</assert>
             <assert id="SECOND" test="$second = 1">text</assert>
           </rule></pattern>"#,
    );
    let found = findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?;

    assert_eq!(ids(&found), ["BOTH", "SECOND"]);
    assert_eq!(found[0].outcome, evaluation_error(CONVERSION_ERROR));
    assert_eq!(found[1].outcome, evaluation_error(DIVISION_ERROR));
    Ok(())
}

#[test]
fn a_function_that_reads_a_failed_variable_fails_its_callers() -> TestResult {
    let rules = schema_with(&format!(
        r#"<xsl:function name="u:limit"><xsl:sequence select="$broken"/></xsl:function>
           <xsl:function name="u:one"><xsl:sequence select="1"/></xsl:function>
           {BROKEN}
           <pattern><rule context="/doc">
             <assert id="CALLS" test="u:limit() = 1 or true()">text</assert>
             <assert id="OTHER" test="u:one() = 1">text</assert>
           </rule></pattern>"#
    ));
    let found = findings(&rules, DOCUMENT_WITH_TEXT_AMOUNT)?;

    assert_eq!(ids(&found), ["CALLS"]);
    assert_eq!(found[0].outcome, evaluation_error(CONVERSION_ERROR));
    Ok(())
}

#[test]
fn the_text_of_a_finding_is_filled_in_for_its_node() -> TestResult {
    let rules = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">
             <ns prefix="c" uri="urn:c"/>
             <pattern><rule context="c:line">
               <assert id="R-1" test="false()">The element <name/> has
                 the <emph>amount</emph> <value-of select="c:amount"/>
                 and codes <value-of select="c:code"/>.</assert>
             </rule></pattern>
           </schema>"#;
    let document = r#"<d xmlns:c="urn:c"><c:line><c:amount>12</c:amount><c:code>A</c:code><c:code>B</c:code></c:line></d>"#;
    let found = findings(rules, document)?;

    assert_eq!(
        found[0].message,
        "The element c:line has the amount 12 and codes A B."
    );
    Ok(())
}

#[test]
fn a_group_of_absolute_paths_selects_each_node_once() -> TestResult {
    let rules = schema_with(
        r#"<let name="active" value="true()"/>
           <pattern><rule context="(/doc | /other)[$active]">
             <assert id="R-1" test="false()">text</assert>
           </rule></pattern>"#,
    );
    let found = findings(&rules, "<doc><a/><b><c/></b></doc>")?;

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].location, "/doc[1]");
    Ok(())
}

#[test]
fn a_rule_file_with_a_report_is_refused() {
    let rules = schema_with(
        r#"<pattern><rule context="/doc"><report id="R-1" test="true()">text</report></rule></pattern>"#,
    );

    assert!(matches!(
        Schema::compile(&rules, &NoIncludes),
        Err(SchemaError::UnsupportedElement { element, .. }) if element == "report"
    ));
}

#[test]
fn two_patterns_with_a_variable_of_the_same_name_are_refused() {
    let rules = schema_with(
        r#"<pattern><let name="x" value="1"/><rule context="/doc"><assert id="A" test="$x = 1">text</assert></rule></pattern>
           <pattern><let name="x" value="2"/></pattern>"#,
    );

    assert!(matches!(
        Schema::compile(&rules, &NoIncludes),
        Err(SchemaError::DuplicateVariable { name }) if name == "x"
    ));
}

#[test]
fn a_comment_with_an_apostrophe_does_not_disable_what_follows() -> TestResult {
    let rules = schema_with(
        r#"<xsl:function name="u:one"><xsl:sequence select="1"/></xsl:function>
           <pattern><rule context="code">
             <assert id="R-1" test="(: don't skip :) u:one() = 1 and string-length() = 2">text</assert>
           </rule></pattern>"#,
    );

    assert!(findings(&rules, "<doc><code>DE</code></doc>")?.is_empty());
    assert_eq!(
        ids(&findings(&rules, "<doc><code>DEU</code></doc>")?),
        ["R-1"]
    );
    Ok(())
}

#[test]
fn a_rule_variable_named_like_a_global_one_is_refused() {
    let rules = schema_with(
        r#"<let name="x" value="1"/>
           <pattern><rule context="/doc">
             <let name="x" value="2"/>
             <assert id="R-1" test="$x = 2">text</assert>
           </rule></pattern>"#,
    );

    assert!(matches!(
        Schema::compile(&rules, &NoIncludes),
        Err(SchemaError::DuplicateVariable { name }) if name == "x"
    ));
}
