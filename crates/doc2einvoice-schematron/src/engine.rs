// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Compiling a rule file and evaluating it on documents.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use xee_xpath::context::{StaticContextBuilder, Variables};
use xee_xpath::query::SequenceQuery;
use xee_xpath::{Documents, Queries, Query, Sequence};
use xot::Node;
use xot::xmlname::OwnedName;

use crate::error::{DocumentError, SchemaError};
use crate::expression::variable_references;
use crate::functions::{self, Function};
use crate::include::{self, IncludeResolver};
use crate::location::{name_of, path_of};
use crate::report::{Finding, Outcome, Report};
use crate::schema::{self, MessagePart, ParsedAssertion, ParsedRule, Variable};
use crate::search::Searches;

/// Location reported for a finding that concerns no particular node.
const DOCUMENT_LOCATION: &str = "/";
/// Why the predicates after a search cannot be reported for an expression.
const PREDICATE_NOT_IN_SOURCE: &str = "a predicate lies outside the text of the expression";
/// Separates the items of a `value-of` in the text of an assertion.
const VALUE_SEPARATOR: &str = " ";

/// A rule file, compiled and ready to validate documents.
///
/// Compiling is the expensive step; a schema is meant to be compiled once and
/// used for many documents. A schema is bound to the thread it was compiled on.
pub struct Schema {
    namespaces: Vec<(String, String)>,
    global_variables: Vec<CompiledVariable>,
    patterns: Vec<Vec<CompiledRule>>,
}

/// A compiled expression together with the variables it reads.
struct Expression {
    query: SequenceQuery,
    /// The variables declared outside the expression that it reads, in the
    /// order they are declared.
    reads: Vec<String>,
    /// The expression as it was compiled, with the functions it calls put in
    /// front of it.
    source: String,
    /// The variables the expression was compiled to see.
    visible: Vec<String>,
}

struct CompiledVariable {
    name: String,
    value: Expression,
}

struct CompiledRule {
    selection: Expression,
    variables: Vec<CompiledVariable>,
    assertions: Vec<CompiledAssertion>,
}

struct CompiledAssertion {
    id: String,
    flag: Option<String>,
    message: Vec<CompiledMessagePart>,
    test: Expression,
}

enum CompiledMessagePart {
    Text(String),
    ContextName,
    ValueOf(Expression),
}

impl Schema {
    /// Compiles a rule file.
    ///
    /// `includes` supplies the text of every file the rule file includes.
    ///
    /// # Errors
    ///
    /// Returns a [`SchemaError`] if the rule file is not a usable Schematron
    /// schema: it is malformed, includes an unknown file, uses a construct
    /// outside the supported subset, or contains an expression the XPath
    /// engine does not accept. A schema is never compiled partially.
    pub fn compile(source: &str, includes: &dyn IncludeResolver) -> Result<Self, SchemaError> {
        let expanded = include::expand(source, includes)?;
        let parsed = schema::parse(&expanded)?;
        let compiler = Compiler {
            namespaces: &parsed.namespaces,
            functions: &parsed.functions,
            queries: Queries::default(),
        };

        let global_names: Vec<String> = parsed
            .global_variables
            .iter()
            .map(|variable| variable.name.clone())
            .collect();
        compiler.check_functions(&global_names)?;

        let global_variables = compiler.compile_variables(&parsed.global_variables, &[])?;
        let patterns = parsed
            .patterns
            .iter()
            .map(|rules| {
                rules
                    .iter()
                    .map(|rule| compiler.compile_rule(rule, &global_names))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            namespaces: parsed.namespaces.clone(),
            global_variables,
            patterns,
        })
    }

    /// The predicates the rule file puts on a step that follows a search
    /// through all descendants, as in `//name[predicate]`: each as it is
    /// written, with white space collapsed.
    ///
    /// Every expression of the rule file is looked into: rule contexts,
    /// variables, tests, the values an assertion text shows, and the bodies
    /// of the functions they call. A rule context that is not anchored at the
    /// root is such a search itself.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaError::Expression`] if an expression cannot be parsed
    /// again, or if a predicate cannot be cut out of the expression it stands
    /// in. Neither happens for a schema that compiled; what is returned is
    /// therefore complete, or it is an error.
    pub fn search_predicates(&self) -> Result<BTreeSet<String>, SchemaError> {
        let mut found = BTreeSet::new();
        for expression in self.expressions() {
            let parsed = static_context(&self.namespaces, &expression.visible)
                .parse_xpath(&expression.source)
                .map_err(|error| SchemaError::Expression {
                    subject: expression.source.clone(),
                    reason: format!("{error:?}"),
                })?;
            Searches::new(&expression.source, &mut found)
                .collect(&parsed)
                .map_err(|_| SchemaError::Expression {
                    subject: expression.source.clone(),
                    reason: PREDICATE_NOT_IN_SOURCE.to_owned(),
                })?;
        }
        Ok(found)
    }

    /// Every compiled expression of the rule file.
    fn expressions(&self) -> impl Iterator<Item = &Expression> {
        let globals = self.global_variables.iter().map(|variable| &variable.value);
        let rules = self.patterns.iter().flatten().flat_map(|rule| {
            let variables = rule.variables.iter().map(|variable| &variable.value);
            let assertions = rule.assertions.iter().flat_map(|assertion| {
                let shown = assertion.message.iter().filter_map(|part| match part {
                    CompiledMessagePart::ValueOf(expression) => Some(expression),
                    CompiledMessagePart::Text(_) | CompiledMessagePart::ContextName => None,
                });
                std::iter::once(&assertion.test).chain(shown)
            });
            std::iter::once(&rule.selection)
                .chain(variables)
                .chain(assertions)
        });
        globals.chain(rules)
    }

    /// The identifiers of all assertions of the rule file, in document order.
    pub fn assertion_ids(&self) -> impl Iterator<Item = &str> {
        self.patterns
            .iter()
            .flatten()
            .flat_map(|rule| &rule.assertions)
            .map(|assertion| assertion.id.as_str())
    }

    /// Validates a document against the rule file.
    ///
    /// # Errors
    ///
    /// Returns a [`DocumentError`] if the document is not well-formed XML or
    /// the XPath engine fails in a way it cannot report. Assertions the
    /// document does not satisfy are not errors; they are the findings of the
    /// returned [`Report`].
    pub fn validate(&self, document: &str) -> Result<Report, DocumentError> {
        // The engine is known to panic on inputs it does not handle. A panic
        // must end this validation, not the application.
        catch_unwind(AssertUnwindSafe(|| self.evaluate(document)))
            .unwrap_or(Err(DocumentError::EngineFailure))
    }

    fn evaluate(&self, document: &str) -> Result<Report, DocumentError> {
        let mut documents = Documents::new();
        let handle = documents
            .add_string_without_uri(document)
            .map_err(|error| DocumentError::Xml(error.to_string()))?;
        let root = documents
            .document_node(handle)
            .ok_or(DocumentError::EngineFailure)?;

        let mut globals = Scope::default();
        globals.bind(&self.global_variables, &mut documents, root);

        let mut findings = Vec::new();
        for rules in &self.patterns {
            // Within one pattern a node is checked by the first rule whose
            // context matches it, and by no other.
            let mut claimed = HashSet::new();
            for rule in rules {
                match globals.evaluate(&rule.selection, &mut documents, root) {
                    Ok(selected) => {
                        for node in selected.nodes().filter_map(Result::ok) {
                            if claimed.insert(node) {
                                rule.check(node, &mut documents, &globals, &mut findings);
                            }
                        }
                    }
                    Err(error) => rule.report_unusable(&error, &mut findings),
                }
            }
        }
        Ok(Report::new(findings))
    }
}

impl CompiledRule {
    /// Checks the assertions of the rule on one node it matches.
    fn check(
        &self,
        node: Node,
        documents: &mut Documents,
        globals: &Scope,
        findings: &mut Vec<Finding>,
    ) {
        let mut scope = globals.clone();
        scope.bind(&self.variables, documents, node);

        for assertion in &self.assertions {
            let outcome = match scope
                .evaluate(&assertion.test, documents, node)
                .and_then(|value| {
                    value
                        .effective_boolean_value()
                        .map_err(|error| error_code(&error))
                }) {
                Ok(true) => continue,
                Ok(false) => Outcome::Violated,
                Err(error) => Outcome::EvaluationError { error },
            };
            findings.push(Finding {
                assertion_id: assertion.id.clone(),
                flag: assertion.flag.clone(),
                message: assertion.message_on(node, documents, &scope),
                location: path_of(documents.xot(), node),
                outcome,
            });
        }
    }

    /// Reports every assertion of a rule whose context cannot be evaluated:
    /// which nodes the rule applies to is unknown, so none of its assertions
    /// can be shown to hold.
    fn report_unusable(&self, error: &str, findings: &mut Vec<Finding>) {
        findings.extend(self.assertions.iter().map(|assertion| Finding {
            assertion_id: assertion.id.clone(),
            flag: assertion.flag.clone(),
            message: assertion.message_without_node(),
            location: DOCUMENT_LOCATION.to_owned(),
            outcome: Outcome::EvaluationError {
                error: error.to_owned(),
            },
        }));
    }
}

impl CompiledAssertion {
    /// The text of the assertion for a finding on `node`.
    ///
    /// A `value-of` that cannot be evaluated is left empty: the text explains
    /// a finding, it does not decide one.
    fn message_on(&self, node: Node, documents: &mut Documents, scope: &Scope) -> String {
        let mut text = String::new();
        for part in &self.message {
            match part {
                CompiledMessagePart::Text(written) => text.push_str(written),
                CompiledMessagePart::ContextName => text.push_str(&name_of(documents.xot(), node)),
                CompiledMessagePart::ValueOf(expression) => {
                    if let Ok(value) = scope.evaluate(expression, documents, node) {
                        let items: Vec<String> = value
                            .iter()
                            .filter_map(|item| item.string_value(documents.xot()).ok())
                            .collect();
                        text.push_str(&items.join(VALUE_SEPARATOR));
                    }
                }
            }
        }
        collapse_whitespace(&text)
    }

    /// The text of the assertion where no node is known: only what is written.
    fn message_without_node(&self) -> String {
        let written: String = self
            .message
            .iter()
            .filter_map(|part| match part {
                CompiledMessagePart::Text(written) => Some(written.as_str()),
                CompiledMessagePart::ContextName | CompiledMessagePart::ValueOf(_) => None,
            })
            .collect();
        collapse_whitespace(&written)
    }
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The variables visible at one point of the evaluation.
///
/// How a variable that cannot be evaluated is handled. An expression is never
/// evaluated with a value that was not computed, so a failure can never turn
/// into a passed assertion:
///
/// | situation                                             | outcome                                   |
/// |-------------------------------------------------------|-------------------------------------------|
/// | a variable evaluates                                  | bound to its value                        |
/// | a variable raises an error                            | failed, with that error                   |
/// | a variable reads a failed variable                    | failed with the same error, not evaluated |
/// | a chain `a`, `b` reads `a`, `c` reads `b`             | `b` and `c` fail like `a`                 |
/// | a rule variable reads a failed global variable        | failed like it                            |
/// | an assertion reads a failed variable                  | reported with that error                  |
/// | an assertion reads two failed variables               | the error of the one declared first       |
/// | an assertion reads `$totalAmount`, `$total` failed    | evaluated: it does not read `$total`      |
/// | an assertion binds a name of its own (`for $x in`)    | evaluated: it does not read the outer `x` |
/// | an assertion calls a function that reads a failed one | reported with that error                  |
/// | a rule context reads a failed variable                | every assertion of the rule is reported   |
/// | a rule context raises an error itself                 | every assertion of the rule is reported   |
/// | an assertion reads no failed variable                 | evaluated                                 |
///
/// Every name means one variable: a rule file in which a rule declares a
/// variable under the name of a global one is refused when it is read, so no
/// case of one variable hiding another arises here.
///
/// This is stricter than a processor that evaluates variables on demand: there
/// an assertion that names a failed variable but never reaches it, for example
/// behind `or`, may pass. Which variables an expression reads is decided when
/// the rule file is compiled, so such an assertion is reported here.
#[derive(Clone, Default)]
struct Scope {
    values: Variables,
    /// The variables that could not be evaluated, with the error of each.
    failed: HashMap<String, String>,
}

impl Scope {
    /// Evaluates variables in the order they are declared and adds them.
    fn bind(
        &mut self,
        declarations: &[CompiledVariable],
        documents: &mut Documents,
        context: Node,
    ) {
        for declaration in declarations {
            let value = match self.evaluate(&declaration.value, documents, context) {
                Ok(value) => value,
                Err(error) => {
                    self.failed.insert(declaration.name.clone(), error);
                    // The engine requires a value for every declared variable.
                    // This one is never read: every expression that reads the
                    // variable is refused before it is evaluated.
                    Sequence::default()
                }
            };
            self.values
                .insert(OwnedName::name(&declaration.name), value);
        }
    }

    /// Evaluates an expression on a node. The error is the code the engine
    /// reports, or the error of the first failed variable the expression reads.
    fn evaluate(
        &self,
        expression: &Expression,
        documents: &mut Documents,
        context: Node,
    ) -> Result<Sequence, String> {
        if let Some(error) = expression
            .reads
            .iter()
            .find_map(|name| self.failed.get(name))
        {
            return Err(error.clone());
        }
        let mut builder = expression.query.program().dynamic_context_builder();
        builder.context_node(context);
        builder.documents(documents.documents().clone());
        builder.variables(self.values.clone());
        let dynamic_context = builder.build();
        expression
            .query
            .execute_with_context(documents, &dynamic_context)
            .map_err(|error| error_code(&error))
    }
}

/// The error code of an engine error, such as `FORG0001`, without the position
/// the engine appends to it.
fn error_code(error: &impl std::fmt::Display) -> String {
    let text = error.to_string();
    text.split_whitespace().next().unwrap_or(&text).to_owned()
}

/// What an expression is parsed and compiled against: the namespaces of the
/// rule file and the variables the expression may read.
fn static_context(
    namespaces: &[(String, String)],
    visible: &[String],
) -> xee_xpath::context::StaticContext {
    let mut builder = StaticContextBuilder::default();
    builder.namespaces(
        namespaces
            .iter()
            .map(|(prefix, uri)| (prefix.as_str(), uri.as_str())),
    );
    builder.variable_names(visible.iter().map(|name| OwnedName::name(name)));
    builder.build()
}

struct Compiler<'a> {
    namespaces: &'a [(String, String)],
    functions: &'a [Function],
    queries: Queries<'a>,
}

impl Compiler<'_> {
    /// Compiles every function definition once on its own, so that a faulty
    /// function is reported as such and not as a fault of the first assertion
    /// that calls it. A function may read any global variable.
    fn check_functions(&self, global_names: &[String]) -> Result<(), SchemaError> {
        for function in self.functions {
            self.compile(
                &format!("${}", function.variable),
                global_names,
                &format!("function `{}`", function.variable),
            )?;
        }
        Ok(())
    }

    /// Compiles an expression that may read the variables in `visible`, and
    /// determines which of them it reads.
    fn compile(
        &self,
        expression: &str,
        visible: &[String],
        subject: &str,
    ) -> Result<Expression, SchemaError> {
        // Function items cannot be passed from one compiled expression into
        // another, so the definitions an expression needs are put in front of
        // it.
        let prelude = functions::prelude_for(self.functions, expression);
        let complete = if prelude.is_empty() {
            expression.to_owned()
        } else {
            format!("{prelude}({expression})")
        };
        let query = self
            .query(&complete, visible)
            .map_err(|reason| SchemaError::Expression {
                subject: subject.to_owned(),
                reason,
            })?;

        // A visible variable is read if its name appears as a reference and
        // the expression does not compile without it. The second test rules
        // out a name the expression binds itself, as in `for $name in`.
        let referenced = variable_references(&complete);
        let reads = visible
            .iter()
            .filter(|name| referenced.contains(*name))
            .filter(|name| {
                let without: Vec<String> = visible
                    .iter()
                    .filter(|other| other != name)
                    .cloned()
                    .collect();
                self.query(&complete, &without).is_err()
            })
            .cloned()
            .collect();
        Ok(Expression {
            query,
            reads,
            source: complete,
            visible: visible.to_vec(),
        })
    }

    /// Compiles an expression that declares exactly the variables in `visible`.
    /// The engine requires a value for every variable an expression declares,
    /// so no expression declares more than exists where it is evaluated.
    fn query(&self, expression: &str, visible: &[String]) -> Result<SequenceQuery, String> {
        self.queries
            .sequence_with_context(expression, static_context(self.namespaces, visible))
            .map_err(|error| error.to_string())
    }

    /// Compiles variables that are declared one after another: each may read
    /// the variables in `outer` and those declared before it.
    fn compile_variables(
        &self,
        declarations: &[Variable],
        outer: &[String],
    ) -> Result<Vec<CompiledVariable>, SchemaError> {
        let mut visible = outer.to_vec();
        let mut compiled = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            compiled.push(CompiledVariable {
                name: declaration.name.clone(),
                value: self.compile(
                    &declaration.expression,
                    &visible,
                    &format!("variable `{}`", declaration.name),
                )?,
            });
            visible.push(declaration.name.clone());
        }
        Ok(compiled)
    }

    fn compile_rule(
        &self,
        rule: &ParsedRule,
        global_names: &[String],
    ) -> Result<CompiledRule, SchemaError> {
        let variables = self.compile_variables(&rule.variables, global_names)?;
        let visible: Vec<String> = global_names
            .iter()
            .cloned()
            .chain(rule.variables.iter().map(|variable| variable.name.clone()))
            .collect();

        Ok(CompiledRule {
            selection: self.compile(
                &rule.selection,
                global_names,
                &format!("rule context `{}`", rule.context),
            )?,
            variables,
            assertions: rule
                .assertions
                .iter()
                .map(|assertion| self.compile_assertion(assertion, &visible))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn compile_assertion(
        &self,
        assertion: &ParsedAssertion,
        visible: &[String],
    ) -> Result<CompiledAssertion, SchemaError> {
        let subject = format!("assertion `{}`", assertion.id);
        let message = assertion
            .message
            .iter()
            .map(|part| {
                Ok(match part {
                    MessagePart::Text(written) => CompiledMessagePart::Text(written.clone()),
                    MessagePart::ContextName => CompiledMessagePart::ContextName,
                    MessagePart::ValueOf(expression) => {
                        CompiledMessagePart::ValueOf(self.compile(expression, visible, &subject)?)
                    }
                })
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;

        Ok(CompiledAssertion {
            id: assertion.id.clone(),
            flag: assertion.flag.clone(),
            message,
            test: self.compile(&assertion.test, visible, &subject)?,
        })
    }
}
