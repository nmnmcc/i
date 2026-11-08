//! Local, effect-preserving simplification over the shared-value IR.
//!
//! Arithmetic uses the language's integer semantics. Invalid floating-point
//! operations stay in the IR so their runtime error and source context survive.

use std::collections::BTreeSet;

use ilang_core::ast::BinOp;
use num_traits::Zero;

use crate::ir::{BitOptions, BitSegment, Clause, Expr, Pattern, Statement};

/// Simplify a subtree without moving, duplicating, or prematurely evaluating
/// calls, assertions, host interactions, or pattern-bound expressions.
pub fn expression(expr: Expr) -> Expr {
    match expr {
        Expr::Call { fun, args } => Expr::Call {
            fun: boxed(*fun),
            args: expressions(args),
        },
        Expr::Lambda { args, body, .. } => {
            let body = boxed(*body);
            let captures = crate::lower::free_variables(&body, &args);
            Expr::Lambda {
                args,
                captures,
                body,
            }
        }
        Expr::Block(statements) => {
            let mut statements: Vec<_> = statements.into_iter().map(statement).collect();
            if statements.len() == 1 && matches!(statements.first(), Some(Statement::Expr(_))) {
                let Statement::Expr(value) = statements.remove(0) else {
                    unreachable!()
                };
                value
            } else {
                Expr::Block(statements)
            }
        }
        Expr::Tuple(elements) => Expr::Tuple(expressions(elements)),
        Expr::List { elements, tail } => Expr::List {
            elements: expressions(elements),
            tail: tail.map(|value| boxed(*value)),
        },
        Expr::Record { tag, args } => Expr::Record {
            tag,
            args: expressions(args),
        },
        Expr::Field { record, index } => Expr::Field {
            record: boxed(*record),
            index,
        },
        Expr::RecordUpdate { record, updates } => Expr::RecordUpdate {
            record: boxed(*record),
            updates: updates
                .into_iter()
                .map(|(i, value)| (i, expression(value)))
                .collect(),
        },
        Expr::BinOp { op, lhs, rhs } => {
            let lhs = expression(*lhs);
            // These branches intentionally skip even optimizing the unreachable
            // subtree. Its references must not keep dead functions alive.
            match (op, &lhs) {
                (BinOp::And, Expr::Bool(false)) => return Expr::Bool(false),
                (BinOp::Or, Expr::Bool(true)) => return Expr::Bool(true),
                (BinOp::And, Expr::Bool(true)) | (BinOp::Or, Expr::Bool(false)) => {
                    return expression(*rhs);
                }
                _ => {}
            }
            let rhs = expression(*rhs);
            fold_binary(op, &lhs, &rhs).unwrap_or_else(|| Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
        }
        Expr::NegateBool(value) => match expression(*value) {
            Expr::Bool(value) => Expr::Bool(!value),
            value => Expr::NegateBool(Box::new(value)),
        },
        Expr::NegateInt(value) => match expression(*value) {
            Expr::Int(value) => Expr::Int(-value),
            value => Expr::NegateInt(Box::new(value)),
        },
        Expr::Case { subjects, clauses } => Expr::Case {
            subjects: expressions(subjects),
            clauses: clauses
                .into_iter()
                .map(|clause| Clause {
                    alternatives: clause
                        .alternatives
                        .into_iter()
                        .map(|patterns| patterns.into_iter().map(pattern).collect())
                        .collect(),
                    guard: clause.guard.map(expression),
                    body: expression(clause.body),
                })
                .collect(),
        },
        Expr::Panic { message, location } => Expr::Panic {
            message: boxed(*message),
            location,
        },
        Expr::Echo {
            value,
            message,
            location,
        } => Expr::Echo {
            value: boxed(*value),
            message: message.map(|value| boxed(*value)),
            location,
        },
        Expr::BitArray(segments) => Expr::BitArray(
            segments
                .into_iter()
                .map(|segment| BitSegment {
                    value: expression(segment.value),
                    options: options(segment.options),
                })
                .collect(),
        ),
        leaf @ (Expr::Nil
        | Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Var(_)
        | Expr::Function(_)
        | Expr::Constant(_)
        | Expr::Constructor { .. }) => leaf,
    }
}

fn boxed(value: Expr) -> Box<Expr> {
    Box::new(expression(value))
}

fn expressions(values: Vec<Expr>) -> Vec<Expr> {
    values.into_iter().map(expression).collect()
}

fn statement(stmt: Statement) -> Statement {
    match stmt {
        Statement::Expr(value) => Statement::Expr(expression(value)),
        Statement::Let {
            pattern: pat,
            value,
            assert,
            message,
            location,
        } => Statement::Let {
            pattern: pattern(pat),
            value: expression(value),
            assert,
            message: message.map(expression),
            location,
        },
        Statement::Assert {
            value,
            message,
            location,
        } => Statement::Assert {
            value: expression(value),
            message: message.map(expression),
            location,
        },
    }
}

fn pattern(pat: Pattern) -> Pattern {
    match pat {
        Pattern::Alias {
            name,
            pattern: inner,
        } => Pattern::Alias {
            name,
            pattern: Box::new(pattern(*inner)),
        },
        Pattern::Tuple(elements) => Pattern::Tuple(elements.into_iter().map(pattern).collect()),
        Pattern::List { elements, tail } => Pattern::List {
            elements: elements.into_iter().map(pattern).collect(),
            tail: tail.map(|tail| Box::new(pattern(*tail))),
        },
        Pattern::Record { tag, arguments } => Pattern::Record {
            tag,
            arguments: arguments.into_iter().map(pattern).collect(),
        },
        Pattern::BitArray(segments) => Pattern::BitArray(
            segments
                .into_iter()
                .map(|segment| BitSegment {
                    value: pattern(segment.value),
                    options: options(segment.options),
                })
                .collect(),
        ),
        Pattern::Value(value) => Pattern::Value(boxed(*value)),
        leaf @ (Pattern::Discard
        | Pattern::Variable(_)
        | Pattern::Nil
        | Pattern::Bool(_)
        | Pattern::Int(_)
        | Pattern::Float(_)
        | Pattern::String(_)
        | Pattern::StringPrefix { .. }) => leaf,
    }
}

fn options(mut options: BitOptions) -> BitOptions {
    options.size = options.size.map(|value| boxed(*value));
    options
}

fn fold_binary(op: BinOp, lhs: &Expr, rhs: &Expr) -> Option<Expr> {
    use BinOp::*;

    if matches!(op, Eq | NotEq) {
        return literal_equal(lhs, rhs)
            .map(|equal| Expr::Bool(if op == Eq { equal } else { !equal }));
    }

    match (lhs, rhs) {
        (Expr::Int(left), Expr::Int(right)) => Some(match op {
            AddInt => Expr::Int(left + right),
            SubInt => Expr::Int(left - right),
            MultInt => Expr::Int(left * right),
            DivInt => Expr::Int(if right.is_zero() {
                Default::default()
            } else {
                left / right
            }),
            RemainderInt => Expr::Int(if right.is_zero() {
                Default::default()
            } else {
                left % right
            }),
            LtInt => Expr::Bool(left < right),
            LtEqInt => Expr::Bool(left <= right),
            GtInt => Expr::Bool(left > right),
            GtEqInt => Expr::Bool(left >= right),
            _ => return None,
        }),
        (Expr::Float(left), Expr::Float(right)) if left.is_finite() && right.is_finite() => {
            let value = match op {
                AddFloat => left + right,
                SubFloat => left - right,
                MultFloat => left * right,
                DivFloat if *right != 0.0 => left / right,
                LtFloat => return Some(Expr::Bool(left < right)),
                LtEqFloat => return Some(Expr::Bool(left <= right)),
                GtFloat => return Some(Expr::Bool(left > right)),
                GtEqFloat => return Some(Expr::Bool(left >= right)),
                _ => return None,
            };
            value.is_finite().then_some(Expr::Float(value))
        }
        (Expr::String(left), Expr::String(right)) if op == Concatenate => {
            let mut text = String::with_capacity(left.len() + right.len());
            text.push_str(left);
            text.push_str(right);
            Some(Expr::String(text))
        }
        _ => None,
    }
}

/// The entire value must be literal before equality can remove its evaluation.
/// In particular, unequal tuple tags or lengths do not justify discarding calls
/// that occur in their fields.
fn literal_equal(lhs: &Expr, rhs: &Expr) -> Option<bool> {
    if !is_literal(lhs) || !is_literal(rhs) {
        return None;
    }
    Some(compare_literals(lhs, rhs))
}

fn is_literal(value: &Expr) -> bool {
    match value {
        Expr::Nil | Expr::Bool(_) | Expr::Int(_) | Expr::String(_) => true,
        Expr::Float(value) => value.is_finite(),
        Expr::Tuple(values)
        | Expr::Record { args: values, .. }
        | Expr::List {
            elements: values,
            tail: None,
        } => values.iter().all(is_literal),
        _ => false,
    }
}

fn compare_literals(lhs: &Expr, rhs: &Expr) -> bool {
    match (lhs, rhs) {
        (Expr::Nil, Expr::Nil) => true,
        (Expr::Bool(a), Expr::Bool(b)) => a == b,
        (Expr::Int(a), Expr::Int(b)) => a == b,
        (Expr::Float(a), Expr::Float(b)) => a == b,
        (Expr::String(a), Expr::String(b)) => a == b,
        (Expr::Tuple(a), Expr::Tuple(b)) => sequence_equal(a, b),
        (
            Expr::Record {
                tag: a_tag,
                args: a,
            },
            Expr::Record {
                tag: b_tag,
                args: b,
            },
        ) => a_tag == b_tag && sequence_equal(a, b),
        (
            Expr::List {
                elements: a,
                tail: None,
            },
            Expr::List {
                elements: b,
                tail: None,
            },
        ) => sequence_equal(a, b),
        _ => false,
    }
}

fn sequence_equal(lhs: &[Expr], rhs: &[Expr]) -> bool {
    lhs.len() == rhs.len() && lhs.iter().zip(rhs).all(|(a, b)| compare_literals(a, b))
}

/// Return dependencies in the simplified tree, including expressions embedded
/// in pattern sizes and lazy failure messages.
pub fn references(expr: &Expr) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    visit(expr, &mut found);
    found
}

fn visit(expr: &Expr, found: &mut BTreeSet<String>) {
    match expr {
        Expr::Function(symbol) | Expr::Constant(symbol) => {
            found.insert(symbol.clone());
        }
        Expr::Call { fun, args } => {
            visit(fun, found);
            visit_all(args, found);
        }
        Expr::Lambda { body, .. } => visit(body, found),
        Expr::Block(statements) => {
            for stmt in statements {
                match stmt {
                    Statement::Expr(value) => visit(value, found),
                    Statement::Let {
                        pattern,
                        value,
                        message,
                        ..
                    } => {
                        visit_pattern(pattern, found);
                        visit(value, found);
                        if let Some(message) = message {
                            visit(message, found);
                        }
                    }
                    Statement::Assert { value, message, .. } => {
                        visit(value, found);
                        if let Some(message) = message {
                            visit(message, found);
                        }
                    }
                }
            }
        }
        Expr::Tuple(values) | Expr::Record { args: values, .. } => visit_all(values, found),
        Expr::List { elements, tail } => {
            visit_all(elements, found);
            if let Some(tail) = tail {
                visit(tail, found);
            }
        }
        Expr::Field { record, .. } => visit(record, found),
        Expr::RecordUpdate { record, updates } => {
            visit(record, found);
            for (_, value) in updates {
                visit(value, found);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            visit(lhs, found);
            visit(rhs, found);
        }
        Expr::NegateBool(value) | Expr::NegateInt(value) => visit(value, found),
        Expr::Case { subjects, clauses } => {
            visit_all(subjects, found);
            for clause in clauses {
                for alternatives in &clause.alternatives {
                    for pat in alternatives {
                        visit_pattern(pat, found);
                    }
                }
                if let Some(guard) = &clause.guard {
                    visit(guard, found);
                }
                visit(&clause.body, found);
            }
        }
        Expr::Panic { message, .. } => visit(message, found),
        Expr::Echo { value, message, .. } => {
            visit(value, found);
            if let Some(message) = message {
                visit(message, found);
            }
        }
        Expr::BitArray(segments) => {
            for segment in segments {
                visit(&segment.value, found);
                if let Some(size) = &segment.options.size {
                    visit(size, found);
                }
            }
        }
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Var(_)
        | Expr::Constructor { .. } => {}
    }
}

fn visit_all(values: &[Expr], found: &mut BTreeSet<String>) {
    for value in values {
        visit(value, found);
    }
}

fn visit_pattern(pattern: &Pattern, found: &mut BTreeSet<String>) {
    match pattern {
        Pattern::Alias { pattern, .. } => visit_pattern(pattern, found),
        Pattern::Tuple(elements)
        | Pattern::Record {
            arguments: elements,
            ..
        } => {
            for pattern in elements {
                visit_pattern(pattern, found);
            }
        }
        Pattern::List { elements, tail } => {
            for pattern in elements {
                visit_pattern(pattern, found);
            }
            if let Some(tail) = tail {
                visit_pattern(tail, found);
            }
        }
        Pattern::BitArray(segments) => {
            for segment in segments {
                if let Some(size) = &segment.options.size {
                    visit(size, found);
                }
                visit_pattern(&segment.value, found);
            }
        }
        Pattern::Value(value) => visit(value, found),
        Pattern::Discard
        | Pattern::Variable(_)
        | Pattern::Nil
        | Pattern::Bool(_)
        | Pattern::Int(_)
        | Pattern::Float(_)
        | Pattern::String(_)
        | Pattern::StringPrefix { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Location;

    fn bin(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
        expression(Expr::BinOp {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        })
    }

    fn int(value: i64) -> Expr {
        Expr::Int(value.into())
    }

    fn assert_int(expression: Expr, expected: i64) {
        let Expr::Int(value) = expression else {
            panic!("expected integer, got {expression:?}")
        };
        assert_eq!(value, expected.into());
    }

    #[test]
    fn division_and_remainder_match_erlang_sign_and_zero_semantics() {
        for (left, right, quotient, remainder) in [
            (-7, 3, -2, -1),
            (7, -3, -2, 1),
            (-7, -3, 2, -1),
            (12, 0, 0, 0),
            (-12, 0, 0, 0),
            (0, 0, 0, 0),
        ] {
            assert_int(bin(BinOp::DivInt, int(left), int(right)), quotient);
            assert_int(bin(BinOp::RemainderInt, int(left), int(right)), remainder);
        }
    }

    #[test]
    fn short_circuit_removes_only_unreachable_effects_and_dependencies() {
        let effect = Expr::Call {
            fun: Box::new(Expr::Function("host.effect".into())),
            args: vec![],
        };
        let discarded = bin(BinOp::And, Expr::Bool(false), effect.clone());
        assert!(matches!(discarded, Expr::Bool(false)));
        assert!(references(&discarded).is_empty());
        let required = bin(BinOp::And, effect, Expr::Bool(false));
        assert!(matches!(required, Expr::BinOp { .. }));
        assert_eq!(
            references(&required),
            BTreeSet::from(["host.effect".into()])
        );
    }

    #[test]
    fn invalid_float_operations_remain_runtime_operations() {
        assert!(matches!(
            bin(BinOp::MultFloat, Expr::Float(f64::MAX), Expr::Float(2.0)),
            Expr::BinOp { .. }
        ));
        assert!(matches!(
            bin(BinOp::DivFloat, Expr::Float(1.0), Expr::Float(-0.0)),
            Expr::BinOp { .. }
        ));
        assert!(
            matches!(bin(BinOp::AddFloat, Expr::Float(0.25), Expr::Float(0.5)), Expr::Float(value) if value == 0.75)
        );
    }

    #[test]
    fn concatenation_preserves_utf8_and_embedded_zero() {
        assert!(
            matches!(bin(BinOp::Concatenate, Expr::String("你好\0".into()), Expr::String("🦀".into())), Expr::String(value) if value == "你好\0🦀")
        );
    }

    #[test]
    fn unequal_aggregate_shape_does_not_remove_field_effects() {
        let effect = Expr::Echo {
            value: Box::new(int(1)),
            message: None,
            location: Location::default(),
        };
        let result = bin(BinOp::Eq, Expr::Tuple(vec![effect]), Expr::Tuple(vec![]));
        assert!(matches!(result, Expr::BinOp { .. }));
    }

    #[test]
    fn references_include_pattern_sizes_and_lazy_failure_messages() {
        let expr = Expr::Block(vec![Statement::Let {
            pattern: Pattern::BitArray(vec![BitSegment {
                value: Pattern::Discard,
                options: BitOptions {
                    size: Some(Box::new(Expr::Constant("sizes.width".into()))),
                    ..Default::default()
                },
            }]),
            value: Expr::Constant("data.bytes".into()),
            assert: true,
            message: Some(Expr::Call {
                fun: Box::new(Expr::Function("errors.describe".into())),
                args: vec![],
            }),
            location: Location::default(),
        }]);
        assert_eq!(
            references(&expr),
            BTreeSet::from([
                "sizes.width".into(),
                "data.bytes".into(),
                "errors.describe".into()
            ])
        );
    }

    #[test]
    fn eliminated_short_circuit_branch_drops_unused_closure_capture() {
        let expr = Expr::Lambda {
            args: vec![],
            captures: vec!["outer".into()],
            body: Box::new(Expr::BinOp {
                op: BinOp::Or,
                lhs: Box::new(Expr::Bool(true)),
                rhs: Box::new(Expr::Var("outer".into())),
            }),
        };
        let Expr::Lambda { captures, body, .. } = expression(expr) else {
            panic!("expected lambda")
        };
        assert!(captures.is_empty());
        assert!(matches!(*body, Expr::Bool(true)));
    }
}
