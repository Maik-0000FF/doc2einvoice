//! Evaluating a search through all descendants as one descent.
//!
//! `//name` is short for `/descendant-or-self::node()/child::name`. Taken
//! literally it visits every node below the context and asks each for its
//! children of that name: one step per node. `descendant::name` selects the
//! same nodes in one walk.
//!
//! The two differ only in what a predicate sees as its position: in
//! `//name[1]` the position counts among the children of one parent, in
//! `descendant::name[1]` among all descendants. The steps are therefore
//! merged only where no predicate can depend on a position: its value is
//! never a number, and nothing in it asks for a position or a size.
//!
//! Whether that holds is decided on the shape of the predicate alone. Every
//! shape that is not known to be safe leaves the steps as they are written:
//!
//! | Step after `//`                                   | Merged |
//! |---------------------------------------------------|--------|
//! | `name`, `*`, `text()`: a child step, no predicate | yes    |
//! | `name[a = 'x']`, `name[a and b]`: a comparison    | yes    |
//! | `name[a/b]`: a path that ends in a step           | yes    |
//! | `name[not(a)]`, `name[exists(a)]`                 | yes    |
//! | `name[a[1] = 'x']`: a position inside, with its   | yes    |
//! | own focus                                         |        |
//! | `name[1]`, `name[$n]`, `name[count(a)]`: possibly | no     |
//! | a number                                          |        |
//! | `name[position() = 2]`, `name[last()]`, also      | no     |
//! | further inside                                    |        |
//! | `name[f(a) = 'x']` with a function that is not     | no     |
//! | listed below as free of positions, such as        |        |
//! | `function-lookup`, which can hand out `position`  |        |
//! | `name[if (a) then b else c]`, `name[some $x in`:  | no     |
//! | a shape that is not looked into                   |        |
//! | `@name`, `..`: not a child step                   | no     |
//! | `descendant-or-self::node()[p]/name`: written out | no     |
//! | with a predicate of its own                       |        |

use xee_xpath_ast::{ast, span::Spanned, FN_NAMESPACE, XS_NAMESPACE};
use xot::xmlname::NameStrInfo;

/// Functions of the standard library whose result depends only on their
/// arguments and, for some, on the context item: never on a position or a
/// size, and none of them hands out a function that does. A function that is
/// not listed counts as depending on a position. The functions that construct
/// a value of an XML Schema type are free as well; they are known by their
/// namespace.
const POSITION_FREE_FUNCTIONS: [&str; 44] = [
    "abs",
    "avg",
    "boolean",
    "ceiling",
    "codepoints-to-string",
    "concat",
    "contains",
    "count",
    "data",
    "distinct-values",
    "empty",
    "ends-with",
    "exists",
    "false",
    "floor",
    "local-name",
    "lower-case",
    "matches",
    "max",
    "min",
    "name",
    "namespace-uri",
    "normalize-space",
    "not",
    "number",
    "replace",
    "reverse",
    "root",
    "round",
    "starts-with",
    "string",
    "string-join",
    "string-length",
    "string-to-codepoints",
    "substring",
    "substring-after",
    "substring-before",
    "sum",
    "tokenize",
    "translate",
    "true",
    "upper-case",
    "year-from-date",
    "zero-or-one",
];

/// Functions of the standard library that always return a boolean. Each is
/// also free of positions.
const TRUTH_FUNCTIONS: [&str; 10] = [
    "not",
    "exists",
    "empty",
    "boolean",
    "true",
    "false",
    "contains",
    "starts-with",
    "ends-with",
    "matches",
];

/// Recognises the predicates that cannot depend on a position.
#[derive(Debug)]
pub(crate) struct Descent {
    position_free_functions: Vec<ast::Name>,
    truth_functions: Vec<ast::Name>,
}

/// The names of functions of the standard library.
fn standard_functions(names: &[&str]) -> Vec<ast::Name> {
    names
        .iter()
        .map(|name| ast::Name::new(name.to_string(), FN_NAMESPACE.to_string(), String::new()))
        .collect()
}

impl Descent {
    pub(crate) fn new() -> Self {
        Self {
            position_free_functions: standard_functions(&POSITION_FREE_FUNCTIONS),
            truth_functions: standard_functions(&TRUTH_FUNCTIONS),
        }
    }

    /// The steps of a path, with every search through all descendants that
    /// can be one descent turned into one.
    pub(crate) fn merged(&self, steps: &[ast::StepExprS]) -> Vec<ast::StepExprS> {
        let mut merged = Vec::with_capacity(steps.len());
        let mut index = 0;
        while index < steps.len() {
            let descent = match (steps.get(index), steps.get(index + 1)) {
                (Some(search), Some(step)) if is_search(&search.value) => self.descent(step),
                _ => None,
            };
            match descent {
                Some(descent) => {
                    merged.push(descent);
                    index += 2;
                }
                None => {
                    merged.push(steps[index].clone());
                    index += 1;
                }
            }
        }
        merged
    }

    /// The descent that selects what `step` selects after a search through
    /// all descendants, if there is one.
    fn descent(&self, step: &ast::StepExprS) -> Option<ast::StepExprS> {
        let ast::StepExpr::AxisStep(child) = &step.value else {
            return None;
        };
        if child.axis != ast::Axis::Child {
            return None;
        }
        if !child
            .predicates
            .iter()
            .all(|predicate| self.is_filter(predicate))
        {
            return None;
        }
        Some(Spanned::new(
            ast::StepExpr::AxisStep(ast::AxisStep {
                axis: ast::Axis::Descendant,
                node_test: child.node_test.clone(),
                predicates: child.predicates.clone(),
            }),
            step.span,
        ))
    }

    /// Whether a predicate only keeps or drops a node, whatever its position.
    fn is_filter(&self, predicate: &ast::ExprS) -> bool {
        match predicate.value.0.as_slice() {
            [single] => self.is_truth(&single.value),
            _ => false,
        }
    }

    /// Whether an expression is never a number and free of positions.
    fn is_truth(&self, expression: &ast::ExprSingle) -> bool {
        match expression {
            ast::ExprSingle::Binary(binary) => {
                is_truth_operator(&binary.operator)
                    && self.path_is_free(&binary.left)
                    && self.path_is_free(&binary.right)
            }
            ast::ExprSingle::Path(path) => {
                self.path_is_free(path)
                    && match path.steps.as_slice() {
                        [only] => match &only.value {
                            ast::StepExpr::AxisStep(_) => true,
                            ast::StepExpr::PrimaryExpr(primary) => self.is_truth_call(&primary.value),
                            ast::StepExpr::PostfixExpr { .. } => false,
                        },
                        [.., last] => matches!(last.value, ast::StepExpr::AxisStep(_)),
                        [] => false,
                    }
            }
            _ => false,
        }
    }

    fn is_truth_call(&self, primary: &ast::PrimaryExpr) -> bool {
        match primary {
            ast::PrimaryExpr::FunctionCall(call) => self.truth_functions.contains(&call.name.value),
            _ => false,
        }
    }

    /// Whether nothing in a path asks for a position or a size.
    fn path_is_free(&self, path: &ast::PathExpr) -> bool {
        path.steps.iter().all(|step| match &step.value {
            ast::StepExpr::AxisStep(step) => step
                .predicates
                .iter()
                .all(|predicate| self.expr_is_free(&predicate.value)),
            ast::StepExpr::PrimaryExpr(primary) => self.primary_is_free(&primary.value),
            ast::StepExpr::PostfixExpr { .. } => false,
        })
    }

    fn expr_is_free(&self, expression: &ast::Expr) -> bool {
        expression
            .0
            .iter()
            .all(|single| self.single_is_free(&single.value))
    }

    fn single_is_free(&self, expression: &ast::ExprSingle) -> bool {
        match expression {
            ast::ExprSingle::Path(path) => self.path_is_free(path),
            ast::ExprSingle::Binary(binary) => {
                self.path_is_free(&binary.left) && self.path_is_free(&binary.right)
            }
            ast::ExprSingle::Apply(apply) => {
                self.path_is_free(&apply.path_expr)
                    && matches!(
                        apply.operator,
                        ast::ApplyOperator::Unary(_)
                            | ast::ApplyOperator::Cast(_)
                            | ast::ApplyOperator::Castable(_)
                            | ast::ApplyOperator::Treat(_)
                            | ast::ApplyOperator::InstanceOf(_)
                    )
            }
            _ => false,
        }
    }

    fn primary_is_free(&self, primary: &ast::PrimaryExpr) -> bool {
        match primary {
            ast::PrimaryExpr::Literal(_)
            | ast::PrimaryExpr::VarRef(_)
            | ast::PrimaryExpr::ContextItem => true,
            ast::PrimaryExpr::Expr(inner) => inner
                .value
                .as_ref()
                .map_or(true, |expression| self.expr_is_free(expression)),
            ast::PrimaryExpr::FunctionCall(call) => {
                self.is_position_free_function(&call.name.value)
                    && call
                        .arguments
                        .iter()
                        .all(|argument| self.single_is_free(&argument.value))
            }
            _ => false,
        }
    }
}

impl Descent {
    fn is_position_free_function(&self, name: &ast::Name) -> bool {
        name.namespace() == XS_NAMESPACE || self.position_free_functions.contains(name)
    }
}

/// Whether a step is the search through all descendants `//` stands for.
///
/// Whoever lists the predicates of the steps that are merged here has to
/// recognise a search the same way and to look at the same step after it, a
/// child step. If this changes, that listing changes with it.
fn is_search(step: &ast::StepExpr) -> bool {
    match step {
        ast::StepExpr::AxisStep(step) => {
            step.axis == ast::Axis::DescendantOrSelf
                && step.node_test == ast::NodeTest::KindTest(ast::KindTest::Any)
                && step.predicates.is_empty()
        }
        _ => false,
    }
}

/// Whether an operator always returns a boolean.
fn is_truth_operator(operator: &ast::BinaryOperator) -> bool {
    matches!(
        operator,
        ast::BinaryOperator::Or
            | ast::BinaryOperator::And
            | ast::BinaryOperator::ValueEq
            | ast::BinaryOperator::ValueNe
            | ast::BinaryOperator::ValueLt
            | ast::BinaryOperator::ValueLe
            | ast::BinaryOperator::ValueGt
            | ast::BinaryOperator::ValueGe
            | ast::BinaryOperator::GenEq
            | ast::BinaryOperator::GenNe
            | ast::BinaryOperator::GenLt
            | ast::BinaryOperator::GenLe
            | ast::BinaryOperator::GenGt
            | ast::BinaryOperator::GenGe
            | ast::BinaryOperator::Is
            | ast::BinaryOperator::Precedes
            | ast::BinaryOperator::Follows
    )
}
