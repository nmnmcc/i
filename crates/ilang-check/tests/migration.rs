//! Independent resolve checks for the syntax-only migration (A1 R4).
mod common;
#[path = "../../ilang-syntax/tests/common.rs"]
mod corpus;
use ilang_check::hir::*;
use ilang_syntax::{
    ast,
    migrate::{self, Env},
    span::Span,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
struct Snapshot {
    owners: HashMap<TyVarId, Span>,
    locals: HashMap<VarId, Span>,
    types: Vec<(Span, TyVarId)>,
    uses: Vec<(Span, ValueRef)>,
    patterns: Vec<Span>,
}
impl Snapshot {
    fn binding(&mut self, b: &Binding) {
        self.patterns.push(b.pattern.span);
        for v in &b.ty_vars {
            self.owners.insert(*v, b.span);
        }
        self.pattern(&b.pattern);
        self.expr(&b.value);
    }
    fn ty(&mut self, t: &Type) {
        match &t.kind {
            TypeKind::Var(v) => self.types.push((t.span, *v)),
            TypeKind::Named { args, .. }
            | TypeKind::Tuple(args)
            | TypeKind::Union(args)
            | TypeKind::Inter(args)
            | TypeKind::Diff(args) => {
                for t in args {
                    self.ty(t);
                }
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                for p in params {
                    self.ty(&p.ty);
                }
                self.ty(ret);
                for t in err.iter().chain(env) {
                    self.ty(t);
                }
            }
            _ => {}
        }
    }
    fn binder(&mut self, b: &Binder) {
        self.locals.insert(b.var, b.span);
    }
    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Var(b) => self.binder(b),
            PatternKind::Typed { binder, ty } => {
                if let Some(b) = binder {
                    self.binder(b);
                }
                self.ty(ty);
            }
            PatternKind::Named { binder, pattern } => {
                self.binder(binder);
                self.pattern(pattern);
            }
            PatternKind::Prefix { rest: Some(b), .. } => self.binder(b),
            PatternKind::Constructor { args, .. }
            | PatternKind::Tuple(args)
            | PatternKind::Error(args) => {
                for p in args {
                    self.pattern(p);
                }
            }
            PatternKind::List { items, rest } => {
                for p in items {
                    self.pattern(p);
                }
                if let Some(Some(b)) = rest {
                    self.binder(b);
                }
            }
            PatternKind::BitArray(ss) => {
                for s in ss {
                    self.pattern(&s.value);
                    if let Some(e) = s.format.as_ref().and_then(|f| f.arg.as_deref()) {
                        self.expr(e);
                    }
                }
            }
            _ => {}
        }
    }
    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => self.binding(b),
                Stmt::Env { ty, value, .. } => {
                    self.ty(ty);
                    self.expr(value);
                }
                Stmt::Expr(e) => self.expr(e),
            }
        }
    }
    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(v) => self.uses.push((e.span, *v)),
            ExprKind::Fn(f) => {
                for p in &f.params {
                    match p {
                        Param::Pattern { pattern, .. } => self.pattern(pattern),
                        Param::Env { var, name, ty } => {
                            self.locals.insert(*var, name.span);
                            self.ty(ty);
                        }
                    }
                }
                if let Some(r) = &f.ret {
                    self.ty(&r.value);
                    for t in r.err.iter().chain(&r.env) {
                        self.ty(t);
                    }
                }
                self.block(&f.body);
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                for c in clauses {
                    for p in &c.patterns {
                        self.pattern(p);
                    }
                    if let Some(e) = &c.guard {
                        self.expr(e);
                    }
                    self.expr(&c.body);
                }
            }
            ExprKind::Tuple(es) => {
                for e in es {
                    self.expr(e);
                }
            }
            ExprKind::List { items, rest } => {
                for e in items.iter().chain(rest.as_deref()) {
                    self.expr(e);
                }
            }
            ExprKind::BitArray(ss) => {
                for s in ss {
                    self.expr(&s.value);
                    if let Some(e) = s.format.as_ref().and_then(|f| f.arg.as_deref()) {
                        self.expr(e);
                    }
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                for a in args {
                    self.expr(&a.value);
                }
            }
            ExprKind::Field { target, .. }
            | ExprKind::TupleIndex { target, .. }
            | ExprKind::Unary { expr: target, .. } => self.expr(target),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::RecordUpdate { base, fields, .. } => {
                self.expr(base);
                for (_, e) in fields {
                    self.expr(e);
                }
            }
            _ => {}
        }
    }
}
fn snapshot(
    p: &Program,
    module: ModuleId,
    plan: &migrate::Plan,
    original: &ast::Module,
) -> (HashMap<Span, Span>, HashMap<Span, String>) {
    let mut s = Snapshot::default();
    let m = p.module(module);
    for t in &m.types {
        let t = p.ty(*t);
        for (_, v) in &t.params {
            s.owners.insert(*v, t.span);
        }
        match &t.body {
            TypeBody::Alias(t) => s.ty(t),
            TypeBody::Custom(cs) => {
                for c in cs {
                    for f in &c.fields {
                        s.ty(&f.ty);
                    }
                }
            }
        }
    }
    for d in &m.defs {
        if let DefKind::External { ty, ty_vars, .. } = &d.kind {
            for v in ty_vars {
                s.owners.insert(*v, d.span);
            }
            s.ty(ty);
        }
    }
    for b in &m.bindings {
        s.binding(&b.binding);
    }
    // A direct tuple split gives each part a new written owner, but its
    // variables still originate in the old tuple binding (R4).
    let _ = original;
    let normalize = |span: Span| {
        if !plan.params.keys().any(|p| p.start == span.start) {
            for split in &plan.splits {
                if split.start < span.start && span.end <= split.end {
                    return *split;
                }
            }
        }
        span
    };
    let types = s
        .types
        .iter()
        .filter_map(|(span, v)| s.owners.get(v).map(|owner| (*span, normalize(*owner))))
        .collect();
    let uses = s
        .uses
        .iter()
        .map(|(span, value)| {
            let target = match value {
                ValueRef::Local(v) => format!("local:{:?}", s.locals.get(v)),
                ValueRef::Def(d) => format!("{}:{:?}", p.module(d.module).path, p.def(*d).span),
                ValueRef::Ctor(c) => format!(
                    "ctor:{}:{}",
                    p.module(p.ty(c.ty).module).path,
                    p.ty(c.ty).name
                ),
            };
            (*span, target)
        })
        .collect();
    (types, uses)
}

#[test]
fn unqualified_types_equal_the_actual_builtin_and_prelude_sets() {
    let mut expected: HashSet<_> = BuiltinType::ALL
        .iter()
        .map(|t| t.name().to_string())
        .collect();
    let module = ilang_syntax::parse_module(include_str!("../../../lib/prelude.i")).0;
    expected.extend(module.items.into_iter().filter_map(|i| {
        if let ast::ItemKind::TypeDef(t) = i.kind {
            Some(t.name.name)
        } else {
            None
        }
    }));
    assert_eq!(
        expected,
        migrate::UNQUALIFIED_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect()
    );
}

#[test]
fn formatting_new_matches_keeps_the_subjects_expected_type() {
    // Spec §4.8 gives a one-clause subject the pattern's expected type.
    // R2c and R5 are migration rules for old cases, whose subjects had no
    // such expected type. Applying them to this new chain would lose it.
    let source = "User { User(name: String) }\nid = (x) { x }\nh = () { (u) -> String { u.name } |> { f: (User) -> String -> f(User(\"x\")) } |> id }";
    let (g, _, d) = common::check_files(&[("src/main.i", source)]);
    assert!(d.is_empty(), "{:?}", g.sources.render_all(&d));
    let out = ilang_syntax::format::format(source).unwrap();
    assert!(!out.contains("g ="), "{out}");
    let (g, _, d) = common::check_files(&[("src/main.i", &out)]);
    assert!(d.is_empty(), "{:?}", g.sources.render_all(&d));
}

#[test]
fn r4_preserves_resolved_owners_and_name_targets() {
    let (graph, _, _) = common::check_files(&[("src/main.i", "x = 1")]);
    let (mut parsed, _) = ilang_check::parse(&graph);
    let module = parsed.iter().position(|p| p.path == "app/main").unwrap();
    let mut tested = 0;
    for (path, source) in corpus::corpus()
        .into_iter()
        .chain(corpus::baseline_corpus())
    {
        let (old, errors) = ilang_syntax::parse_module_old(&source);
        let old = if errors.is_empty() {
            old
        } else if path.contains("tests/migrate/mixed_") {
            ilang_syntax::parse_module(&source).0
        } else {
            continue;
        };
        let plan = migrate::plan(&old, &Env::new(&old));
        if plan.refused.is_some() {
            continue;
        }
        parsed[module].ast = old.clone();
        let (before, _) = ilang_check::resolve::resolve(&graph, &parsed);
        parsed[module].ast = migrate::apply(&plan, old.clone());
        let (after, _) = ilang_check::resolve::resolve(&graph, &parsed);
        let a = snapshot(&before, ModuleId(module as u32), &plan, &old);
        let b = snapshot(&after, ModuleId(module as u32), &plan, &old);
        assert_eq!(a.0, b.0, "type variable owner changed: {path}\n{source}");
        for (span, target) in a.1 {
            assert_eq!(
                b.1.get(&span),
                Some(&target),
                "name target changed: {path} at {span:?}\n{source}"
            );
        }
        tested += 1;
    }
    assert!(tested > 250, "only {tested} sources checked");
}

#[test]
fn one_clause_report_checks_the_formatted_binding_positions() {
    let Ok(path) = std::env::var("ONE_CLAUSE_REPORT") else {
        return;
    };
    assert!(!std::hint::black_box(ilang_check::ONE_CLAUSE_AS_CASE));
    let report = std::fs::read_to_string(&path).unwrap();
    let mut output = Vec::new();
    let mut count = 0;
    for line in report.lines() {
        let fields: Vec<_> = line.splitn(5, ' ').collect();
        if fields.first() != Some(&"one-clause") {
            output.push(line.to_string());
            continue;
        }
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(fields[2].rsplit_once(':').unwrap().0);
        let position = fields[4]
            .split_whitespace()
            .find_map(|s| s.strip_prefix("out="))
            .expect("output position");
        let (line_number, column) = position.split_once(':').unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("i.toml"), "name = \"spec\"\n").unwrap();
        std::fs::create_dir(dir.path().join("test")).unwrap();
        let name = file.file_name().unwrap();
        std::fs::copy(&file, dir.path().join("test").join(name)).unwrap();
        let (graph, d) = ilang_package::load(dir.path());
        assert!(d.is_empty());
        let checked = ilang_check::check(&graph);
        assert!(
            !checked.diags.iter().any(|d| d.is_error()),
            "{:?}",
            graph.sources.render_all(&checked.diags)
        );
        let p = checked.program.as_ref().unwrap();
        let module = p
            .modules
            .iter()
            .position(|m| Some(m.package) == graph.root)
            .unwrap();
        let text = &graph.sources.get(p.modules[module].source).text;
        let lines = ilang_syntax::span::LineIndex::new(text);
        let wanted = (
            line_number.parse::<u32>().unwrap(),
            column.parse::<u32>().unwrap(),
        );
        let start = text
            .char_indices()
            .map(|(i, _)| i as u32)
            .find(|i| lines.line_col(*i) == wanted)
            .expect("position inside the output source");
        let typed = checked.types.as_ref().unwrap();
        let found = typed
            .may_fail
            .iter()
            .find(|((m, span), _)| *m == ModuleId(module as u32) && span.start == start)
            .map(|(_, fail)| *fail);
        let mut snapshot = Snapshot::default();
        for b in &p.modules[module].bindings {
            snapshot.binding(&b.binding);
        }
        assert!(
            found.is_some() || snapshot.patterns.iter().any(|span| span.start == start),
            "report did not find a binding at {position}"
        );
        let flag = if found == Some(true) { "may-fail " } else { "" };
        output.push(format!(
            "{} {} {} {} {flag}{}",
            fields[0], fields[1], fields[2], fields[3], fields[4]
        ));
        count += 1;
    }
    assert!(count > 0, "no one-clause entries were checked");
    std::fs::write(path, output.join("\n") + "\n").unwrap();
}
