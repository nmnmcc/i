//! Structural traversal of AST data, including every span and string payload.
use crate::{ast::*, span::Span};

pub(crate) trait Visitor {
    fn expr(&mut self, _: &mut Expr, _: bool) {}
    fn span(&mut self, _: &mut Span) {}
    fn string(&mut self, _: &mut String) {}
    fn float(&mut self, _: &mut f64) {}
}
// Each queued pointer points into the exclusively borrowed root. A node is
// enqueued only once, and the visitor cannot access an ancestor while children
// are queued. Expr exit is queued before its children and runs only after them;
// thus a visitor may replace an expression on entry or exit without invalidating
// any pending child pointers.
pub(crate) enum Action {
    Node(*mut (), unsafe fn(*mut (), &mut Vec<Action>)),
    ExprEnter(*mut Expr),
    ExprExit(*mut Expr),
    Span(*mut Span),
    String(*mut String),
    Float(*mut f64),
}
pub(crate) trait Walk {
    fn enqueue(&mut self, work: &mut Vec<Action>);
    fn walk(&mut self, v: &mut impl Visitor)
    where
        Self: Sized,
    {
        let mut work = Vec::new();
        push(self, &mut work);
        while let Some(action) = work.pop() {
            let old = work.len();
            // All pointers are derived from the one exclusive root borrow;
            // see the invariant above. Descendant tasks finish before their
            // parent's exit callback can mutate the parent.
            unsafe {
                match action {
                    Action::Node(p, f) => f(p, &mut work),
                    Action::ExprEnter(p) => {
                        let e = &mut *p;
                        v.expr(e, true);
                        push(&mut e.kind, &mut work);
                        push(&mut e.span, &mut work);
                        work.push(Action::ExprExit(p));
                    }
                    Action::ExprExit(p) => v.expr(&mut *p, false),
                    Action::Span(p) => v.span(&mut *p),
                    Action::String(p) => v.string(&mut *p),
                    Action::Float(p) => v.float(&mut *p),
                }
            }
            // Implementations enqueue in source order; execute in that order.
            work[old..].reverse();
        }
    }
}
fn push<T: Walk>(node: &mut T, work: &mut Vec<Action>) {
    unsafe fn call<T: Walk>(p: *mut (), work: &mut Vec<Action>) {
        unsafe { (&mut *p.cast::<T>()).enqueue(work) }
    }
    work.push(Action::Node((node as *mut T).cast(), call::<T>));
}
impl Walk for Span {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        work.push(Action::Span(self));
    }
}
impl Walk for f64 {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        work.push(Action::Float(self));
    }
}
impl Walk for String {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        work.push(Action::String(self));
    }
}
impl<T: Walk> Walk for Vec<T> {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        for x in self {
            push(x, work);
        }
    }
}
impl<T: Walk> Walk for Option<T> {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        if let Some(x) = self {
            push(x, work);
        }
    }
}
impl<T: Walk> Walk for Box<T> {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        push(&mut **self, work);
    }
}
impl<A: Walk, B: Walk> Walk for (A, B) {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        push(&mut self.0, work);
        push(&mut self.1, work);
    }
}
macro_rules! fields {
    ($t:ty { $($f:ident),* } skip { $($skip:ident),* }) => {
        impl Walk for $t { fn enqueue(&mut self, work: &mut Vec<Action>) {
            let Self { $($f,)* $($skip: _,)* } = self;
            $(push($f, work);)*
        } }
    };
}
fields!(Ident { name, span } skip {});
fields!(Module { doc, imports, exports, items, imports_span, exports_span } skip {  });
fields!(ImportItem { alias, names, path, span } skip {});
fields!(ExportItem { name, span } skip { constructors });
fields!(Item { doc, attributes, kind, span } skip {});
fields!(Attribute { module, name, args, call, span } skip {});
fields!(TypeDef { name, params, head, body } skip { alias_pattern });
fields!(Constructor { name, fields, span } skip {});
fields!(Field { label, ty, span } skip {});
fields!(Binding { pattern, value, span, type_params } skip {});
fields!(Block { stmts, span } skip {});
impl Walk for Expr {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        work.push(Action::ExprEnter(self));
    }
}
fields!(Arg { label, value } skip {});
fields!(Clause { patterns, guard, body, span } skip {});
fields!(Function { params, ret, body } skip {});
fields!(RetType { value, err, env } skip {});
fields!(Format { name, arg } skip {});
fields!(Pattern { kind, span } skip {});
fields!(Binder { name, span } skip { discard });
fields!(TypeExpr { kind, span } skip {});
fields!(FnParamType { label, ty } skip {});
impl<T: Walk> Walk for Segment<T> {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        push(&mut self.value, work);
        push(&mut self.format, work);
        push(&mut self.span, work);
    }
}
impl Walk for ItemKind {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::TypeDef(t) => push(t, work),
            Self::Binding(b) => push(b, work),
            Self::Declaration {
                name,
                type_params,
                ty,
            } => {
                push(name, work);
                push(type_params, work);
                push(ty, work);
            }
            Self::MacroItem { module, name, args } => {
                push(module, work);
                push(name, work);
                push(args, work);
            }
        }
    }
}
impl Walk for TypeBody {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::Custom(cs) => push(cs, work),
            Self::Alias(t) => push(t, work),
        }
    }
}
impl Walk for Stmt {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::Bind(b) => push(b, work),
            Self::Expr(e) => push(e, work),
            Self::Env { ty, value, span } => {
                push(ty, work);
                push(value, work);
                push(span, work);
            }
        }
    }
}
impl Walk for Param {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::Pattern(p) => push(p, work),
            Self::Env { name, ty } => {
                push(name, work);
                push(ty, work);
            }
        }
    }
}
impl Walk for ExprKind {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::Quote(d) => push(d, work),
            Self::Int(_) => {}
            Self::Float(x) => push(x, work),
            Self::String(s) => push(s, work),
            Self::Name { module, name } => {
                push(module, work);
                push(name, work);
            }
            Self::Tuple(es) => push(es, work),
            Self::List { items, rest } => {
                push(items, work);
                push(rest, work);
            }
            Self::BitArray(ss) => push(ss, work),
            Self::Block(b) => push(b, work),
            Self::Fn(f) => push(f, work),
            Self::Case { subject, clauses } => {
                push(subject, work);
                push(clauses, work);
            }
            Self::Clauses(cs) => push(cs, work),
            Self::Call { callee, args } => {
                push(callee, work);
                push(args, work);
            }
            Self::Field { target, name } => {
                push(target, work);
                push(name, work);
            }
            Self::TupleIndex {
                target,
                index: _,
                index_span,
            } => {
                push(target, work);
                push(index_span, work);
            }
            Self::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                push(constructor, work);
                push(base, work);
                push(fields, work);
            }
            Self::Record { base, fields } => {
                push(base, work);
                push(fields, work);
            }
            Self::Unary { op: _, expr } => push(expr, work),
            Self::Binary { op: _, lhs, rhs } => {
                push(lhs, work);
                push(rhs, work);
            }
        }
    }
}
impl Walk for PatternKind {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::Quote(d) => push(d, work),
            Self::Int { .. } => {}
            Self::Float { value, .. } => push(value, work),
            Self::Var(s) | Self::Discard(s) | Self::String(s) => push(s, work),
            Self::Typed { binder, ty } => {
                push(binder, work);
                push(ty, work);
            }
            Self::Named { name, pattern } => {
                push(name, work);
                push(pattern, work);
            }
            Self::Prefix { prefix, rest } => {
                push(prefix, work);
                push(rest, work);
            }
            Self::Constructor {
                module,
                name,
                args,
                spread: _,
            } => {
                push(module, work);
                push(name, work);
                push(args, work);
            }
            Self::Tuple(ps) => push(ps, work),
            Self::List { items, rest } => {
                push(items, work);
                push(rest, work);
            }
            Self::BitArray(ss) => push(ss, work),
        }
    }
}
impl Walk for crate::lisp::Datum {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        use crate::lisp::DatumKind as D;
        push(&mut self.span, work);
        for doc in &mut self.docs {
            push(&mut doc.span, work);
            push(&mut doc.text, work);
        }
        match &mut self.kind {
            D::Name(s) | D::Discard(s) | D::UpName(s) | D::String(s) | D::Symbol(s) => {
                push(s, work)
            }
            D::Round(xs) | D::Square(xs) | D::Curly(xs) | D::Bits(xs) => push(xs, work),
            D::Quote(x) | D::Unquote(x) | D::Splice(x) => push(x, work),
            D::Int(_) => {}
            D::Float(x) => push(x, work),
        }
    }
}
impl Walk for TypeKind {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        match self {
            Self::Lit(LitValue::Str(s)) => push(s, work),
            Self::Lit(LitValue::Int(_)) => {}
            Self::Named { module, name, args } => {
                push(module, work);
                push(name, work);
                push(args, work);
            }
            Self::ValueOf { module, name } => {
                push(module, work);
                push(name, work);
            }
            Self::Call { module, name, args } => {
                push(module, work);
                push(name, work);
                push(args, work);
            }
            Self::Hole => {}
            Self::Tuple(ts) | Self::Union(ts) | Self::Inter(ts) | Self::Diff(ts) => push(ts, work),
            Self::Record {
                base,
                fields,
                removed,
            } => {
                push(base, work);
                push(fields, work);
                push(removed, work);
            }
            Self::Fn {
                params,
                ret,
                err,
                env,
            } => {
                push(params, work);
                push(ret, work);
                push(err, work);
                push(env, work);
            }
        }
    }
}

impl Walk for RecordLabel {
    fn enqueue(&mut self, work: &mut Vec<Action>) {
        push(&mut self.ident, work);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deeply_nested_walk_keeps_order_and_exits_after_children() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut e = Expr {
                    kind: ExprKind::String("leaf".into()),
                    span: Span::new(1, 2),
                };
                for _ in 0..10_000 {
                    e = Expr {
                        kind: ExprKind::Tuple(vec![e]),
                        span: Span::new(0, 3),
                    };
                }
                struct Count {
                    depth: usize,
                    max: usize,
                    entered: usize,
                    exited: usize,
                }
                impl Visitor for Count {
                    fn expr(&mut self, e: &mut Expr, enter: bool) {
                        if enter {
                            self.depth += 1;
                            self.max = self.max.max(self.depth);
                            self.entered += 1;
                            if let ExprKind::String(s) = &mut e.kind {
                                s.push('!');
                            }
                        } else {
                            assert!(self.depth > 0);
                            self.depth -= 1;
                            self.exited += 1;
                        }
                    }
                }
                let mut count = Count {
                    depth: 0,
                    max: 0,
                    entered: 0,
                    exited: 0,
                };
                e.walk(&mut count);
                assert_eq!(
                    (count.entered, count.exited, count.depth, count.max),
                    (10_001, 10_001, 0, 10_001)
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn replacing_expression_on_entry_visits_new_children() {
        struct Replace(Vec<String>);
        impl Visitor for Replace {
            fn expr(&mut self, e: &mut Expr, enter: bool) {
                if enter && matches!(&e.kind, ExprKind::Int(_)) {
                    e.kind = ExprKind::Tuple(vec![Expr {
                        kind: ExprKind::String("new".into()),
                        span: e.span,
                    }]);
                }
            }
            fn string(&mut self, s: &mut String) {
                self.0.push(s.clone());
            }
        }
        let mut e = Expr {
            kind: ExprKind::Int(1u32.into()),
            span: Span::new(0, 1),
        };
        let mut v = Replace(Vec::new());
        e.walk(&mut v);
        assert_eq!(v.0, ["new"]);
    }
}
