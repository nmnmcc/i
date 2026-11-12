//! Facts for the syntax-three label migration (docs/design/syntax-3.md §7.2
//! stage 4, §7.3): the checker records, while it resolves and type checks,
//! the written places that `migrate-syntax3-labels` rewrites. Recording is
//! off unless a tool turns it on; the facts never change what is checked.

use crate::hir::{Expr, ExprKind, Stmt, ValueRef};
use ilang_syntax::span::Span;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// An argument as written: the span of its label, when it has one, and of
/// its value, and whether the value is a direct value (spec §6.10).
#[derive(Clone, Debug)]
pub struct ArgFact {
    pub label: Option<Span>,
    pub value: Span,
    pub direct: bool,
}

/// The written file the spans of a fact are in: its shown path and its
/// length. A fact from a checker run on other text (such as a macro
/// sandbox) is recognised by the length.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct File {
    pub shown: String,
    pub len: u32,
}

impl File {
    pub fn of(sources: &ilang_package::Sources, source: ilang_package::SourceId) -> File {
        let f = sources.get(source);
        File {
            shown: f.display.clone(),
            len: f.text.len() as u32,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Fact {
    /// A call of a function that is not a constructor with an argument by
    /// label, or a call of a constructor. `params` are the labels of the
    /// parameters (the field labels of a constructor); `to[i]` is the
    /// parameter of the argument `i`, counting the left side of `|>`
    /// (`first`) as argument 0.
    Call {
        file: File,
        span: Span,
        callee: Span,
        callee_direct: bool,
        ctor: bool,
        first: Option<(Span, bool)>,
        args: Vec<ArgFact>,
        params: Vec<Option<String>>,
        to: Vec<usize>,
    },
    /// A part of a constructor pattern matched to the field `label` by
    /// label, whose top is a name or `name: T` (§7.1).
    PatternPart {
        file: File,
        span: Span,
        label: String,
    },
    /// A label of a parameter of a function type: from the start of the
    /// label to the start of the type.
    FnTypeLabel {
        file: File,
        label: Span,
        ty: Span,
    },
}

static ON: AtomicBool = AtomicBool::new(false);
static FACTS: Mutex<Vec<Fact>> = Mutex::new(Vec::new());

/// Starts recording.
pub fn enable() {
    ON.store(true, Ordering::SeqCst);
}

pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub fn record(f: Fact) {
    FACTS.lock().expect("syntax-three facts").push(f);
}

/// The facts recorded since the last call.
pub fn take() -> Vec<Fact> {
    std::mem::take(&mut *FACTS.lock().expect("syntax-three facts"))
}

/// Spec §6.10: an expression made without running any code.
pub fn direct(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name(_)
        | ExprKind::Fn(_) => true,
        ExprKind::Unary { expr, .. } => {
            matches!(expr.kind, ExprKind::Int(_) | ExprKind::Float(_)) || literal_group(expr)
        }
        ExprKind::Tuple(items) => items.iter().all(direct),
        ExprKind::List { items, rest } => items.iter().chain(rest.as_deref()).all(direct),
        ExprKind::Record { base: None, fields } => fields.iter().all(|(_, e)| direct(e)),
        ExprKind::Call { callee, args } => {
            matches!(callee.kind, ExprKind::Name(ValueRef::Ctor(_)))
                && args.iter().all(|a| direct(&a.value))
        }
        ExprKind::Block(b) => matches!(&b.stmts[..], [Stmt::Expr(e)] if direct(e)),
        _ => false,
    }
}

fn literal_group(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) => true,
        ExprKind::Block(b) => matches!(&b.stmts[..], [Stmt::Expr(e)] if literal_group(e)),
        _ => false,
    }
}
