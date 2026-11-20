// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Finding the predicates an expression puts on a step that follows a search
//! through all descendants (`//`).
//!
//! The XPath engine evaluates such a search as one descent where no predicate
//! of the step can depend on a position. Which predicates the rule files use
//! there is therefore worth knowing exactly, and worth noticing when it
//! changes. This module reads them from the parsed expression, never from its
//! text: only the parser knows where a predicate begins and ends.
//!
//! Every part of an expression is looked into, also the bodies of functions,
//! the branches of a condition and the predicates of other predicates. The
//! matches below name every kind of expression the parser knows, so that a new
//! kind cannot be skipped without the compiler saying so.
//!
//! What counts as a search, and that only a child step after it matters, is
//! decided a second time in the XPath engine: in `src/descent.rs` of the
//! patched copy of its compiler under `vendor/`, which merges exactly those
//! steps. The two have to agree. If the engine comes to recognise a search
//! differently, this module follows.

use std::collections::BTreeSet;

use xee_xpath_ast::ast;

/// Collects the predicates found in one expression.
pub(crate) struct Searches<'a> {
    /// The expression as it was parsed; the predicates are cut out of it.
    source: &'a str,
    found: &'a mut BTreeSet<String>,
    /// Whether a predicate could not be cut out of the expression.
    incomplete: bool,
}

/// A predicate lies where the expression has no text: what was collected is
/// not everything the expression holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Incomplete;

impl<'a> Searches<'a> {
    pub(crate) fn new(source: &'a str, found: &'a mut BTreeSet<String>) -> Self {
        Self {
            source,
            found,
            incomplete: false,
        }
    }

    /// Adds every predicate that stands on a child step after a search.
    ///
    /// | Predicate after a search                          | Result            |
    /// |---------------------------------------------------|-------------------|
    /// | its place lies within the expression              | added as written  |
    /// | its place lies outside the expression, or cuts a  | [`Incomplete`]:   |
    /// | character in two                                  | never left out    |
    /// |                                                   | silently          |
    ///
    /// # Errors
    ///
    /// Returns [`Incomplete`] as the table states. The predicates that could
    /// be cut out have been added all the same.
    pub(crate) fn collect(&mut self, xpath: &ast::XPath) -> Result<(), Incomplete> {
        self.expr(&xpath.0.value);
        if self.incomplete {
            Err(Incomplete)
        } else {
            Ok(())
        }
    }

    fn expr(&mut self, expression: &ast::Expr) {
        for single in &expression.0 {
            self.single(&single.value);
        }
    }

    fn optional(&mut self, expression: &ast::ExprOrEmptyS) {
        if let Some(expression) = &expression.value {
            self.expr(expression);
        }
    }

    fn single(&mut self, expression: &ast::ExprSingle) {
        match expression {
            ast::ExprSingle::Path(path) => self.path(path),
            ast::ExprSingle::Apply(apply) => {
                self.path(&apply.path_expr);
                match &apply.operator {
                    ast::ApplyOperator::SimpleMap(paths) => {
                        for path in paths {
                            self.path(path);
                        }
                    }
                    ast::ApplyOperator::Unary(_)
                    | ast::ApplyOperator::Cast(_)
                    | ast::ApplyOperator::Castable(_)
                    | ast::ApplyOperator::Treat(_)
                    | ast::ApplyOperator::InstanceOf(_) => {}
                }
            }
            ast::ExprSingle::Let(binding) => {
                self.single(&binding.var_expr.value);
                self.single(&binding.return_expr.value);
            }
            ast::ExprSingle::If(condition) => {
                self.expr(&condition.condition.value);
                self.single(&condition.then.value);
                self.single(&condition.else_.value);
            }
            ast::ExprSingle::Binary(binary) => {
                self.path(&binary.left);
                self.path(&binary.right);
            }
            ast::ExprSingle::For(iteration) => {
                self.single(&iteration.var_expr.value);
                self.single(&iteration.return_expr.value);
            }
            ast::ExprSingle::Quantified(quantified) => {
                self.single(&quantified.var_expr.value);
                self.single(&quantified.satisfies_expr.value);
            }
        }
    }

    fn path(&mut self, path: &ast::PathExpr) {
        let mut after_search = false;
        for step in &path.steps {
            if after_search {
                self.note(&step.value);
            }
            after_search = is_search(&step.value);
            self.step(&step.value);
        }
    }

    /// Notes the predicates of a step that follows a search, if it is a child
    /// step: only such a step can be merged with the search.
    fn note(&mut self, step: &ast::StepExpr) {
        let ast::StepExpr::AxisStep(step) = step else {
            return;
        };
        if step.axis != ast::Axis::Child {
            return;
        }
        for predicate in &step.predicates {
            let span = predicate.span;
            match self.source.get(span.start..span.end) {
                Some(text) => {
                    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    self.found.insert(text);
                }
                None => self.incomplete = true,
            }
        }
    }

    fn step(&mut self, step: &ast::StepExpr) {
        match step {
            ast::StepExpr::AxisStep(step) => {
                for predicate in &step.predicates {
                    self.expr(&predicate.value);
                }
            }
            ast::StepExpr::PrimaryExpr(primary) => self.primary(&primary.value),
            ast::StepExpr::PostfixExpr { primary, postfixes } => {
                self.primary(&primary.value);
                for postfix in postfixes {
                    match postfix {
                        ast::Postfix::Predicate(predicate) => self.expr(&predicate.value),
                        ast::Postfix::ArgumentList(arguments) => {
                            for argument in arguments {
                                self.single(&argument.value);
                            }
                        }
                        ast::Postfix::Lookup(key) => self.key(key),
                    }
                }
            }
        }
    }

    fn primary(&mut self, primary: &ast::PrimaryExpr) {
        match primary {
            ast::PrimaryExpr::Literal(_)
            | ast::PrimaryExpr::VarRef(_)
            | ast::PrimaryExpr::ContextItem
            | ast::PrimaryExpr::NamedFunctionRef(_) => {}
            ast::PrimaryExpr::Expr(inner) => self.optional(inner),
            ast::PrimaryExpr::FunctionCall(call) => {
                for argument in &call.arguments {
                    self.single(&argument.value);
                }
            }
            ast::PrimaryExpr::InlineFunction(function) => self.optional(&function.body),
            ast::PrimaryExpr::MapConstructor(map) => {
                for entry in &map.entries {
                    self.single(&entry.key.value);
                    self.single(&entry.value.value);
                }
            }
            ast::PrimaryExpr::ArrayConstructor(ast::ArrayConstructor::Square(members)) => {
                self.expr(&members.value);
            }
            ast::PrimaryExpr::ArrayConstructor(ast::ArrayConstructor::Curly(members)) => {
                self.optional(members);
            }
            ast::PrimaryExpr::UnaryLookup(key) => self.key(key),
        }
    }

    fn key(&mut self, key: &ast::KeySpecifier) {
        match key {
            ast::KeySpecifier::Expr(expression) => self.optional(expression),
            ast::KeySpecifier::NcName(_)
            | ast::KeySpecifier::Integer(_)
            | ast::KeySpecifier::Star => {}
        }
    }
}

/// Whether a step is the search through all descendants `//` stands for. The
/// XPath engine asks the same of a step before it merges it (see the module
/// documentation).
fn is_search(step: &ast::StepExpr) -> bool {
    match step {
        ast::StepExpr::AxisStep(step) => {
            step.axis == ast::Axis::DescendantOrSelf
                && step.node_test == ast::NodeTest::KindTest(ast::KindTest::Any)
                && step.predicates.is_empty()
        }
        ast::StepExpr::PrimaryExpr(_) | ast::StepExpr::PostfixExpr { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{Incomplete, Searches};
    use std::collections::BTreeSet;
    use xee_xpath_ast::{Namespaces, VariableNames, ast};

    const EXPRESSION: &str = "//a[b]";

    fn parsed() -> Result<ast::XPath, String> {
        ast::XPath::parse(
            EXPRESSION,
            &Namespaces::default(),
            &VariableNames::default(),
        )
        .map_err(|error| format!("{error:?}"))
    }

    #[test]
    fn a_predicate_within_the_expression_is_collected() -> Result<(), String> {
        let mut found = BTreeSet::new();

        assert_eq!(
            Searches::new(EXPRESSION, &mut found).collect(&parsed()?),
            Ok(())
        );
        assert_eq!(found.into_iter().collect::<Vec<_>>(), ["b"]);
        Ok(())
    }

    #[test]
    fn a_predicate_outside_the_expression_is_an_error() -> Result<(), String> {
        // The tree is that of the whole expression, the text ends before the
        // predicate: there is nothing to cut it out of.
        let shorter = "//a";
        let mut found = BTreeSet::new();

        assert_eq!(
            Searches::new(shorter, &mut found).collect(&parsed()?),
            Err(Incomplete)
        );
        Ok(())
    }
}
