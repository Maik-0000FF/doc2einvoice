// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Translation of user-defined XSLT functions into XPath.
//!
//! Rule files define helper functions (check digits, IBAN tests) as XSLT
//! `function` elements. Without an XSLT processor they are translated into
//! XPath inline functions bound to variables. The supported body is a list of
//! `variable` instructions followed by the result, which is given by `sequence`
//! instructions or by a `choose`.

use roxmltree::Node;

use crate::error::SchemaError;
use crate::expression::{function_variable, normalize, variable_references};

const INSTRUCTION_PARAM: &str = "param";
const INSTRUCTION_VARIABLE: &str = "variable";
const INSTRUCTION_SEQUENCE: &str = "sequence";
const INSTRUCTION_VALUE_OF: &str = "value-of";
const INSTRUCTION_CHOOSE: &str = "choose";
const INSTRUCTION_WHEN: &str = "when";
const INSTRUCTION_OTHERWISE: &str = "otherwise";

const ATTRIBUTE_NAME: &str = "name";
const ATTRIBUTE_SELECT: &str = "select";
const ATTRIBUTE_TEST: &str = "test";
const ATTRIBUTE_AS: &str = "as";

const EMPTY_SEQUENCE: &str = "()";
/// Marks a type that also admits the empty sequence, as in `xs:string?`.
const OPTIONAL_TYPE_SUFFIX: char = '?';
/// Parameter through which a recursive function receives itself.
const SELF_PARAMETER: &str = "self";
const RECURSION_VARIABLE: &str = "recursive";

/// A user-defined function as an XPath expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Function {
    /// The variable the function item is bound to.
    pub(crate) variable: String,
    /// An expression that evaluates to the function item.
    pub(crate) expression: String,
}

struct Parameter {
    name: String,
    /// The declared atomic type, without an occurrence indicator.
    declared_type: Option<String>,
}

/// Translates every function, ordered so that each one follows the functions
/// it calls. `call_names` are the names of all functions of the rule file.
pub(crate) fn translate_all(
    nodes: &[Node],
    call_names: &[String],
) -> Result<Vec<Function>, SchemaError> {
    let mut pending = nodes
        .iter()
        .map(|node| translate(node, call_names))
        .collect::<Result<Vec<_>, _>>()?;
    let mut ordered = Vec::with_capacity(pending.len());

    while !pending.is_empty() {
        let independent = pending.iter().position(|candidate| {
            pending
                .iter()
                .all(|other| other.variable == candidate.variable || !calls(candidate, other))
        });
        // Functions that call each other in a cycle cannot be ordered. They are
        // kept in document order; compiling them then fails with the name of
        // the first unresolved call.
        ordered.push(pending.remove(independent.unwrap_or(0)));
    }
    Ok(ordered)
}

/// The definitions an expression needs, as a chain of `let` bindings to put
/// in front of it: the functions it calls and those they call in turn, each
/// after the functions it depends on. Empty if the expression calls none.
///
/// `functions` must be in the order [`translate_all`] returns.
pub(crate) fn prelude_for(functions: &[Function], expression: &str) -> String {
    let mut needed = variable_references(expression);
    // The functions are ordered callees first, so one pass from the last to
    // the first reaches everything a needed function calls.
    for function in functions.iter().rev() {
        if needed.contains(&function.variable) {
            needed.extend(variable_references(&function.expression));
        }
    }
    functions
        .iter()
        .filter(|function| needed.contains(&function.variable))
        .map(|function| binding(&function.variable, &function.expression))
        .collect()
}

/// One `let` binding, to be followed by the expression that reads it.
fn binding(name: &str, value: &str) -> String {
    format!("let ${name} := {value} return ")
}

/// Whether the body of `function` calls `target`.
fn calls(function: &Function, target: &Function) -> bool {
    function.expression.contains(&call_of(&target.variable))
}

fn call_of(variable: &str) -> String {
    format!("${variable}(")
}

fn translate(node: &Node, call_names: &[String]) -> Result<Function, SchemaError> {
    let call_name = node
        .attribute(ATTRIBUTE_NAME)
        .ok_or(SchemaError::MissingAttribute {
            element: "function",
            attribute: ATTRIBUTE_NAME,
        })?;
    let variable = function_variable(call_name);
    let children: Vec<Node> = node.children().filter(Node::is_element).collect();
    let parameters = parameters_of(&children);

    let body = normalize(
        &format!(
            "{}{}",
            conversions(&parameters),
            body_expression(&children, call_name)?
        ),
        call_names,
    );
    let parameter_list = parameters
        .iter()
        .map(|parameter| format!("${}", parameter.name))
        .collect::<Vec<_>>()
        .join(", ");

    let own_call = call_of(&variable);
    let expression = if body.contains(&own_call) {
        // An inline function cannot refer to itself by name, so a recursive
        // function is handed to itself as its first argument.
        let recursive_body =
            body.replace(&own_call, &format!("${SELF_PARAMETER}(${SELF_PARAMETER}, "));
        format!(
            "let ${RECURSION_VARIABLE} := function(${SELF_PARAMETER}, {parameter_list}) {{ {recursive_body} }} \
             return function({parameter_list}) {{ ${RECURSION_VARIABLE}(${RECURSION_VARIABLE}, {parameter_list}) }}"
        )
    } else {
        format!("function({parameter_list}) {{ {body} }}")
    };
    Ok(Function {
        variable,
        expression,
    })
}

fn parameters_of(children: &[Node]) -> Vec<Parameter> {
    children
        .iter()
        .filter(|child| child.tag_name().name() == INSTRUCTION_PARAM)
        .map(|child| Parameter {
            name: child
                .attribute(ATTRIBUTE_NAME)
                .unwrap_or_default()
                .to_owned(),
            declared_type: child
                .attribute(ATTRIBUTE_AS)
                .map(|declared| declared.trim_end_matches(OPTIONAL_TYPE_SUFFIX).to_owned()),
        })
        .collect()
}

/// Declared parameter types are applied as explicit conversions at the start of
/// the body, because the engine does not convert a node argument to a declared
/// atomic type by itself. The constructor functions accept the empty sequence,
/// so optional parameters need no special case.
fn conversions(parameters: &[Parameter]) -> String {
    parameters
        .iter()
        .filter_map(|parameter| {
            parameter
                .declared_type
                .as_ref()
                .map(|declared| (&parameter.name, declared))
        })
        .map(|(name, declared)| binding(name, &format!("{declared}(${name})")))
        .collect()
}

/// Translates a list of instructions into one expression: the variables become
/// nested `let` bindings around the result.
fn body_expression(instructions: &[Node], function: &str) -> Result<String, SchemaError> {
    let mut bindings = String::new();
    let mut results = Vec::new();

    for instruction in instructions {
        match instruction.tag_name().name() {
            INSTRUCTION_PARAM => {}
            INSTRUCTION_VARIABLE => {
                let name = instruction.attribute(ATTRIBUTE_NAME).unwrap_or_default();
                let value = variable_value(instruction, function)?;
                bindings.push_str(&binding(name, &format!("({value})")));
            }
            INSTRUCTION_SEQUENCE | INSTRUCTION_VALUE_OF => {
                let select = instruction
                    .attribute(ATTRIBUTE_SELECT)
                    .unwrap_or(EMPTY_SEQUENCE);
                results.push(format!("({select})"));
            }
            INSTRUCTION_CHOOSE => {
                results.push(format!("({})", choose_expression(instruction, function)?));
            }
            other => {
                return Err(SchemaError::UnsupportedInstruction {
                    function: function.to_owned(),
                    instruction: other.to_owned(),
                });
            }
        }
    }
    Ok(format!("{bindings}({})", results.join(", ")))
}

/// A variable states its value as a `select` expression, as nested
/// instructions, or as plain text.
fn variable_value(variable: &Node, function: &str) -> Result<String, SchemaError> {
    if let Some(select) = variable.attribute(ATTRIBUTE_SELECT) {
        return Ok(select.to_owned());
    }
    let nested: Vec<Node> = variable.children().filter(Node::is_element).collect();
    if nested.is_empty() {
        return Ok(string_literal(variable.text().unwrap_or_default()));
    }
    body_expression(&nested, function)
}

fn choose_expression(choose: &Node, function: &str) -> Result<String, SchemaError> {
    let mut expression = EMPTY_SEQUENCE.to_owned();
    let branches: Vec<Node> = choose.children().filter(Node::is_element).collect();

    // Built from the last branch to the first, so that each `when` wraps the
    // branches that follow it.
    for branch in branches.iter().rev() {
        let instructions: Vec<Node> = branch.children().filter(Node::is_element).collect();
        let body = body_expression(&instructions, function)?;
        expression = match branch.tag_name().name() {
            INSTRUCTION_OTHERWISE => body,
            INSTRUCTION_WHEN => {
                let test =
                    branch
                        .attribute(ATTRIBUTE_TEST)
                        .ok_or(SchemaError::MissingAttribute {
                            element: INSTRUCTION_WHEN,
                            attribute: ATTRIBUTE_TEST,
                        })?;
                format!("if ({test}) then ({body}) else ({expression})")
            }
            other => {
                return Err(SchemaError::UnsupportedInstruction {
                    function: function.to_owned(),
                    instruction: other.to_owned(),
                });
            }
        };
    }
    Ok(expression)
}

/// An XPath string literal; a quote inside it is written twice.
fn string_literal(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::{prelude_for, translate_all};
    use crate::error::SchemaError;

    const XSLT: &str = "http://www.w3.org/1999/XSL/Transform";

    fn translated(functions: &str, names: &[&str]) -> Result<Vec<super::Function>, SchemaError> {
        let document = format!("<root xmlns=\"{XSLT}\">{functions}</root>");
        let parsed = roxmltree::Document::parse(&document)?;
        let nodes: Vec<_> = parsed
            .root_element()
            .children()
            .filter(roxmltree::Node::is_element)
            .collect();
        let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        translate_all(&nodes, &names)
    }

    #[test]
    fn variables_become_let_bindings_around_the_result() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:f"><param name="v"/><variable name="n" select="$v + 1"/><sequence select="$n * 2"/></function>"#,
            &["u:f"],
        )?;

        assert_eq!(functions[0].variable, "fn_u_f");
        assert_eq!(
            functions[0].expression,
            "function($v) { let $n := ($v + 1) return (($n * 2)) }"
        );
        Ok(())
    }

    #[test]
    fn a_declared_parameter_type_becomes_a_conversion() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:f"><param name="v" as="xs:string?"/><sequence select="$v"/></function>"#,
            &["u:f"],
        )?;

        assert!(
            functions[0]
                .expression
                .contains("let $v := xs:string($v) return ")
        );
        Ok(())
    }

    #[test]
    fn a_text_variable_becomes_a_string_literal() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:f"><variable name="allowed">AB"C</variable><sequence select="$allowed"/></function>"#,
            &["u:f"],
        )?;

        assert!(
            functions[0]
                .expression
                .contains(r#"let $allowed := ("AB""C") return "#)
        );
        Ok(())
    }

    #[test]
    fn choose_becomes_a_conditional() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:f"><param name="v"/><choose><when test="$v"><sequence select="1"/></when><otherwise><sequence select="2"/></otherwise></choose></function>"#,
            &["u:f"],
        )?;

        assert!(
            functions[0]
                .expression
                .contains("if ($v) then (((1))) else (((2)))")
        );
        Ok(())
    }

    #[test]
    fn a_recursive_function_receives_itself() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:f"><param name="v"/><sequence select="if ($v) then u:f($v - 1) else 0"/></function>"#,
            &["u:f"],
        )?;

        assert!(functions[0].expression.contains("$self($self, $v - 1)"));
        assert!(
            functions[0]
                .expression
                .contains("$recursive($recursive, $v)")
        );
        Ok(())
    }

    #[test]
    fn a_function_follows_the_function_it_calls() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:outer"><param name="v"/><sequence select="u:inner($v)"/></function><function name="u:inner"><param name="v"/><sequence select="$v"/></function>"#,
            &["u:outer", "u:inner"],
        )?;

        assert_eq!(functions[0].variable, "fn_u_inner");
        assert_eq!(functions[1].variable, "fn_u_outer");
        assert!(prelude_for(&functions, "$fn_u_outer(1)").starts_with("let $fn_u_inner := "));
        Ok(())
    }

    #[test]
    fn only_the_functions_an_expression_needs_are_defined() -> Result<(), SchemaError> {
        let functions = translated(
            r#"<function name="u:a"><sequence select="1"/></function><function name="u:b"><sequence select="u:a()"/></function><function name="u:c"><sequence select="3"/></function>"#,
            &["u:a", "u:b", "u:c"],
        )?;

        assert_eq!(prelude_for(&functions, "count(x) = 1"), "");
        assert!(!prelude_for(&functions, "$fn_u_b()").contains("fn_u_c"));
        assert!(prelude_for(&functions, "$fn_u_b()").contains("let $fn_u_a := "));
        assert!(!prelude_for(&functions, "'$fn_u_c'").contains("fn_u_c"));
        Ok(())
    }

    #[test]
    fn an_unknown_instruction_is_refused() {
        let result = translated(
            r#"<function name="u:f"><for-each select="1"/></function>"#,
            &["u:f"],
        );

        assert!(matches!(
            result,
            Err(SchemaError::UnsupportedInstruction { instruction, .. }) if instruction == "for-each"
        ));
    }
}
