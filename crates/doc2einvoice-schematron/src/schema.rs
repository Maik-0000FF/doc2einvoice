// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading a rule file into its parts, before any expression is compiled.
//!
//! Reading is strict. Every element is either implemented, known to have no
//! effect on which rules are reported, or refused: a construct that was
//! silently skipped could be a rule that is never checked.
//!
//! | where     | implemented                 | without effect, skipped | refused        |
//! |-----------|-----------------------------|-------------------------|----------------|
//! | `schema`  | `ns`, `let`, `pattern`, XSLT `function` | `title`, `p`, `phase` | everything else |
//! | `pattern` | `rule`, `let`               | `title`, `p`            | everything else, `abstract`, `is-a` |
//! | `rule`    | `assert`, `let`             |                         | everything else (`report`, `extends`), `abstract` |
//! | `assert`  | text, `name`, `value-of`, `emph`, `span`, `dir` |     | everything else, `name` with `path` |
//!
//! A `phase` only names a selection of patterns. Without a `defaultPhase` all
//! patterns are active, which is what this interpreter evaluates; a schema
//! that selects a default phase is refused.

use roxmltree::{Document, Node};

use crate::error::SchemaError;
use crate::expression::{context_selection, normalize};
use crate::functions::{self, Function};

pub(crate) const SCHEMATRON_NAMESPACE: &str = "http://purl.oclc.org/dsdl/schematron";
const XSLT_NAMESPACE: &str = "http://www.w3.org/1999/XSL/Transform";
const XML_SCHEMA_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";
/// Prefix the rule files use for XML Schema types without always declaring it.
const XML_SCHEMA_PREFIX: &str = "xs";

const ELEMENT_SCHEMA: &str = "schema";
const ELEMENT_NAMESPACE: &str = "ns";
const ELEMENT_PATTERN: &str = "pattern";
const ELEMENT_RULE: &str = "rule";
const ELEMENT_ASSERT: &str = "assert";
const ELEMENT_LET: &str = "let";
const ELEMENT_FUNCTION: &str = "function";
const ELEMENT_NAME: &str = "name";
const ELEMENT_VALUE_OF: &str = "value-of";

/// Elements that document a schema and have no effect on its rules.
const DOCUMENTATION_ELEMENTS: [&str; 2] = ["title", "p"];
/// See the module documentation for why a `phase` may be skipped.
const ELEMENT_PHASE: &str = "phase";
/// Elements that only mark up the text of a message.
const TEXT_MARKUP_ELEMENTS: [&str; 3] = ["emph", "span", "dir"];

const ATTRIBUTE_PREFIX: &str = "prefix";
const ATTRIBUTE_URI: &str = "uri";
const ATTRIBUTE_NAME: &str = "name";
const ATTRIBUTE_VALUE: &str = "value";
const ATTRIBUTE_CONTEXT: &str = "context";
const ATTRIBUTE_TEST: &str = "test";
const ATTRIBUTE_ID: &str = "id";
const ATTRIBUTE_FLAG: &str = "flag";
const ATTRIBUTE_SELECT: &str = "select";
const ATTRIBUTE_PATH: &str = "path";
const ATTRIBUTE_ABSTRACT: &str = "abstract";
const ATTRIBUTE_IS_A: &str = "is-a";
const ATTRIBUTE_DEFAULT_PHASE: &str = "defaultPhase";
const ATTRIBUTE_QUERY_BINDING: &str = "queryBinding";

const ABSTRACT_TRUE: &str = "true";
/// The value of `defaultPhase` that selects every pattern.
const PHASE_ALL: &str = "#ALL";
/// Query bindings whose expressions are XPath 2.0 or later, which is what the
/// engine evaluates. The original `xslt` binding means XPath 1.0, whose rules
/// for comparisons and type conversion differ.
const SUPPORTED_QUERY_BINDINGS: [&str; 5] = ["xslt2", "xslt3", "xpath2", "xpath3", "xpath31"];

/// A rule file, read but not yet compiled.
pub(crate) struct ParsedSchema {
    pub(crate) namespaces: Vec<(String, String)>,
    pub(crate) functions: Vec<Function>,
    /// Variables visible to every rule, in the order they are declared.
    pub(crate) global_variables: Vec<Variable>,
    pub(crate) patterns: Vec<Vec<ParsedRule>>,
}

/// A `let`: a name and the expression of its value.
pub(crate) struct Variable {
    pub(crate) name: String,
    pub(crate) expression: String,
}

pub(crate) struct ParsedRule {
    /// The context as written, for messages.
    pub(crate) context: String,
    /// Selects every node the context matches.
    pub(crate) selection: String,
    pub(crate) variables: Vec<Variable>,
    pub(crate) assertions: Vec<ParsedAssertion>,
}

pub(crate) struct ParsedAssertion {
    pub(crate) id: String,
    pub(crate) flag: Option<String>,
    pub(crate) message: Vec<MessagePart>,
    pub(crate) test: String,
}

/// A piece of the text of an assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessagePart {
    /// Text as written.
    Text(String),
    /// The name of the node the assertion is checked on (`name`).
    ContextName,
    /// The value of an expression on that node (`value-of`).
    ValueOf(String),
}

/// Reads a rule file whose includes have already been expanded.
pub(crate) fn parse(source: &str) -> Result<ParsedSchema, SchemaError> {
    let document = Document::parse(source)?;
    let root = document.root_element();
    if !is_schematron(&root, ELEMENT_SCHEMA) {
        return Err(SchemaError::NotASchema);
    }
    check_schema_attributes(&root)?;

    let function_nodes: Vec<Node> = elements(&root)
        .filter(|node| is_xslt(node, ELEMENT_FUNCTION))
        .collect();
    let function_names: Vec<String> = function_nodes
        .iter()
        .filter_map(|node| node.attribute(ATTRIBUTE_NAME))
        .map(str::to_owned)
        .collect();

    let mut namespaces = Vec::new();
    let mut global_variables = Vec::new();
    let mut patterns = Vec::new();
    for child in elements(&root) {
        if is_schematron(&child, ELEMENT_NAMESPACE) {
            namespaces.push((
                non_empty(&child, ELEMENT_NAMESPACE, ATTRIBUTE_PREFIX)?.to_owned(),
                non_empty(&child, ELEMENT_NAMESPACE, ATTRIBUTE_URI)?.to_owned(),
            ));
        } else if is_schematron(&child, ELEMENT_LET) {
            global_variables.push(parse_variable(&child, &function_names)?);
        } else if is_schematron(&child, ELEMENT_PATTERN) {
            patterns.push(parse_pattern(
                &child,
                &function_names,
                &mut global_variables,
            )?);
        } else if !(is_xslt(&child, ELEMENT_FUNCTION)
            || is_schematron(&child, ELEMENT_PHASE)
            || is_documentation(&child))
        {
            return Err(unsupported(&child, ELEMENT_SCHEMA));
        }
    }
    refuse_duplicates(&global_variables)?;
    refuse_hidden_globals(&patterns, &global_variables)?;
    if !namespaces
        .iter()
        .any(|(prefix, _)| prefix == XML_SCHEMA_PREFIX)
    {
        namespaces.push((
            XML_SCHEMA_PREFIX.to_owned(),
            XML_SCHEMA_NAMESPACE.to_owned(),
        ));
    }

    Ok(ParsedSchema {
        namespaces,
        functions: functions::translate_all(&function_nodes, &function_names)?,
        global_variables,
        patterns,
    })
}

fn check_schema_attributes(root: &Node) -> Result<(), SchemaError> {
    if let Some(phase) = root
        .attribute(ATTRIBUTE_DEFAULT_PHASE)
        .filter(|phase| *phase != PHASE_ALL)
    {
        return Err(SchemaError::UnsupportedAttribute {
            element: ELEMENT_SCHEMA,
            attribute: ATTRIBUTE_DEFAULT_PHASE,
            value: phase.to_owned(),
        });
    }
    if let Some(binding) = root
        .attribute(ATTRIBUTE_QUERY_BINDING)
        .filter(|binding| !SUPPORTED_QUERY_BINDINGS.contains(binding))
    {
        return Err(SchemaError::UnsupportedAttribute {
            element: ELEMENT_SCHEMA,
            attribute: ATTRIBUTE_QUERY_BINDING,
            value: binding.to_owned(),
        });
    }
    Ok(())
}

/// Reads the rules of a pattern. The variables a pattern declares are added to
/// the global ones: the rule files use a pattern without rules to declare
/// variables for all patterns. Because they share one scope, two patterns
/// cannot each have a variable of the same name; such a rule file is refused
/// rather than evaluated with one of the two values.
fn parse_pattern(
    pattern: &Node,
    function_names: &[String],
    global_variables: &mut Vec<Variable>,
) -> Result<Vec<ParsedRule>, SchemaError> {
    refuse_abstract(pattern, ELEMENT_PATTERN)?;
    if let Some(parent) = pattern.attribute(ATTRIBUTE_IS_A) {
        return Err(SchemaError::UnsupportedAttribute {
            element: ELEMENT_PATTERN,
            attribute: ATTRIBUTE_IS_A,
            value: parent.to_owned(),
        });
    }

    let mut rules = Vec::new();
    for child in elements(pattern) {
        if is_schematron(&child, ELEMENT_RULE) {
            rules.push(parse_rule(&child, function_names)?);
        } else if is_schematron(&child, ELEMENT_LET) {
            global_variables.push(parse_variable(&child, function_names)?);
        } else if !is_documentation(&child) {
            return Err(unsupported(&child, ELEMENT_PATTERN));
        }
    }
    Ok(rules)
}

fn parse_rule(rule: &Node, function_names: &[String]) -> Result<ParsedRule, SchemaError> {
    refuse_abstract(rule, ELEMENT_RULE)?;
    let context = required(rule, ELEMENT_RULE, ATTRIBUTE_CONTEXT)?;

    let mut variables = Vec::new();
    let mut assertions = Vec::new();
    for child in elements(rule) {
        if is_schematron(&child, ELEMENT_LET) {
            variables.push(parse_variable(&child, function_names)?);
        } else if is_schematron(&child, ELEMENT_ASSERT) {
            assertions.push(parse_assertion(&child, function_names)?);
        } else {
            return Err(unsupported(&child, ELEMENT_RULE));
        }
    }
    refuse_duplicates(&variables)?;
    Ok(ParsedRule {
        context: context.to_owned(),
        selection: context_selection(&normalize(context, function_names)),
        variables,
        assertions,
    })
}

fn parse_variable(node: &Node, function_names: &[String]) -> Result<Variable, SchemaError> {
    Ok(Variable {
        name: required(node, ELEMENT_LET, ATTRIBUTE_NAME)?.to_owned(),
        expression: normalize(
            required(node, ELEMENT_LET, ATTRIBUTE_VALUE)?,
            function_names,
        ),
    })
}

fn parse_assertion(
    assertion: &Node,
    function_names: &[String],
) -> Result<ParsedAssertion, SchemaError> {
    let mut message = Vec::new();
    collect_message(assertion, function_names, &mut message)?;
    Ok(ParsedAssertion {
        id: required(assertion, ELEMENT_ASSERT, ATTRIBUTE_ID)?.to_owned(),
        flag: assertion.attribute(ATTRIBUTE_FLAG).map(str::to_owned),
        message,
        test: normalize(
            required(assertion, ELEMENT_ASSERT, ATTRIBUTE_TEST)?,
            function_names,
        ),
    })
}

/// Reads the text of an assertion: plain text, text inside markup elements,
/// and the two elements that are filled in when the assertion is reported.
fn collect_message(
    node: &Node,
    function_names: &[String],
    message: &mut Vec<MessagePart>,
) -> Result<(), SchemaError> {
    for child in node.children() {
        if child.is_text() {
            message.push(MessagePart::Text(
                child.text().unwrap_or_default().to_owned(),
            ));
        } else if is_schematron(&child, ELEMENT_NAME) {
            if let Some(path) = child.attribute(ATTRIBUTE_PATH) {
                return Err(SchemaError::UnsupportedAttribute {
                    element: ELEMENT_NAME,
                    attribute: ATTRIBUTE_PATH,
                    value: path.to_owned(),
                });
            }
            message.push(MessagePart::ContextName);
        } else if is_schematron(&child, ELEMENT_VALUE_OF) {
            message.push(MessagePart::ValueOf(normalize(
                required(&child, ELEMENT_VALUE_OF, ATTRIBUTE_SELECT)?,
                function_names,
            )));
        } else if TEXT_MARKUP_ELEMENTS
            .iter()
            .any(|markup| is_schematron(&child, markup))
        {
            collect_message(&child, function_names, message)?;
        } else if child.is_element() {
            return Err(unsupported(&child, ELEMENT_ASSERT));
        }
    }
    Ok(())
}

fn elements<'a, 'input>(node: &Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(Node::is_element)
}

fn is_schematron(node: &Node, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(SCHEMATRON_NAMESPACE)
}

fn is_xslt(node: &Node, name: &str) -> bool {
    node.is_element()
        && node.tag_name().name() == name
        && node.tag_name().namespace() == Some(XSLT_NAMESPACE)
}

fn is_documentation(node: &Node) -> bool {
    DOCUMENTATION_ELEMENTS
        .iter()
        .any(|element| is_schematron(node, element))
}

fn unsupported(node: &Node, parent: &str) -> SchemaError {
    SchemaError::UnsupportedElement {
        element: node.tag_name().name().to_owned(),
        parent: parent.to_owned(),
    }
}

fn refuse_abstract(node: &Node, element: &'static str) -> Result<(), SchemaError> {
    if node.attribute(ATTRIBUTE_ABSTRACT) == Some(ABSTRACT_TRUE) {
        return Err(SchemaError::UnsupportedAttribute {
            element,
            attribute: ATTRIBUTE_ABSTRACT,
            value: ABSTRACT_TRUE.to_owned(),
        });
    }
    Ok(())
}

/// Refuses variables that share a name within one scope: only one value could
/// be bound to the name, and every expression would silently read that one.
fn refuse_duplicates(variables: &[Variable]) -> Result<(), SchemaError> {
    for (index, variable) in variables.iter().enumerate() {
        if variables[..index]
            .iter()
            .any(|earlier| earlier.name == variable.name)
        {
            return Err(SchemaError::DuplicateVariable {
                name: variable.name.clone(),
            });
        }
    }
    Ok(())
}

/// Refuses a rule variable that takes the name of a global one.
///
/// A final decision, stricter than the reference processor, which lets the
/// rule variable hide the global one inside the rule. Emulating that would
/// need two variables under one name: the user-defined functions a rule calls
/// must keep reading the global one. No published rule file does this, and a
/// file that does is refused rather than evaluated with the wrong variable.
fn refuse_hidden_globals(
    patterns: &[Vec<ParsedRule>],
    global_variables: &[Variable],
) -> Result<(), SchemaError> {
    let hidden = patterns
        .iter()
        .flatten()
        .flat_map(|rule| &rule.variables)
        .find(|variable| {
            global_variables
                .iter()
                .any(|global| global.name == variable.name)
        });
    match hidden {
        Some(variable) => Err(SchemaError::DuplicateVariable {
            name: variable.name.clone(),
        }),
        None => Ok(()),
    }
}

/// A required attribute whose value must not be empty.
fn non_empty<'a>(
    node: &'a Node,
    element: &'static str,
    attribute: &'static str,
) -> Result<&'a str, SchemaError> {
    let value = required(node, element, attribute)?;
    if value.is_empty() {
        return Err(SchemaError::UnsupportedAttribute {
            element,
            attribute,
            value: String::new(),
        });
    }
    Ok(value)
}

fn required<'a>(
    node: &'a Node,
    element: &'static str,
    attribute: &'static str,
) -> Result<&'a str, SchemaError> {
    node.attribute(attribute)
        .ok_or(SchemaError::MissingAttribute { element, attribute })
}

#[cfg(test)]
mod tests {
    use super::{MessagePart, parse};
    use crate::error::SchemaError;

    const RULES: &str = r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="xslt2">
      <title>Example</title>
      <ns prefix="a" uri="urn:a"/>
      <let name="top" value="1"/>
      <phase id="model"><active pattern="rules"/></phase>
      <pattern><let name="shared" value="2"/></pattern>
      <pattern id="rules">
        <rule context="a:item | /a:root">
          <let name="local" value="3"/>
          <assert id="R-1" flag="fatal" test="a:x">[R-1]-The element <name/> has
            <emph>an</emph> x, not <value-of select="a:y"/>.</assert>
        </rule>
      </pattern>
    </schema>"#;

    fn schema_with(body: &str) -> String {
        format!(r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron">{body}</schema>"#)
    }

    fn rule_with(body: &str) -> String {
        schema_with(&format!(
            r#"<pattern><rule context="a">{body}</rule></pattern>"#
        ))
    }

    #[test]
    fn reads_namespaces_and_adds_the_schema_type_prefix() -> Result<(), SchemaError> {
        let schema = parse(RULES)?;

        assert_eq!(
            schema.namespaces,
            vec![
                ("a".to_owned(), "urn:a".to_owned()),
                (
                    "xs".to_owned(),
                    "http://www.w3.org/2001/XMLSchema".to_owned()
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn schema_and_pattern_variables_are_global_in_document_order() -> Result<(), SchemaError> {
        let schema = parse(RULES)?;
        let names: Vec<&str> = schema
            .global_variables
            .iter()
            .map(|v| v.name.as_str())
            .collect();

        assert_eq!(names, ["top", "shared"]);
        Ok(())
    }

    #[test]
    fn reads_rules_with_their_variables_and_assertions() -> Result<(), SchemaError> {
        let schema = parse(RULES)?;
        let rule = &schema.patterns[1][0];

        assert_eq!(schema.patterns[0].len(), 0);
        assert_eq!(rule.selection, "//a:item | /a:root");
        assert_eq!(rule.variables[0].name, "local");
        assert_eq!(rule.assertions[0].id, "R-1");
        assert_eq!(rule.assertions[0].flag.as_deref(), Some("fatal"));
        Ok(())
    }

    #[test]
    fn the_text_of_an_assertion_keeps_its_filled_in_parts() -> Result<(), SchemaError> {
        let schema = parse(RULES)?;
        let message = &schema.patterns[1][0].assertions[0].message;

        assert_eq!(message[1], MessagePart::ContextName);
        assert_eq!(message[3], MessagePart::Text("an".to_owned()));
        assert_eq!(message[5], MessagePart::ValueOf("a:y".to_owned()));
        assert_eq!(message.len(), 7);
        Ok(())
    }

    #[test]
    fn refuses_a_document_that_is_not_a_schema() {
        assert!(matches!(parse("<other/>"), Err(SchemaError::NotASchema)));
    }

    #[test]
    fn refuses_an_assertion_without_an_id() {
        let result = parse(&rule_with(r#"<assert test="b">text</assert>"#));

        assert!(matches!(
            result,
            Err(SchemaError::MissingAttribute {
                element: "assert",
                attribute: "id"
            })
        ));
    }

    // Input table: constructs that are not implemented must refuse the file.
    //
    // | construct                                   | refused as            |
    // |---------------------------------------------|-----------------------|
    // | `report` in a rule                          | UnsupportedElement    |
    // | `extends` in a rule                         | UnsupportedElement    |
    // | unknown element in a rule                   | UnsupportedElement    |
    // | `diagnostics` in the schema                 | UnsupportedElement    |
    // | foreign element in the schema (xsl:variable)| UnsupportedElement    |
    // | `param` in a pattern                        | UnsupportedElement    |
    // | unknown element in an assertion             | UnsupportedElement    |
    // | include that was not expanded               | UnsupportedElement    |
    // | abstract pattern, abstract rule             | UnsupportedAttribute  |
    // | pattern with `is-a`                         | UnsupportedAttribute  |
    // | `name` with `path`                          | UnsupportedAttribute  |
    // | `defaultPhase` other than `#ALL`            | UnsupportedAttribute  |
    // | `queryBinding="xslt"` (XPath 1.0)           | UnsupportedAttribute  |
    // | `ns` with an empty prefix or an empty uri   | UnsupportedAttribute  |
    // | the same variable on the schema twice       | DuplicateVariable     |
    // | the same variable in two patterns           | DuplicateVariable     |
    // | the same variable on schema and pattern     | DuplicateVariable     |
    // | the same variable twice in one rule         | DuplicateVariable     |
    // | a rule variable named like a schema variable| DuplicateVariable     |
    // | a rule variable named like a pattern variable declared after the rule | DuplicateVariable |
    // | the same variable name in two rules         | accepted: two scopes  |
    #[test]
    fn an_element_that_is_not_implemented_refuses_the_file() {
        for rules in [
            rule_with(r#"<report id="R" test="b">text</report>"#),
            rule_with(r#"<extends rule="other"/>"#),
            rule_with("<unheard-of/>"),
            schema_with("<diagnostics/>"),
            schema_with(
                r#"<variable xmlns="http://www.w3.org/1999/XSL/Transform" name="v" select="1"/>"#,
            ),
            schema_with(r#"<pattern><param name="p" value="1"/></pattern>"#),
            rule_with(r#"<assert id="R" test="b">text <unheard-of/></assert>"#),
            schema_with(r#"<include href="other.sch"/>"#),
        ] {
            assert!(
                matches!(parse(&rules), Err(SchemaError::UnsupportedElement { .. })),
                "must be refused: {rules}"
            );
        }
    }

    #[test]
    fn an_attribute_that_asks_for_more_refuses_the_file() {
        for rules in [
            schema_with(r#"<pattern abstract="true"/>"#),
            schema_with(r#"<pattern is-a="other"/>"#),
            schema_with(r#"<pattern><rule abstract="true" context="a"/></pattern>"#),
            rule_with(r#"<assert id="R" test="b"><name path="c"/></assert>"#),
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" defaultPhase="model"/>"#
                .to_owned(),
            r#"<schema xmlns="http://purl.oclc.org/dsdl/schematron" queryBinding="xslt"/>"#
                .to_owned(),
        ] {
            assert!(
                matches!(parse(&rules), Err(SchemaError::UnsupportedAttribute { .. })),
                "must be refused: {rules}"
            );
        }
    }

    #[test]
    fn a_namespace_declaration_without_prefix_or_uri_refuses_the_file() {
        for rules in [
            schema_with(r#"<ns prefix="" uri="urn:a"/>"#),
            schema_with(r#"<ns prefix="a" uri=""/>"#),
        ] {
            assert!(
                matches!(parse(&rules), Err(SchemaError::UnsupportedAttribute { .. })),
                "must be refused: {rules}"
            );
        }
    }

    #[test]
    fn a_variable_declared_twice_in_one_scope_refuses_the_file() {
        const X: &str = r#"<let name="x" value="1"/>"#;
        for rules in [
            schema_with(&format!("{X}{X}")),
            schema_with(&format!("<pattern>{X}</pattern><pattern>{X}</pattern>")),
            schema_with(&format!("{X}<pattern>{X}</pattern>")),
            rule_with(&format!("{X}{X}")),
        ] {
            assert!(
                matches!(
                    parse(&rules),
                    Err(SchemaError::DuplicateVariable { ref name }) if name == "x"
                ),
                "must be refused: {rules}"
            );
        }
    }

    #[test]
    fn a_rule_variable_named_like_a_global_one_refuses_the_file() {
        const RULE: &str =
            r#"<pattern><rule context="a"><let name="x" value="2"/></rule></pattern>"#;
        for rules in [
            schema_with(&format!(r#"<let name="x" value="1"/>{RULE}"#)),
            schema_with(&format!(
                r#"{RULE}<pattern><let name="x" value="1"/></pattern>"#
            )),
        ] {
            assert!(
                matches!(
                    parse(&rules),
                    Err(SchemaError::DuplicateVariable { ref name }) if name == "x"
                ),
                "must be refused: {rules}"
            );
        }
    }

    #[test]
    fn two_rules_may_each_have_a_variable_of_the_same_name() -> Result<(), SchemaError> {
        let rules = schema_with(
            r#"<pattern>
                 <rule context="a"><let name="x" value="1"/></rule>
                 <rule context="b"><let name="x" value="2"/></rule>
               </pattern>"#,
        );

        assert_eq!(parse(&rules)?.patterns[0].len(), 2);
        Ok(())
    }

    #[test]
    fn what_has_no_effect_on_the_rules_is_accepted() -> Result<(), SchemaError> {
        for rules in [
            schema_with("<title>t</title><p>text</p>"),
            schema_with(r#"<phase id="x"><active pattern="y"/></phase>"#),
            schema_with(r#"<pattern abstract="false"><title>t</title></pattern>"#),
            r##"<schema xmlns="http://purl.oclc.org/dsdl/schematron" defaultPhase="#ALL" queryBinding="xslt3"/>"##
                .to_owned(),
        ] {
            parse(&rules)?;
        }
        Ok(())
    }
}
