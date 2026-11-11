//! Structural traversal of AST data, including every span and string payload.
use crate::{ast::*, span::Span};

pub(crate) trait Visitor {
    fn expr(&mut self, _: &mut Expr, _: bool) {}
    fn pattern(&mut self, _: &mut Pattern) {}
    fn float(&mut self, _: &mut f64) {}
    fn span(&mut self, _: &mut Span) {}
    fn string(&mut self, _: &mut String) {}
}
pub(crate) trait Walk {
    fn walk(&mut self, v: &mut impl Visitor);
}
impl Walk for Span {
    fn walk(&mut self, v: &mut impl Visitor) {
        v.span(self);
    }
}
impl Walk for String {
    fn walk(&mut self, v: &mut impl Visitor) {
        v.string(self);
    }
}
impl<T: Walk> Walk for Vec<T> {
    fn walk(&mut self, v: &mut impl Visitor) {
        for x in self {
            x.walk(v);
        }
    }
}
impl<T: Walk> Walk for Option<T> {
    fn walk(&mut self, v: &mut impl Visitor) {
        if let Some(x) = self {
            x.walk(v);
        }
    }
}
impl<T: Walk> Walk for Box<T> {
    fn walk(&mut self, v: &mut impl Visitor) {
        (**self).walk(v);
    }
}
impl<A: Walk, B: Walk> Walk for (A, B) {
    fn walk(&mut self, v: &mut impl Visitor) {
        self.0.walk(v);
        self.1.walk(v);
    }
}
macro_rules! fields {
    ($t:ty { $($f:ident),* } skip { $($skip:ident),* }) => {
        impl Walk for $t { fn walk(&mut self, v: &mut impl Visitor) {
            let Self { $($f,)* $($skip: _,)* } = self;
            $($f.walk(v);)*
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
    fn walk(&mut self, v: &mut impl Visitor) {
        v.expr(self, true);
        self.kind.walk(v);
        self.span.walk(v);
        v.expr(self, false);
    }
}
fields!(Arg { label, value } skip {});
fields!(Clause { patterns, guard, body, span } skip {});
fields!(Function { params, ret, body } skip {});
fields!(RetType { value, err, env } skip {});
fields!(Format { name, arg } skip {});
impl Walk for Pattern {
    fn walk(&mut self, v: &mut impl Visitor) {
        v.pattern(self);
        self.kind.walk(v);
        self.span.walk(v);
    }
}
fields!(Binder { name, span } skip { discard });
fields!(TypeExpr { kind, span } skip {});
fields!(FnParamType { label, ty } skip {});
impl<T: Walk> Walk for Segment<T> {
    fn walk(&mut self, v: &mut impl Visitor) {
        self.value.walk(v);
        self.format.walk(v);
        self.span.walk(v);
    }
}
impl Walk for ItemKind {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::TypeDef(t) => t.walk(v),
            Self::Binding(b) => b.walk(v),
            Self::Declaration {
                name,
                type_params,
                ty,
            } => {
                name.walk(v);
                type_params.walk(v);
                ty.walk(v);
            }
            Self::MacroItem { module, name, args } => {
                module.walk(v);
                name.walk(v);
                args.walk(v);
            }
        }
    }
}
impl Walk for TypeBody {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::Custom(cs) => cs.walk(v),
            Self::Alias(t) => t.walk(v),
        }
    }
}
impl Walk for Stmt {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::Bind(b) => b.walk(v),
            Self::Expr(e) => e.walk(v),
            Self::Env { ty, value, span } => {
                ty.walk(v);
                value.walk(v);
                span.walk(v);
            }
        }
    }
}
impl Walk for Param {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::Pattern(p) => p.walk(v),
            Self::Env { name, ty } => {
                name.walk(v);
                ty.walk(v);
            }
        }
    }
}
impl Walk for ExprKind {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::Int(_) => {}
            Self::Quote(d) => d.walk(v),
            Self::Float(value) => v.float(value),
            Self::String(s) => s.walk(v),
            Self::Name { module, name } => {
                module.walk(v);
                name.walk(v);
            }
            Self::Tuple(es) => es.walk(v),
            Self::List { items, rest } => {
                items.walk(v);
                rest.walk(v);
            }
            Self::BitArray(ss) => ss.walk(v),
            Self::Block(b) => b.walk(v),
            Self::Fn(f) => f.walk(v),
            Self::Case { subject, clauses } => {
                subject.walk(v);
                clauses.walk(v);
            }
            Self::Clauses(cs) => cs.walk(v),
            Self::Call { callee, args } => {
                callee.walk(v);
                args.walk(v);
            }
            Self::Field { target, name } => {
                target.walk(v);
                name.walk(v);
            }
            Self::TupleIndex {
                target,
                index: _,
                index_span,
            } => {
                target.walk(v);
                index_span.walk(v);
            }
            Self::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                constructor.walk(v);
                base.walk(v);
                fields.walk(v);
            }
            Self::Record { base, fields } => {
                base.walk(v);
                fields.walk(v);
            }
            Self::Unary { op: _, expr } => expr.walk(v),
            Self::Binary { op: _, lhs, rhs } => {
                lhs.walk(v);
                rhs.walk(v);
            }
        }
    }
}
impl Walk for PatternKind {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::Int { .. } => {}
            Self::Float { value, .. } => v.float(value),
            Self::Quote { datum, captures, .. } => { datum.walk(v); for (_, pattern) in captures { pattern.walk(v); } }
            Self::Var(s) | Self::Discard(s) | Self::String(s) => s.walk(v),
            Self::Typed { binder, ty } => {
                binder.walk(v);
                ty.walk(v);
            }
            Self::Named { name, pattern } => {
                name.walk(v);
                pattern.walk(v);
            }
            Self::Prefix { prefix, rest } => {
                prefix.walk(v);
                rest.walk(v);
            }
            Self::Constructor {
                module,
                name,
                args,
                spread: _,
            } => {
                module.walk(v);
                name.walk(v);
                args.walk(v);
            }
            Self::Tuple(ps) => ps.walk(v),
            Self::List { items, rest } => {
                items.walk(v);
                rest.walk(v);
            }
            Self::BitArray(ss) => ss.walk(v),
        }
    }
}
impl Walk for TypeKind {
    fn walk(&mut self, v: &mut impl Visitor) {
        match self {
            Self::Lit(LitValue::Str(s)) => s.walk(v),
            Self::Lit(LitValue::Int(_)) => {}
            Self::Named { module, name, args } => {
                module.walk(v);
                name.walk(v);
                args.walk(v);
            }
            Self::ValueOf { module, name } => {
                module.walk(v);
                name.walk(v);
            }
            Self::Call { module, name, args } => {
                module.walk(v);
                name.walk(v);
                args.walk(v);
            }
            Self::Hole => {}
            Self::Tuple(ts) | Self::Union(ts) | Self::Inter(ts) | Self::Diff(ts) => ts.walk(v),
            Self::Record {
                base,
                fields,
                removed,
            } => {
                base.walk(v);
                fields.walk(v);
                removed.walk(v);
            }
            Self::Fn {
                params,
                ret,
                err,
                env,
            } => {
                params.walk(v);
                ret.walk(v);
                err.walk(v);
                env.walk(v);
            }
        }
    }
}

impl Walk for RecordLabel {
    fn walk(&mut self, v: &mut impl Visitor) {
        self.ident.walk(v);
    }
}

impl Walk for crate::lisp::Datum {
    fn walk(&mut self, v: &mut impl Visitor) {
        use crate::lisp::DatumKind as D;
        self.span.walk(v);
        for doc in &mut self.docs { doc.span.walk(v); doc.text.walk(v); }
        match &mut self.kind {
            D::Name(s) | D::Discard(s) | D::UpName(s) | D::String(s) | D::Symbol(s) => s.walk(v),
            D::Round(xs) | D::Square(xs) | D::Curly(xs) | D::Bits(xs) => xs.walk(v),
            D::Quote(x) | D::Unquote(x) | D::Splice(x) => x.walk(v),
            D::Int(_) => {},
            D::Float(x) => v.float(x),
        }
    }
}
