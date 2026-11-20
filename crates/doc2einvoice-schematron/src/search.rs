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

use std::collections::BTreeSet;

use xee_xpath_ast::ast;

/// Collects the predicates found in one expression.
pub(crate) struct Searches<'a> {
    /// The expression as it was parsed; the predicates are cut out of it.
    source: &'a str,
    found: &'a mut BTreeSet<String>,
}

impl<'a> Searches<'a> {
    pub(crate) fn new(source: &'a str, found: &'a mut BTreeSet<String>) -> Self {
        Self { source, found }
    }

    /// Adds every predicate that stands on a child step after a search.
    pub(crate) fn collect(&mut self, xpath: &ast::XPath) {
        self.expr(&xpath.0.value);
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
            if let Some(text) = self.source.get(span.start..span.end) {
                let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
                self.found.insert(text);
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

/// Whether a step is the search through all descendants `//` stands for.
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
