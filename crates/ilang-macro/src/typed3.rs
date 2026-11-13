//! The parts of `typed` that read or build the tree, on the tree of syntax
//! three: the expressions of a module by span, the spans of an output, the
//! function boundaries around a forwarded argument, and the wrappers that
//! the driver builds.
//!
//! The visits are plain recursions (`syn3::walk`): the driver runs them on
//! a deep stack when the tree can hold the result of a macro.
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::{
    Binding, Block, DItem, DPItem, Expr, ExprKind, Format, Function, Ident, Item, ItemKind,
    Module, Pattern, PatternKind, Stmt, TItem, Type, TypeKind,
};
use ilang_syntax::syn3::walk::{self, Visitor};
use std::collections::HashMap;

struct Expressions(HashMap<Span, Expr>);
impl Visitor for Expressions {
    fn expr(&mut self, e: &mut Expr, enter: bool) {
        if enter {
            self.0.insert(e.span.0, e.clone());
        }
    }
}

/// Every expression of a module, by its span.
pub fn module_expressions(module: &Module) -> HashMap<Span, Expr> {
    let mut expressions = Expressions(HashMap::new());
    walk::module(&mut module.clone(), &mut expressions);
    expressions.0
}

/// Every expression inside an expression (and the expression), by its span.
pub fn expressions(expression: &Expr) -> HashMap<Span, Expr> {
    let mut expressions = Expressions(HashMap::new());
    walk::expr(&mut expression.clone(), &mut expressions);
    expressions.0
}

/// Visits the span of every expression inside an expression, outside-in.
pub fn for_each_expression_span(expression: &Expr, visit: impl FnMut(Span)) {
    struct Spans<F>(F);
    impl<F: FnMut(Span)> Visitor for Spans<F> {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if enter {
                (self.0)(e.span.0);
            }
        }
    }
    walk::expr(&mut expression.clone(), &mut Spans(visit));
}

/// Visits every span of an expression: of each expression, pattern, type,
/// block, binding and function, and of each written name in them.
pub fn for_each_span(expression: &Expr, visit: impl FnMut(Span)) {
    struct Spans<F>(F);
    impl<F: FnMut(Span)> Spans<F> {
        fn id(&mut self, id: &Ident) {
            (self.0)(id.span.0);
        }
        fn opt(&mut self, id: &Option<Ident>) {
            if let Some(id) = id {
                self.id(id);
            }
        }
        fn format(&mut self, format: &Option<Format>) {
            if let Some(format) = format {
                self.id(&format.name);
            }
        }
        fn ditems(&mut self, items: &[DItem]) {
            for item in items {
                if let DItem::Label(label, _) = item {
                    self.id(label);
                }
            }
        }
        fn dpitems(&mut self, items: &[DPItem]) {
            for item in items {
                match item {
                    DPItem::Key(key, _) => self.id(key),
                    DPItem::Rest(Some(rest)) => (self.0)(rest.span.0),
                    DPItem::Pos(_) | DPItem::Rest(None) => {}
                }
            }
        }
        fn function(&mut self, f: &Function) {
            (self.0)(f.span.0);
            f.generics.iter().for_each(|g| self.id(g));
            for param in &f.params {
                self.opt(&param.key);
            }
            for service in f.services.iter().flatten() {
                self.id(&service.name);
            }
        }
    }
    impl<F: FnMut(Span)> Visitor for Spans<F> {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if !enter {
                return;
            }
            (self.0)(e.span.0);
            match &e.kind {
                ExprKind::Name { module, name } => {
                    self.opt(module);
                    self.id(name);
                }
                ExprKind::Field { name, .. } => self.id(name),
                ExprKind::Data(items) | ExprKind::Call { args: items, .. } => self.ditems(items),
                ExprKind::BitArray(segments) => {
                    for segment in segments {
                        self.format(&segment.format);
                    }
                }
                ExprKind::Chain(fs) => fs.iter().for_each(|f| self.function(f)),
                _ => {}
            }
        }
        fn pattern(&mut self, p: &mut Pattern, enter: bool) {
            if !enter {
                return;
            }
            (self.0)(p.span.0);
            match &p.kind {
                PatternKind::Typed { binder, .. } | PatternKind::Prefix { rest: binder, .. } => {
                    (self.0)(binder.span.0)
                }
                PatternKind::Ctor { module, name, args } => {
                    self.opt(module);
                    self.id(name);
                    if let Some(args) = args {
                        self.dpitems(args);
                    }
                }
                PatternKind::Data(items) => self.dpitems(items),
                PatternKind::List {
                    rest: Some(Some(rest)),
                    ..
                } => (self.0)(rest.span.0),
                PatternKind::BitArray(segments) => {
                    for segment in segments {
                        self.format(&segment.format);
                    }
                }
                _ => {}
            }
        }
        fn ty(&mut self, t: &mut Type, enter: bool) {
            if !enter {
                return;
            }
            (self.0)(t.span.0);
            match &t.kind {
                TypeKind::Named { module, name, .. }
                | TypeKind::ValueOf { module, name }
                | TypeKind::Call { module, name, .. } => {
                    self.opt(module);
                    self.id(name);
                }
                TypeKind::Data(items) => {
                    for item in items {
                        if let TItem::Label(label, _) = item {
                            self.id(label);
                        }
                    }
                }
                TypeKind::Fn(f) => f.generics.iter().for_each(|g| self.id(g)),
                _ => {}
            }
        }
        fn block(&mut self, b: &mut Block, enter: bool) {
            if !enter {
                return;
            }
            (self.0)(b.span.0);
            for stmt in &b.stmts {
                if let Stmt::Bind(binding) = stmt {
                    (self.0)(binding.span.0);
                }
            }
        }
    }
    walk::expr(&mut expression.clone(), &mut Spans(visit));
}

/// Whether an expression at one of the `holes` stands inside a function
/// of the expression: a chain, but not the branches of a match
/// (`a |> F1 | F2`, the old tree's `Case`).
pub fn hole_inside_function(expression: &Expr, mut is_hole: impl FnMut(Span) -> bool) -> bool {
    struct Delayed<F> {
        depth: usize,
        is_hole: F,
        /// The chains that are the branches of a match.
        matches: Vec<*const Expr>,
        found: bool,
    }
    impl<F: FnMut(Span) -> bool> Delayed<F> {
        fn function(&self, e: &Expr) -> bool {
            matches!(e.kind, ExprKind::Chain(_)) && !self.matches.contains(&(e as *const Expr))
        }
    }
    impl<F: FnMut(Span) -> bool> Visitor for Delayed<F> {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if enter {
                if self.depth > 0 && (self.is_hole)(e.span.0) {
                    self.found = true;
                }
                if let ExprKind::Pipe { rhs, .. } = &e.kind
                    && let ExprKind::Chain(fs) = &rhs.kind
                    && crate::body_walk3::is_match(fs)
                {
                    self.matches.push(&**rhs as *const Expr);
                }
                if self.function(e) {
                    self.depth += 1;
                }
            } else {
                if self.function(e) {
                    self.depth -= 1;
                }
                if matches!(e.kind, ExprKind::Pipe { .. }) {
                    let e: &Expr = e;
                    if let ExprKind::Pipe { rhs, .. } = &e.kind {
                        let rhs = &**rhs as *const Expr;
                        self.matches.retain(|m| *m != rhs);
                    }
                }
            }
        }
    }
    let mut delayed = Delayed {
        depth: 0,
        is_hole: &mut is_hole,
        matches: Vec::new(),
        found: false,
    };
    walk::expr(&mut expression.clone(), &mut delayed);
    delayed.found
}

/// `() -> T { }`: the function that carries the written output type of a
/// typed macro to the resolver.
pub fn output_type_function(ty: Type) -> Expr {
    let span = ty.span.0;
    Expr::new(
        ExprKind::Chain(vec![Function {
            generics: Vec::new(),
            params: Vec::new(),
            guard: None,
            services: None,
            ret: Some(ty),
            body: Block {
                stmts: Vec::new(),
                span: Span::default().into(),
            },
            span: span.into(),
        }]),
        span,
    )
}

/// A module of one item `_ = value`, for the body walker.
pub fn wrapper(value: Expr, call: Span) -> Module {
    let span = value.span.0;
    Module {
        items: vec![Item {
            doc: Vec::new(),
            attributes: Vec::new(),
            span: span.into(),
            kind: ItemKind::Bind(Binding {
                pattern: Pattern::new(PatternKind::Discard("_".into()), span),
                value,
                span: call.into(),
            }),
        }],
        ..Module::default()
    }
}

/// The value of a [`wrapper`].
pub fn unwrap(mut wrapper: Module) -> Expr {
    let Some(Item {
        kind: ItemKind::Bind(binding),
        ..
    }) = wrapper.items.pop()
    else {
        unreachable!("an output wrapper has one binding")
    };
    binding.value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expression(source: &str) -> Expr {
        ilang_syntax::syn3::parse_expr(source).unwrap_or_else(|e| panic!("{source}: {e:?}"))
    }

    fn span_of(source: &str, part: &str) -> Span {
        let start = source.find(part).unwrap() as u32;
        Span::new(start, start + part.len() as u32)
    }

    #[test]
    fn a_hole_is_delayed_inside_a_function_and_not_inside_a_match() {
        let delayed = |source: &str| {
            let hole = span_of(source, "hole");
            hole_inside_function(&expression(source), |span| span == hole)
        };
        assert!(!delayed("f(hole)"));
        assert!(delayed("(x) { f(hole, x) }"));
        assert!(delayed("g((0) { hole } | (_) { 1 })"));
        assert!(!delayed("x |> (0) { hole } | (_) { 1 }"));
        assert!(delayed("x |> (0) { (y) { hole } } | (_) { 1 }"));
        assert!(delayed("x |> (y) -> Int { hole }"));
        assert!(!delayed("(x |> (0) { 1 } | (_) { 2 }) + hole"));
    }

    #[test]
    fn every_expression_and_every_name_has_its_span_visited() {
        let source = "(a, b: Int) <- (s: S) { C(k = a) |> m.f(lab = b.x) }";
        let e = expression(source);
        let mut spans = Vec::new();
        for_each_span(&e, |span| spans.push(span));
        for part in ["a", "b", "Int", "s", "S", "C(k = a)", "k", "m", "f", "lab", "b.x", "x"] {
            assert!(spans.contains(&span_of(source, part)), "{part}");
        }
        let by_span = expressions(&e);
        assert!(by_span.contains_key(&span_of(source, "b.x")));
        let mut count = 0;
        for_each_expression_span(&e, |_| count += 1);
        assert_eq!(count, by_span.len());
    }

    #[test]
    fn the_wrapper_keeps_the_value() {
        let e = expression("f(1)");
        let module = wrapper(e.clone(), Span::new(3, 4));
        assert_eq!(module.items.len(), 1);
        assert_eq!(unwrap(module), e);
        let ty = ilang_syntax::syn3::parse_type("List(Int)").unwrap();
        let function = output_type_function(ty.clone());
        let ExprKind::Chain(fs) = &function.kind else {
            panic!()
        };
        assert_eq!(fs[0].ret, Some(ty));
    }
}
