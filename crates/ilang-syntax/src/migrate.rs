//! The temporary migration from the old surface to the new one (A1 R1-R12).
//! Planning uses only syntax and the names of unqualified types. Rendering,
//! byte splicing, and the independent tree transform share this plan.
use crate::ast::*;
use crate::span::Span;
use std::collections::{HashMap, HashSet};

pub const UNQUALIFIED_TYPES: &[&str] = &[
    "Int",
    "Float",
    "String",
    "BitArray",
    "List",
    "Never",
    "Boundary",
    "Continuation",
    "Nil",
    "Bool",
    "Result",
    "UtfCodepoint",
    "Assert",
    "AssertionFailure",
];

#[derive(Clone, Debug)]
pub struct Env {
    pub types: HashSet<String>,
    pub module: String,
    pub choices_split: bool,
}
impl Env {
    pub fn new(module: &Module) -> Self {
        let mut types: HashSet<_> = UNQUALIFIED_TYPES.iter().map(|s| s.to_string()).collect();
        for item in &module.items {
            if let ItemKind::TypeDef(def) = &item.kind {
                types.insert(def.name.name.clone());
            }
        }
        for import in &module.imports {
            types.extend(
                import
                    .names
                    .iter()
                    .filter(|n| n.name.starts_with(char::is_uppercase))
                    .map(|n| n.name.clone()),
            );
        }
        Self {
            types,
            module: "module".into(),
            choices_split: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub reason: &'static str,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub category: &'static str,
    pub owner: String,
    pub span: Span,
    pub reason: String,
    pub pattern: Option<Span>,
}
#[derive(Clone, Debug)]
pub struct Extraction {
    pub name: String,
    pub function: Expr,
}
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub module: String,
    pub renames: HashMap<Span, String>,
    pub params: HashMap<Span, Vec<String>>,
    pub groups: HashSet<Span>,
    pub type_groups: HashSet<Span>,
    pub extractions: HashMap<Span, Extraction>,
    pub replacements: HashMap<Span, String>,
    pub splits: HashSet<Span>,
    pub choices: HashSet<Span>,
    pub entries: Vec<Entry>,
    pub refused: Option<Refused>,
    pub old_cases: HashSet<Span>,
    pub drop_imports: bool,
    pub drop_exports: bool,
    pub headers: bool,
}
impl Plan {
    pub fn is_empty(&self) -> bool {
        self.renames.is_empty()
            && self.params.is_empty()
            && self.groups.is_empty()
            && self.type_groups.is_empty()
            && self.extractions.is_empty()
            && self.splits.is_empty()
            && self.choices.is_empty()
            && self.old_cases.is_empty()
            && !self.drop_imports
            && !self.drop_exports
            && !self.headers
    }
}

// One complete walk, shared by planning, applying, and the syntax-only
// capture checks. Hooks run before and after the original children.
pub(crate) trait Visit {
    fn descend_binding(&mut self, _: &Binding) -> bool {
        true
    }
    fn binding(&mut self, _: &mut Binding, _: bool) {}
    fn definition(&mut self, _: &mut TypeDef, _: bool) {}
    fn declaration(&mut self, _: &Ident, _: &mut Vec<Ident>, _: &mut TypeExpr, _: bool) {}
    fn expr(&mut self, _: &mut Expr, _: bool) {}
    fn ty(&mut self, _: &mut TypeExpr) {}
    fn block(&mut self, _: &mut Block) {}
    fn pattern(&mut self, _: &mut Pattern) {}
}
pub(crate) fn visit_module(m: &mut Module, v: &mut impl Visit) {
    for i in &mut m.items {
        for a in &mut i.attributes {
            if let Some((args, _)) = &mut a.call {
                for a in args {
                    visit_expr(&mut a.value, v);
                }
            }
        }
        match &mut i.kind {
            ItemKind::Binding(b) => visit_binding(b, v),
            ItemKind::Declaration {
                name,
                type_params,
                ty,
            } => {
                v.declaration(name, type_params, ty, true);
                visit_type(ty, v);
                v.declaration(name, type_params, ty, false);
            }
            ItemKind::TypeDef(d) => {
                v.definition(d, true);
                for t in &mut d.head {
                    visit_type(t, v);
                }
                match &mut d.body {
                    TypeBody::Alias(t) => visit_type(t, v),
                    TypeBody::Custom(cs) => {
                        for c in cs {
                            for f in &mut c.fields {
                                visit_type(&mut f.ty, v);
                            }
                        }
                    }
                }
                v.definition(d, false);
            }
            ItemKind::MacroItem { args, .. } => {
                for a in args {
                    visit_expr(&mut a.value, v);
                }
            }
        }
    }
}
fn visit_binding(b: &mut Binding, v: &mut impl Visit) {
    if !v.descend_binding(b) {
        return;
    }
    v.binding(b, true);
    visit_pattern(&mut b.pattern, v);
    visit_expr(&mut b.value, v);
    v.binding(b, false);
}
fn visit_pattern(p: &mut Pattern, v: &mut impl Visit) {
    match &mut p.kind {
        PatternKind::Typed { ty, .. } => visit_type(ty, v),
        PatternKind::Named { pattern, .. } => visit_pattern(pattern, v),
        PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
            for p in args {
                visit_pattern(p, v);
            }
        }
        PatternKind::List { items, .. } => {
            for p in items {
                visit_pattern(p, v);
            }
        }
        PatternKind::BitArray(ss) => {
            for s in ss {
                visit_pattern(&mut s.value, v);
                if let Some(a) = s.format.as_mut().and_then(|f| f.arg.as_deref_mut()) {
                    visit_expr(a, v);
                }
            }
        }
        _ => {}
    }
    v.pattern(p);
}
fn visit_type(t: &mut TypeExpr, v: &mut impl Visit) {
    match &mut t.kind {
        TypeKind::Named { args, .. }
        | TypeKind::Tuple(args)
        | TypeKind::Union(args)
        | TypeKind::Inter(args)
        | TypeKind::Diff(args) => {
            for t in args {
                visit_type(t, v);
            }
        }
        TypeKind::Call { args, .. } => {
            for a in args {
                if let TypeArg::Type(t) = a {
                    visit_type(t, v);
                }
            }
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            for p in params {
                visit_type(&mut p.ty, v);
            }
            visit_type(ret, v);
            for t in err.iter_mut().chain(env) {
                visit_type(t, v);
            }
        }
        _ => {}
    }
    v.ty(t);
}
fn visit_block(b: &mut Block, v: &mut impl Visit) {
    for s in &mut b.stmts {
        match s {
            Stmt::Bind(b) => visit_binding(b, v),
            Stmt::Env { ty, value, .. } => {
                visit_type(ty, v);
                visit_expr(value, v);
            }
            Stmt::Expr(e) => visit_expr(e, v),
        }
    }
    v.block(b);
}
pub(crate) fn visit_expr(e: &mut Expr, v: &mut impl Visit) {
    v.expr(e, true);
    match &mut e.kind {
        ExprKind::Fn(f) => {
            for p in &mut f.params {
                match p {
                    Param::Pattern(p) => visit_pattern(p, v),
                    Param::Env { ty, .. } => visit_type(ty, v),
                }
            }
            if let Some(r) = &mut f.ret {
                visit_type(&mut r.value, v);
                for t in r.err.iter_mut().chain(&mut r.env) {
                    visit_type(t, v);
                }
            }
            visit_block(&mut f.body, v);
        }
        ExprKind::Block(b) => visit_block(b, v),
        ExprKind::Case {
            subject, clauses, ..
        } => {
            visit_expr(subject, v);
            visit_clauses(clauses, v);
        }
        ExprKind::Clauses(cs) => visit_clauses(cs, v),
        ExprKind::Tuple(es) => {
            for e in es {
                visit_expr(e, v);
            }
        }
        ExprKind::List { items, rest } => {
            for e in items.iter_mut().chain(rest.as_deref_mut()) {
                visit_expr(e, v);
            }
        }
        ExprKind::BitArray(ss) => {
            for s in ss {
                visit_expr(&mut s.value, v);
                if let Some(a) = s.format.as_mut().and_then(|f| f.arg.as_deref_mut()) {
                    visit_expr(a, v);
                }
            }
        }
        ExprKind::Call { callee, args } => {
            visit_expr(callee, v);
            for a in args {
                visit_expr(&mut a.value, v);
            }
        }
        ExprKind::Field { target, .. }
        | ExprKind::TupleIndex { target, .. }
        | ExprKind::Unary { expr: target, .. } => visit_expr(target, v),
        ExprKind::Binary { lhs, rhs, .. } => {
            visit_expr(lhs, v);
            visit_expr(rhs, v);
        }
        ExprKind::RecordUpdate {
            constructor,
            base,
            fields,
        } => {
            visit_expr(constructor, v);
            visit_expr(base, v);
            for (_, e) in fields {
                visit_expr(e, v);
            }
        }
        _ => {}
    }
    v.expr(e, false);
}
fn visit_clauses(cs: &mut [Clause], v: &mut impl Visit) {
    for c in cs {
        for p in &mut c.patterns {
            visit_pattern(p, v);
        }
        for e in c.guard.iter_mut() {
            visit_expr(e, v);
        }
        visit_expr(&mut c.body, v);
    }
}
#[derive(Default)]
struct Lower(Vec<String>);
impl Visit for Lower {
    // Resolve's block_annotations deliberately leaves nested bindings to
    // their own frames. They are not annotations of the enclosing binding.
    fn descend_binding(&mut self, _: &Binding) -> bool {
        false
    }
    fn ty(&mut self, t: &mut TypeExpr) {
        if let TypeKind::Var(n) = &t.kind
            && !self.0.contains(n)
        {
            self.0.push(n.clone());
        }
    }
}
#[derive(Clone)]
struct Owner {
    span: Span,
    vars: Vec<usize>,
    used: HashSet<usize>,
    outer: HashSet<String>,
    captured: HashSet<String>,
    binding: Option<Binding>,
    declaration: bool,
    definition: Option<TypeDef>,
}
struct Variable {
    old: String,
    owner: usize,
    uses: Vec<Span>,
    unowned: bool,
}
struct Frame {
    owner: usize,
    names: HashMap<String, usize>,
    declared: HashSet<String>,
}
struct Scan<'a> {
    names: HashSet<String>,
    env: &'a Env,
    plan: Plan,
    owners: Vec<Owner>,
    vars: Vec<Variable>,
    frames: Vec<Frame>,
    def: Option<usize>,
    unowned: HashMap<(usize, String), usize>,
    top_owner: String,
}
fn binding_name(p: &Pattern) -> Option<&str> {
    match &p.kind {
        PatternKind::Var(n) => Some(n),
        PatternKind::Typed { binder, .. } if !binder.discard => Some(&binder.name),
        _ => None,
    }
}
fn within(a: Span, b: Span) -> bool {
    a.start <= b.start && b.end <= a.end
}
fn renamed(n: &str) -> String {
    let mut upper = true;
    let mut out = String::new();
    for c in n.chars() {
        if c == '_' {
            upper = true;
        } else {
            out.push(if upper { c.to_ascii_uppercase() } else { c });
            upper = false;
        }
    }
    out
}
impl Scan<'_> {
    fn owner(
        &mut self,
        span: Span,
        lower: Vec<String>,
        params: &[Ident],
        binding: Option<Binding>,
        declaration: bool,
        definition: Option<TypeDef>,
    ) {
        let id = self.owners.len();
        let outer = self
            .frames
            .iter()
            .flat_map(|f| &f.declared)
            .cloned()
            .collect();
        self.owners.push(Owner {
            span,
            vars: vec![],
            used: HashSet::new(),
            outer,
            captured: HashSet::new(),
            binding,
            declaration,
            definition,
        });
        let mut names = HashMap::new();
        if params.is_empty() {
            for n in lower {
                if !names.contains_key(&n) && !self.frames.iter().any(|f| f.names.contains_key(&n))
                {
                    let var = self.vars.len();
                    self.vars.push(Variable {
                        old: n.clone(),
                        owner: id,
                        uses: vec![],
                        unowned: false,
                    });
                    self.owners[id].vars.push(var);
                    names.insert(n, var);
                }
            }
        }
        self.frames.push(Frame {
            owner: id,
            names,
            declared: params.iter().map(|n| n.name.clone()).collect(),
        });
    }
    fn entry(
        &mut self,
        category: &'static str,
        span: Span,
        reason: impl Into<String>,
        pattern: Option<Span>,
    ) {
        self.plan.entries.push(Entry {
            category,
            owner: self.top_owner.clone(),
            span,
            reason: reason.into(),
            pattern,
        });
    }
    fn refuse(&mut self, reason: &'static str, span: Span) {
        if self.plan.refused.is_none() {
            self.plan.refused = Some(Refused { reason, span });
        }
        self.entry("hand", span, reason, None);
    }
    fn group(&mut self, e: &Expr, report: bool) {
        if !matches!(e.kind, ExprKind::Block(_)) {
            self.plan.groups.insert(e.span);
            if report {
                self.entry("group", e.span, "match-left-operand", None);
            }
        }
    }
    fn extraction(&mut self, container: &Expr, f: &Expr, safe: bool) {
        let ExprKind::Fn(function) = &f.kind else {
            return;
        };
        if function.ret.is_none()
            || !function
                .params
                .iter()
                .all(|p| matches!(p, Param::Pattern(_)))
        {
            return;
        }
        if !safe {
            self.refuse("fn-expected-type", f.span);
            return;
        }
        let names = &mut self.names;
        let mut name = "g".to_string();
        let mut i = 2;
        while names.contains(&name) {
            name = format!("g{i}");
            i += 1;
        }
        names.insert(name.clone());
        self.plan.extractions.insert(
            container.span,
            Extraction {
                name: name.clone(),
                function: f.clone(),
            },
        );
        self.plan.replacements.insert(f.span, name);
        self.entry("g-binding", f.span, "function-at-brace-start", None);
    }
}
impl Visit for Scan<'_> {
    fn binding(&mut self, b: &mut Binding, enter: bool) {
        if !enter {
            self.frames.pop();
            return;
        }
        if self.frames.is_empty() {
            self.top_owner = if matches!(
                ungroup(&b.value).kind,
                ExprKind::Fn(_) | ExprKind::Clauses(_)
            ) {
                binding_name(&b.pattern).map_or_else(
                    || "ilang_start".into(),
                    |n| format!("{}.{n}", self.env.module),
                )
            } else {
                "ilang_start".into()
            };
        }
        let mut lower = Lower::default();
        visit_pattern(&mut b.pattern.clone(), &mut lower);
        visit_expr(&mut b.value.clone(), &mut lower);
        self.owner(
            b.span,
            lower.0,
            &b.type_params,
            Some(b.clone()),
            false,
            None,
        );
    }
    fn declaration(
        &mut self,
        name: &Ident,
        params: &mut Vec<Ident>,
        ty: &mut TypeExpr,
        enter: bool,
    ) {
        if enter {
            let mut lower = Lower::default();
            visit_type(&mut ty.clone(), &mut lower);
            self.owner(name.span, lower.0, params, None, true, None);
        } else {
            self.frames.pop();
        }
    }
    fn definition(&mut self, d: &mut TypeDef, enter: bool) {
        if !enter {
            self.frames.pop();
            self.def = None;
            return;
        }
        // A mixed head is already reserved, and capitalizing its lower
        // parts could turn that error into a valid parameter list (§4.3).
        if !d.head.is_empty() && d.head.iter().any(|t| matches!(t.kind, TypeKind::Var(_))) {
            self.refuse("tyvar-owner", d.name.span);
        }
        let lower: Vec<_> = d
            .params
            .iter()
            .filter(|p| {
                p.name
                    .starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
            })
            .map(|p| p.name.clone())
            .collect();
        let mut defs = d.clone();
        let params = std::mem::take(&mut defs.params);
        self.owner(d.name.span, lower, &[], None, false, Some(d.clone()));
        let id = self.owners.len() - 1;
        self.def = Some(id);
        self.frames.last_mut().unwrap().declared.extend(
            params
                .iter()
                .filter(|p| p.name.starts_with(char::is_uppercase))
                .map(|p| p.name.clone()),
        );
    }
    fn ty(&mut self, t: &mut TypeExpr) {
        let TypeKind::Var(n) = &t.kind else { return };
        let found = self
            .frames
            .iter()
            .rev()
            .find_map(|f| f.names.get(n).copied());
        let var = match found {
            Some(v) => v,
            None => {
                let owner = self
                    .def
                    .unwrap_or_else(|| self.frames.last().map_or(0, |f| f.owner));
                if self.owners.is_empty() {
                    self.owner(t.span, vec![], &[], None, false, None);
                }
                let key = (owner, n.clone());
                if let Some(&v) = self.unowned.get(&key) {
                    v
                } else {
                    let v = self.vars.len();
                    self.vars.push(Variable {
                        old: n.clone(),
                        owner,
                        uses: vec![],
                        unowned: true,
                    });
                    self.owners[owner].vars.push(v);
                    self.unowned.insert(key, v);
                    v
                }
            }
        };
        self.vars[var].uses.push(t.span);
        for f in &self.frames {
            self.owners[f.owner].used.insert(var);
        }
        let owner = self.vars[var].owner;
        for f in &self.frames {
            if f.owner > owner {
                self.owners[owner]
                    .captured
                    .extend(f.declared.iter().cloned());
            }
        }
    }
    fn expr(&mut self, e: &mut Expr, enter: bool) {
        if !enter {
            return;
        }
        match &e.kind {
            ExprKind::Binary { lhs, rhs, op } => {
                if is_case(lhs) {
                    self.group(lhs, op.level() <= 6);
                }
                if op.level() >= 7 && is_case(rhs) {
                    self.group(rhs, false);
                }
            }
            ExprKind::Call { callee: target, .. }
            | ExprKind::Field { target, .. }
            | ExprKind::TupleIndex { target, .. }
            | ExprKind::Unary { expr: target, .. } => {
                if is_case(target) {
                    self.group(target, false);
                }
            }
            ExprKind::Block(b) => {
                if let Some(Stmt::Expr(first)) = b.stmts.first()
                    && let Some((f, safe)) = leading(first)
                {
                    self.extraction(e, f, safe || b.stmts.len() > 1);
                }
            }
            _ => {}
        }
        if let ExprKind::Case {
            subject,
            clauses,
            syntax,
        } = &e.kind
        {
            if crate::print::level(subject) < 6 {
                self.group(subject, false);
            }
            if *syntax == Syntax::Old {
                self.plan.old_cases.insert(e.span);
                if clauses.len() == 1 && clauses[0].guard.is_none() {
                    let c = &clauses[0];
                    if c.patterns.len() > 1 && self.env.choices_split {
                        self.plan.choices.insert(e.span);
                        self.entry(
                            "choices-split",
                            e.span,
                            "one clause per choice",
                            Some(c.patterns[0].span),
                        );
                    } else if c.patterns.len() == 1 {
                        if known_constructor_use(subject, &c.patterns[0], &c.body) {
                            self.refuse("one-clause-known", e.span);
                        } else {
                            self.entry(
                                "one-clause",
                                e.span,
                                "fitted binding",
                                Some(c.patterns[0].span),
                            );
                        }
                    }
                }
            }
        }
        if let ExprKind::Case { clauses, .. } | ExprKind::Clauses(clauses) = &e.kind {
            for c in clauses {
                if matches!(
                    e.kind,
                    ExprKind::Case {
                        syntax: Syntax::Old,
                        ..
                    }
                ) && c.guard.is_some()
                {
                    for p in &c.patterns {
                        if let Some(t) = trailing_function(p) {
                            self.plan.type_groups.insert(t);
                        }
                    }
                }
            }
            if crate::print::first_clause_needs_group(
                clauses,
                matches!(e.kind, ExprKind::Clauses(_)),
            ) && let Some((f, safe)) = leading(&clauses[0].body)
            {
                let ExprKind::Fn(function) = &f.kind else {
                    unreachable!()
                };
                self.group(&clauses[0].body, false);
                if function.ret.is_some() {
                    self.extraction(&clauses[0].body, f, safe);
                } else {
                    self.entry("group", clauses[0].body.span, "first-clause-function", None);
                }
            }
        }
        if self.plan.groups.contains(&e.span)
            && let Some((f, safe)) = leading(e)
        {
            self.extraction(e, f, safe);
        }
    }
}
fn is_case(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::Case {
            syntax: Syntax::Old,
            ..
        }
    )
}
fn ungroup(e: &Expr) -> &Expr {
    if let ExprKind::Block(b) = &e.kind
        && let [Stmt::Expr(inner)] = &b.stmts[..]
    {
        return ungroup(inner);
    }
    e
}
fn leading(e: &Expr) -> Option<(&Expr, bool)> {
    let (child, safe) = match &e.kind {
        ExprKind::Fn(_) => return Some((e, false)),
        ExprKind::Call { callee, .. } => (callee.as_ref(), true),
        ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
            (target.as_ref(), true)
        }
        ExprKind::Case { subject, .. } => (subject.as_ref(), true),
        ExprKind::Binary { lhs, op, .. } => (lhs.as_ref(), *op != BinOp::Pipe),
        _ => return None,
    };
    if matches!(child.kind, ExprKind::Fn(_)) {
        Some((child, safe))
    } else {
        leading(child)
    }
}
fn expression_names(e: &Expr) -> HashSet<String> {
    #[derive(Default)]
    struct Names(HashSet<String>);
    impl Visit for Names {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if enter && let ExprKind::Name { name, .. } = &e.kind {
                self.0.insert(name.name.clone());
            }
        }
        fn binding(&mut self, b: &mut Binding, enter: bool) {
            if enter {
                pattern_names(&b.pattern, &mut self.0);
            }
        }
    }
    let mut names = Names::default();
    visit_expr(&mut e.clone(), &mut names);
    names.0
}
fn pattern_names(p: &Pattern, out: &mut HashSet<String>) {
    match &p.kind {
        PatternKind::Var(n) => {
            out.insert(n.clone());
        }
        PatternKind::Typed { binder, .. } => {
            if !binder.discard {
                out.insert(binder.name.clone());
            }
        }
        PatternKind::Named { name, pattern } => {
            out.insert(name.name.clone());
            pattern_names(pattern, out);
        }
        PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
            for p in args {
                pattern_names(p, out);
            }
        }
        PatternKind::List { items, rest } => {
            for p in items {
                pattern_names(p, out);
            }
            if let Some(Some(b)) = rest {
                out.insert(b.name.clone());
            }
        }
        PatternKind::Prefix { rest, .. } => {
            if !rest.discard {
                out.insert(rest.name.clone());
            }
        }
        PatternKind::BitArray(ss) => {
            for s in ss {
                pattern_names(&s.value, out);
            }
        }
        _ => {}
    }
}
fn known_constructor_use(s: &Expr, p: &Pattern, b: &Expr) -> bool {
    let ExprKind::Name { module: None, name } = &s.kind else {
        return false;
    };
    let mut bound = HashSet::new();
    pattern_names(p, &mut bound);
    if bound.contains(&name.name) || !expression_names(b).contains(&name.name) {
        return false;
    }
    let mut p = p;
    while let PatternKind::Named { pattern, .. } = &p.kind {
        p = pattern;
    }
    matches!(p.kind, PatternKind::Constructor { .. })
}
fn trailing_function(p: &Pattern) -> Option<Span> {
    let t = match &p.kind {
        PatternKind::Typed { ty, .. } => ty,
        PatternKind::Named { pattern, .. } => return trailing_function(pattern),
        _ => return None,
    };
    fn tail(t: &TypeExpr) -> Option<Span> {
        match &t.kind {
            TypeKind::Fn { .. } => Some(t.span),
            TypeKind::Union(ts) | TypeKind::Inter(ts) | TypeKind::Diff(ts) => {
                ts.last().and_then(tail)
            }
            _ => None,
        }
    }
    tail(t)
}
pub fn is_direct(e: &Expr) -> bool {
    match &ungroup(e).kind {
        ExprKind::Fn(_)
        | ExprKind::Clauses(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name { .. } => true,
        ExprKind::Unary { expr, .. } => matches!(
            expr.kind,
            ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_)
        ),
        ExprKind::Tuple(es) => es.iter().all(is_direct),
        ExprKind::List { items, rest } => items.iter().chain(rest.as_deref()).all(is_direct),
        ExprKind::Call { callee, args } => {
            constructor(callee) && args.iter().all(|a| is_direct(&a.value))
        }
        ExprKind::Binary {
            lhs,
            rhs,
            op: BinOp::Pipe,
        } => {
            is_direct(lhs)
                && (constructor(rhs)
                    || matches!(&rhs.kind, ExprKind::Call { callee, args } if constructor(callee) && args.iter().all(|a| is_direct(&a.value))))
        }
        _ => false,
    }
}
fn constructor(e: &Expr) -> bool {
    matches!(&e.kind, ExprKind::Name { name, .. } if name.name.starts_with(char::is_uppercase))
}

pub fn plan(module: &Module, env: &Env) -> Plan {
    use crate::walk::{Visitor, Walk};
    struct Names(HashSet<String>);
    impl Visitor for Names {
        fn string(&mut self, name: &mut String) {
            self.0.insert(name.clone());
        }
    }
    // A module-wide superset is conservative for every insertion point. It
    // includes qualifiers and binders in patterns, types and nested scopes.
    let mut names = Names(HashSet::new());
    module.clone().walk(&mut names);
    let mut scan = Scan {
        names: names.0,
        env,
        plan: Plan::default(),
        owners: vec![],
        vars: vec![],
        frames: vec![],
        def: None,
        unowned: HashMap::new(),
        top_owner: "ilang_start".into(),
    };
    visit_module(&mut module.clone(), &mut scan);
    let mut new_names: HashMap<usize, String> = HashMap::new();
    for id in 0..scan.owners.len() {
        let o = scan.owners[id].clone();
        let mut forbidden = env.types.clone();
        forbidden.extend(o.outer.iter().cloned());
        forbidden.extend(o.captured.iter().cloned());
        for var in &o.used {
            if scan.vars[*var].owner != id
                && let Some(n) = new_names.get(var)
            {
                forbidden.insert(n.clone());
            }
        }
        if let Some(d) = &o.definition {
            struct Upper(HashSet<String>);
            impl Visit for Upper {
                fn ty(&mut self, t: &mut TypeExpr) {
                    if let TypeKind::Named { name, .. } = &t.kind {
                        self.0.insert(name.name.clone());
                    }
                }
            }
            let mut upper = Upper(HashSet::new());
            let mut m = Module::default();
            m.items.push(Item {
                doc: vec![],
                attributes: vec![],
                kind: ItemKind::TypeDef(d.clone()),
                span: o.span,
            });
            visit_module(&mut m, &mut upper);
            forbidden.extend(upper.0);
        }
        for &v in &o.vars {
            let variable = &scan.vars[v];
            let mut name = renamed(&variable.old);
            let mut n = 1;
            let fallback = forbidden.contains(&name);
            if fallback {
                while forbidden.contains(&format!("A{n}")) {
                    n += 1;
                }
                name = format!("A{n}");
            }
            forbidden.insert(name.clone());
            new_names.insert(v, name.clone());
            for &span in &variable.uses {
                scan.plan.renames.insert(span, name.clone());
            }
            if fallback || variable.unowned {
                scan.plan.entries.push(Entry {
                    category: if variable.unowned {
                        "unowned"
                    } else {
                        "rename"
                    },
                    owner: owner_name(&o, env),
                    span: variable.uses.first().copied().unwrap_or(o.span),
                    reason: format!("{} -> {name}", variable.old),
                    pattern: None,
                });
            }
            if variable.unowned {
                scan.plan.entries.push(Entry {
                    category: "hand",
                    owner: owner_name(&o, env),
                    span: o.span,
                    reason: "tyvar-unowned".into(),
                    pattern: None,
                });
            }
        }
        let declared: Vec<_> = o
            .vars
            .iter()
            .filter(|v| !scan.vars[**v].unowned)
            .map(|v| new_names[v].clone())
            .collect();
        if let Some(d) = &o.definition {
            for param in &d.params {
                if let Some(v) = o.vars.iter().find(|v| scan.vars[**v].old == param.name) {
                    scan.plan.renames.insert(param.span, new_names[v].clone());
                }
            }
        } else if !declared.is_empty() {
            if o.declaration
                || o.binding
                    .as_ref()
                    .is_some_and(|b| binding_name(&b.pattern).is_some())
            {
                scan.plan.params.insert(o.span, declared);
            } else if let Some(b) = &o.binding {
                if split_safe(b, &o.vars, &scan.vars) {
                    scan.plan.splits.insert(b.span);
                    scan.plan.params.insert(b.span, declared);
                    scan.plan.entries.push(Entry {
                        category: "split",
                        owner: owner_name(&o, env),
                        span: b.span,
                        reason: if scan.owners[..id].iter().any(|x| {
                            x.binding
                                .as_ref()
                                .is_some_and(|outer| within(outer.value.span, b.span))
                        }) {
                            "direct tuple"
                        } else {
                            "whole direct top-level tuple"
                        }
                        .into(),
                        pattern: None,
                    });
                } else {
                    scan.top_owner = owner_name(&o, env);
                    scan.refuse("tyvar-owner", b.span);
                }
            }
        }
    }
    scan.plan.headers = module.imports_span.is_some()
        && module.imports_style == HeaderStyle::Keyword
        || module.exports_span.is_some() && module.exports_style == HeaderStyle::Keyword;
    scan.plan.module = env.module.clone();
    scan.plan.drop_imports = module.imports_span.is_some() && module.imports.is_empty();
    scan.plan.drop_exports = module.exports_span.is_some() && module.exports.is_empty();
    scan.plan
}
fn owner_name(o: &Owner, env: &Env) -> String {
    o.binding
        .as_ref()
        .and_then(|b| binding_name(&b.pattern))
        .map_or_else(|| "ilang_start".into(), |n| format!("{}.{n}", env.module))
}
fn split_safe(b: &Binding, vars: &[usize], variables: &[Variable]) -> bool {
    let (PatternKind::Tuple(ps), ExprKind::Tuple(es)) = (&b.pattern.kind, &b.value.kind) else {
        return false;
    };
    if ps.len() != es.len() || !is_direct(&b.value) || ps.iter().any(|p| binding_name(p).is_none())
    {
        return false;
    }
    let names: Vec<_> = ps.iter().map(|p| binding_name(p).unwrap()).collect();
    for (i, e) in es.iter().enumerate() {
        if expression_names(e).iter().any(|n| {
            names
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && n == other)
        }) {
            return false;
        }
    }
    vars.iter().all(|v| {
        ps.iter()
            .zip(es)
            .filter(|(p, e)| {
                variables[*v]
                    .uses
                    .iter()
                    .any(|s| within(p.span, *s) || within(e.span, *s))
            })
            .count()
            <= 1
    })
}

pub fn apply(plan: &Plan, mut module: Module) -> Module {
    struct Apply<'a> {
        plan: &'a Plan,
        extracted: HashMap<Span, Expr>,
    }
    impl Visit for Apply<'_> {
        fn ty(&mut self, t: &mut TypeExpr) {
            if let Some(n) = self.plan.renames.get(&t.span)
                && matches!(t.kind, TypeKind::Var(_))
            {
                t.kind = TypeKind::Named {
                    module: None,
                    name: Ident {
                        name: n.clone(),
                        span: t.span,
                    },
                    args: vec![],
                };
            }
        }
        fn binding(&mut self, b: &mut Binding, enter: bool) {
            if !enter
                && !self.plan.splits.contains(&b.span)
                && let Some(params) = self.plan.params.get(&b.span)
            {
                b.type_params = params
                    .iter()
                    .map(|n| Ident {
                        name: n.clone(),
                        span: b.pattern.span,
                    })
                    .collect();
            }
        }
        fn declaration(
            &mut self,
            name: &Ident,
            params: &mut Vec<Ident>,
            _: &mut TypeExpr,
            enter: bool,
        ) {
            if !enter && let Some(names) = self.plan.params.get(&name.span) {
                *params = names
                    .iter()
                    .map(|n| Ident {
                        name: n.clone(),
                        span: name.span,
                    })
                    .collect();
            }
        }
        fn definition(&mut self, d: &mut TypeDef, enter: bool) {
            if !enter {
                for p in &mut d.params {
                    if let Some(n) = self.plan.renames.get(&p.span) {
                        p.name = n.clone();
                    }
                }
            }
        }
        fn block(&mut self, b: &mut Block) {
            b.stmts = std::mem::take(&mut b.stmts)
                .into_iter()
                .flat_map(|s| match s {
                    Stmt::Bind(binding) if self.plan.splits.contains(&binding.span) => {
                        split_binding(self.plan, binding)
                            .into_iter()
                            .map(Stmt::Bind)
                            .collect()
                    }
                    other => vec![other],
                })
                .collect();
        }
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if enter {
                return;
            }
            if let ExprKind::Case {
                syntax, clauses, ..
            } = &mut e.kind
            {
                if self.plan.choices.contains(&e.span) {
                    let c = &clauses[0];
                    *clauses = c
                        .patterns
                        .iter()
                        .map(|p| Clause {
                            patterns: vec![p.clone()],
                            guard: None,
                            body: c.body.clone(),
                            span: c.span,
                        })
                        .collect();
                }
                *syntax = Syntax::New;
            }
            if let Some(name) = self.plan.replacements.get(&e.span) {
                self.extracted.insert(e.span, e.clone());
                e.kind = ExprKind::Name {
                    module: None,
                    name: Ident {
                        name: name.clone(),
                        span: e.span,
                    },
                };
            }
            if self.plan.groups.contains(&e.span) && !matches!(e.kind, ExprKind::Block(_)) {
                let inner = e.clone();
                e.kind = ExprKind::Block(Block {
                    stmts: vec![Stmt::Expr(inner)],
                    span: e.span,
                });
            }
            if let Some(extract) = self.plan.extractions.get(&e.span) {
                let value = self.extracted[&extract.function.span].clone();
                let binding = Stmt::Bind(Binding {
                    pattern: Pattern {
                        kind: PatternKind::Var(extract.name.clone()),
                        span: extract.function.span,
                    },
                    value,
                    span: extract.function.span,
                    type_params: vec![],
                });
                let inner = e.clone();
                e.kind = ExprKind::Block(Block {
                    stmts: vec![binding, Stmt::Expr(inner)],
                    span: e.span,
                });
            }
        }
    }
    let mut apply = Apply {
        plan,
        extracted: HashMap::new(),
    };
    visit_module(&mut module, &mut apply);
    module.items = std::mem::take(&mut module.items)
        .into_iter()
        .flat_map(|i| {
            if let ItemKind::Binding(b) = &i.kind
                && plan.splits.contains(&b.span)
            {
                return split_binding(plan, b.clone())
                    .into_iter()
                    .map(|b| Item {
                        kind: ItemKind::Binding(b),
                        ..i.clone()
                    })
                    .collect();
            }
            vec![i]
        })
        .collect();
    module.imports_style = if module.imports.is_empty() {
        HeaderStyle::Keyword
    } else {
        HeaderStyle::Group
    };
    module.exports_style = if module.exports.is_empty() {
        HeaderStyle::Keyword
    } else {
        HeaderStyle::Group
    };
    if plan.drop_imports {
        module.imports_span = None;
    }
    if plan.drop_exports {
        module.exports_span = None;
    }
    module
}
fn split_binding(plan: &Plan, b: Binding) -> Vec<Binding> {
    let (PatternKind::Tuple(ps), ExprKind::Tuple(es)) = (b.pattern.kind, b.value.kind) else {
        unreachable!()
    };
    ps.into_iter()
        .zip(es)
        .map(|(pattern, value)| {
            let names = type_names_in(&pattern, &value);
            let type_params = plan.params[&b.span]
                .iter()
                .filter(|n| names.contains(*n))
                .map(|n| Ident {
                    name: n.clone(),
                    span: pattern.span,
                })
                .collect();
            Binding {
                span: pattern.span.to(value.span),
                pattern,
                value,
                type_params,
            }
        })
        .collect()
}
fn type_names_in(p: &Pattern, e: &Expr) -> HashSet<String> {
    struct Names(HashSet<String>);
    impl Visit for Names {
        fn ty(&mut self, t: &mut TypeExpr) {
            if let TypeKind::Named {
                module: None, name, ..
            } = &t.kind
            {
                self.0.insert(name.name.clone());
            }
        }
    }
    let mut v = Names(HashSet::new());
    visit_pattern(&mut p.clone(), &mut v);
    visit_expr(&mut e.clone(), &mut v);
    v.0
}

pub fn refused_diagnostic(refused: &Refused) -> crate::Diagnostic {
    crate::Diagnostic::new(
        crate::Code::Unexpected,
        refused.span,
        format!(
            "this file needs a hand edit before it can be written in the new syntax: {}",
            refused.reason
        ),
    )
}
pub fn check_written(old: &Module, plan: &Plan, out: &str) -> Result<(), Vec<crate::Diagnostic>> {
    let (new, errors) = crate::parse_module(out);
    if !errors.is_empty() {
        return Err(errors);
    }
    let expected = crate::shape::canonical(&apply(plan, old.clone()));
    let actual = crate::shape::canonical(&new);
    if expected != actual {
        let expected = format!("{expected:?}");
        let actual = format!("{actual:?}");
        let at = expected
            .bytes()
            .zip(actual.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(expected.len().min(actual.len()));
        let a: String = expected
            .chars()
            .skip(at.saturating_sub(50))
            .take(180)
            .collect();
        let b: String = actual
            .chars()
            .skip(at.saturating_sub(50))
            .take(180)
            .collect();
        return Err(vec![crate::Diagnostic::new(
            crate::Code::Unexpected,
            Span::default(),
            format!(
                "migration write check: first differing node near shape byte {at}: expected {a}; found {b}"
            ),
        )]);
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Edit {
    pub span: Span,
    pub text: String,
    scope: Span,
    order: i32,
}
#[derive(Clone, Debug)]
pub struct Spliced {
    pub text: String,
    pub edits: Vec<Edit>,
}
impl Spliced {
    /// Maps an old byte position through all edits, with inserted text on
    /// its left. Positions inside replaced text map to its start.
    pub fn offset(&self, old: u32) -> u32 {
        let mut delta = 0i64;
        for e in &self.edits {
            if old < e.span.start {
                break;
            }
            if old < e.span.end {
                return (i64::from(e.span.start) + delta) as u32;
            }
            delta += e.text.len() as i64 - i64::from(e.span.end - e.span.start);
        }
        (i64::from(old) + delta) as u32
    }
}
fn edit(span: Span, text: impl Into<String>, scope: Span, order: i32) -> Edit {
    Edit {
        span,
        text: text.into(),
        scope,
        order,
    }
}
fn inserted(at: u32, text: impl Into<String>, scope: Span, order: i32) -> Edit {
    edit(Span::new(at, at), text, scope, order)
}
fn edited_range(
    source: &str,
    span: Span,
    edits: &[Edit],
) -> Result<String, Vec<crate::Diagnostic>> {
    let mut edits: Vec<_> = edits
        .iter()
        .filter(|e| within(span, e.scope) && within(span, e.span))
        .collect();
    edits.sort_by_key(|e| (e.span.start, e.order, e.span.end));
    let mut out = String::new();
    let mut at = span.start;
    for e in edits {
        if e.span.start < at {
            return Err(vec![crate::Diagnostic::new(
                crate::Code::Unexpected,
                e.span,
                "overlapping migration edits",
            )]);
        }
        out.push_str(&source[at as usize..e.span.start as usize]);
        out.push_str(&e.text);
        at = e.span.end;
    }
    out.push_str(&source[at as usize..span.end as usize]);
    Ok(out)
}
/// Edits the original bytes. No byte outside an edit is touched. The final
/// parse and independent apply transform must agree before text is returned.
pub fn splice(source: &str, plan: &Plan) -> Result<Spliced, Vec<crate::Diagnostic>> {
    if let Some(r) = &plan.refused {
        return Err(vec![refused_diagnostic(r)]);
    }
    let (module, old_errors) = crate::parse_module_old(source);
    let lexed = crate::lex(source);
    let mut edits = Vec::new();
    for (span, name) in &plan.renames {
        if let Some(token) = lexed
            .tokens
            .iter()
            .find(|t| within(*span, t.span) && matches!(t.kind, crate::token::TokenKind::Name(_)))
        {
            edits.push(edit(token.span, name, *span, 0));
        }
    }
    for (span, params) in &plan.params {
        if !plan.splits.contains(span) {
            let token = lexed
                .tokens
                .iter()
                .find(|t| t.span.start == span.start)
                .unwrap();
            edits.push(inserted(
                token.span.end,
                format!("({})", params.join(", ")),
                *span,
                0,
            ));
        }
    }
    for (span, drop, style) in [
        (module.imports_span, plan.drop_imports, module.imports_style),
        (module.exports_span, plan.drop_exports, module.exports_style),
    ] {
        if let Some(span) = span {
            for token in lexed.tokens.iter().filter(|t| within(span, t.span)) {
                if drop
                    || style == HeaderStyle::Keyword
                        && matches!(
                            token.kind,
                            crate::token::TokenKind::Import | crate::token::TokenKind::Export
                        )
                {
                    edits.push(edit(token.span, "", span, 0));
                }
            }
        }
    }
    struct Cases<'a> {
        plan: &'a Plan,
        source: &'a str,
        tokens: &'a [crate::token::Token],
        edits: &'a mut Vec<Edit>,
    }
    impl Visit for Cases<'_> {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if !enter {
                return;
            }
            if let ExprKind::Case {
                subject,
                clauses,
                syntax: Syntax::Old,
            } = &e.kind
            {
                let keyword = self
                    .tokens
                    .iter()
                    .find(|t| t.span.start == e.span.start)
                    .unwrap();
                self.edits.push(edit(keyword.span, "", e.span, 0));
                self.edits
                    .push(inserted(subject.span.end, " |>", e.span, 1000));
                for c in clauses {
                    if let Some(g) = &c.guard
                        && let Some(t) = self.tokens.iter().find(|t| {
                            t.span.start >= c.patterns.last().unwrap().span.end
                                && t.span.end <= g.span.start
                                && t.kind == crate::token::TokenKind::If
                        })
                    {
                        self.edits.push(edit(t.span, "?", c.span, 0));
                    }
                }
            }
            if self.plan.groups.contains(&e.span) {
                self.edits.push(inserted(e.span.start, "{ ", e.span, -10));
                self.edits.push(inserted(e.span.end, " }", e.span, 10));
            }
        }
        fn ty(&mut self, t: &mut TypeExpr) {
            if self.plan.type_groups.contains(&t.span) {
                let before = self
                    .tokens
                    .iter()
                    .rfind(|token| token.span.end <= t.span.start);
                let after = self
                    .tokens
                    .iter()
                    .find(|token| token.span.start >= t.span.end);
                if !before.is_some_and(|t| t.kind == crate::token::TokenKind::LBrace)
                    || !after.is_some_and(|t| t.kind == crate::token::TokenKind::RBrace)
                {
                    self.edits.push(inserted(t.span.start, "{ ", t.span, -10));
                    self.edits.push(inserted(t.span.end, " }", t.span, 10));
                }
            }
        }
        fn block(&mut self, b: &mut Block) {
            let spans: Vec<_> = b
                .stmts
                .iter()
                .map(|s| match s {
                    Stmt::Bind(b) => b.span,
                    Stmt::Env { span, .. } => *span,
                    Stmt::Expr(e) => e.span,
                })
                .collect();
            self.lines(&spans);
        }
    }
    impl Cases<'_> {
        fn lines(&mut self, spans: &[Span]) {
            for pair in spans.windows(2) {
                let next = pair[1];
                let mut token = self.tokens.iter().find(|t| t.span.start == next.start);
                while token.is_some_and(|t| t.kind == crate::token::TokenKind::Case) {
                    token = self
                        .tokens
                        .iter()
                        .find(|t| t.span.start >= token.unwrap().span.end);
                }
                let needs = self.plan.extractions.contains_key(&next)
                    || token
                        .is_some_and(|t| self.plan.groups.iter().any(|s| s.start == t.span.start))
                    || self.plan.groups.contains(&next)
                    || token.is_some_and(|t| {
                        matches!(
                            t.kind,
                            crate::token::TokenKind::LParen
                                | crate::token::TokenKind::LBrace
                                | crate::token::TokenKind::Minus
                                | crate::token::TokenKind::MinusDot
                        )
                    });
                if needs
                    && !self.source[pair[0].end as usize..next.start as usize]
                        .contains(['\n', '\r'])
                {
                    self.edits.push(inserted(next.start, "\n", next, -200));
                }
            }
        }
    }
    let mut cases = Cases {
        plan,
        source,
        tokens: &lexed.tokens,
        edits: &mut edits,
    };
    visit_module(&mut module.clone(), &mut cases);
    cases.lines(&module.items.iter().map(|i| i.span).collect::<Vec<_>>());
    let mut top: Vec<_> = [module.imports_span, module.exports_span]
        .into_iter()
        .flatten()
        .collect();
    if let Some(first) = module.items.first() {
        top.push(first.span);
    }
    for pair in top.windows(2) {
        if !source[pair[0].end as usize..pair[1].start as usize].contains(['\n', '\r']) {
            edits.push(inserted(pair[1].start, "\n", pair[1], -200));
        }
    }
    // A choice split keeps the original body and its comments on the last
    // clause; earlier clauses get only copies of its code.
    struct Choices<'a> {
        source: &'a str,
        plan: &'a Plan,
        tokens: &'a [crate::token::Token],
        edits: &'a mut Vec<Edit>,
    }
    impl Visit for Choices<'_> {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if !enter || !self.plan.choices.contains(&e.span) {
                return;
            }
            let ExprKind::Case { clauses, .. } = &e.kind else {
                return;
            };
            let c = &clauses[0];
            let body = edited_range(self.source, c.body.span, self.edits).unwrap();
            let lexed = crate::lex(&body);
            let mut clean = body.clone();
            for c in lexed.comments.iter().rev() {
                clean.replace_range(c.span.start as usize..c.span.end as usize, "");
            }
            for pair in c.patterns.windows(2) {
                let token = self
                    .tokens
                    .iter()
                    .find(|t| {
                        t.kind == crate::token::TokenKind::Pipe
                            && pair[0].span.end <= t.span.start
                            && t.span.end <= pair[1].span.start
                    })
                    .unwrap();
                self.edits
                    .push(edit(token.span, format!("-> {clean}\n"), e.span, 0));
            }
        }
    }
    visit_module(
        &mut module.clone(),
        &mut Choices {
            source,
            plan,
            tokens: &lexed.tokens,
            edits: &mut edits,
        },
    );
    // Extract the smallest functions first, so an enclosing extracted
    // function receives the same edits as its original body.
    let mut extractions: Vec<_> = plan.extractions.iter().collect();
    extractions.sort_by_key(|(_, x)| x.function.span.end - x.function.span.start);
    for (container, extraction) in extractions {
        let f = extraction.function.span;
        let value = edited_range(source, f, &edits)?;
        edits.retain(|e| !within(f, e.scope));
        edits.push(edit(f, extraction.name.clone(), f, 0));
        edits.push(inserted(
            container.start,
            format!("{{ {} = {value}\n", extraction.name),
            *container,
            -100,
        ));
        edits.push(inserted(container.end, " }", *container, 100));
    }
    // Splits reorder whole pattern/value parts, so each part is rendered
    // from its edited original slice, never from made-up source spans.
    struct Splits<'a> {
        source: &'a str,
        plan: &'a Plan,
        edits: &'a mut Vec<Edit>,
    }
    impl Visit for Splits<'_> {
        fn binding(&mut self, b: &mut Binding, enter: bool) {
            if enter || !self.plan.splits.contains(&b.span) {
                return;
            }
            let (PatternKind::Tuple(ps), ExprKind::Tuple(es)) = (&b.pattern.kind, &b.value.kind)
            else {
                unreachable!()
            };
            let mut parts = Vec::new();
            for (p, e) in ps.iter().zip(es) {
                let mut pattern = edited_range(self.source, p.span, self.edits).unwrap();
                let names: Vec<_> = self.plan.params[&b.span]
                    .iter()
                    .filter(|n| {
                        self.plan.renames.iter().any(|(span, value)| {
                            *n == value && (within(p.span, *span) || within(e.span, *span))
                        })
                    })
                    .cloned()
                    .collect();
                if !names.is_empty() {
                    let end = pattern
                        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .unwrap_or(pattern.len());
                    pattern.insert_str(end, &format!("({})", names.join(", ")));
                }
                parts.push(format!(
                    "{pattern} = {}",
                    edited_range(self.source, e.span, self.edits).unwrap()
                ));
            }
            self.edits.retain(|e| !within(b.span, e.scope));
            self.edits.push(edit(b.span, parts.join("\n"), b.span, 0));
        }
    }
    visit_module(
        &mut module.clone(),
        &mut Splits {
            source,
            plan,
            edits: &mut edits,
        },
    );
    if source.contains('\r') && edits.iter().any(|e| e.order == -200) {
        return Err(vec![refused_diagnostic(&Refused {
            reason: "cr-line-end",
            span: edits.iter().find(|e| e.order == -200).unwrap().span,
        })]);
    }
    edits.sort_by_key(|e| (e.span.start, e.order, e.span.end));
    // Type definition parameter and type-expression spans may coincide in
    // a recovered tree. Identical edits are one edit, never overlapping.
    edits.dedup_by(|a, b| {
        a.span == b.span
            && a.text == b.text
            && (a.span.start != a.span.end || a.order == b.order && a.scope == b.scope)
    });
    let text = edited_range(source, Span::new(0, source.len() as u32), &edits)?;
    let before: Vec<_> = crate::lex(source)
        .comments
        .into_iter()
        .map(|c| c.text)
        .collect();
    let after: Vec<_> = crate::lex(&text)
        .comments
        .into_iter()
        .map(|c| c.text)
        .collect();
    if before != after {
        return Err(vec![refused_diagnostic(&Refused {
            reason: "tyvar-owner",
            span: Span::default(),
        })]);
    }
    if source.bytes().filter(|b| *b == b'\r').count()
        != text.bytes().filter(|b| *b == b'\r').count()
    {
        return Err(vec![refused_diagnostic(&Refused {
            reason: "cr-line-end",
            span: Span::default(),
        })]);
    }
    if old_errors.is_empty() {
        check_written(&module, plan, &text)?;
    } else {
        let (_, new_errors) = crate::parse_module(&text);
        if old_errors.iter().map(|d| d.code).collect::<Vec<_>>()
            != new_errors.iter().map(|d| d.code).collect::<Vec<_>>()
        {
            return Err(vec![refused_diagnostic(&Refused {
                reason: "parse-errors",
                span: old_errors[0].span,
            })]);
        }
    }
    Ok(Spliced { text, edits })
}

fn json(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
/// Each report entry is traceable to an original node. Output positions
/// come from a structural walk of apply(plan) and the parsed written text.
pub fn report(
    old: &Module,
    plan: &Plan,
    source: &str,
    output: Option<&str>,
    path: &str,
) -> Vec<String> {
    #[derive(Default)]
    struct Patterns(Vec<Span>);
    impl Visit for Patterns {
        fn pattern(&mut self, p: &mut Pattern) {
            self.0.push(p.span);
        }
    }
    let mut positions = HashMap::new();
    let new = output.map(crate::parse_module).map(|(m, _)| m);
    if let Some(new) = &new {
        let mut expected = Patterns::default();
        visit_module(&mut apply(plan, old.clone()), &mut expected);
        let mut actual = Patterns::default();
        visit_module(&mut new.clone(), &mut actual);
        if expected.0.len() == actual.0.len() {
            positions.extend(expected.0.into_iter().zip(actual.0));
        }
    }
    let lines = crate::span::LineIndex::new(source);
    let out_lines = output.map(crate::span::LineIndex::new);
    let mut entries = plan.entries.clone();
    if let (Some(new), Some(output)) = (&new, output) {
        for item in &old.items {
            struct Texts {
                patterns: Vec<Span>,
            }
            impl Visit for Texts {
                fn binding(&mut self, b: &mut Binding, enter: bool) {
                    if enter {
                        self.patterns.push(b.pattern.span);
                    }
                }
                fn expr(&mut self, e: &mut Expr, enter: bool) {
                    if enter && let ExprKind::Fn(f) = &e.kind {
                        self.patterns.extend(f.params.iter().filter_map(|p| {
                            if let Param::Pattern(p) = p {
                                Some(p.span)
                            } else {
                                None
                            }
                        }));
                    }
                }
            }
            let _ = new;
            let owner = if let ItemKind::Binding(b) = &item.kind {
                if matches!(
                    ungroup(&b.value).kind,
                    ExprKind::Fn(_) | ExprKind::Clauses(_)
                ) {
                    binding_name(&b.pattern)
                        .map(|n| {
                            // Every ordinary entry carries this module path.
                            let module = &plan.module;
                            format!("{module}.{n}")
                        })
                        .unwrap_or_else(|| "ilang_start".into())
                } else {
                    "ilang_start".into()
                }
            } else {
                "ilang_start".into()
            };
            let mut texts = Texts { patterns: vec![] };
            let mut m = Module {
                items: vec![item.clone()],
                ..Module::default()
            };
            visit_module(&mut m, &mut texts);
            for p in texts.patterns {
                if let Some(np) = positions.get(&p) {
                    let a = &source[p.start as usize..p.end as usize];
                    let b = &output[np.start as usize..np.end as usize];
                    if a != b {
                        entries.push(Entry {
                            category: "pattern-text",
                            owner: owner.clone(),
                            span: p,
                            reason: format!("{{\"old\":{},\"new\":{}}}", json(a), json(b)),
                            pattern: None,
                        });
                    }
                }
            }
        }
    }
    entries.sort_by_key(|e| (e.span.start, e.category, e.reason.clone()));
    entries.dedup_by(|a, b| a.span == b.span && a.category == b.category && a.reason == b.reason);
    entries
        .into_iter()
        .enumerate()
        .map(|(id, e)| {
            let (line, _) = lines.line_col(e.span.start);
            let mut reason = e.reason;
            if e.category == "split" && reason.starts_with("whole ") {
                reason = format!("whole {reason}");
            }
            if e.category != "pattern-text" {
                reason.push_str(&format!(" span={}:{}", e.span.start, e.span.end));
            }
            if let Some(p) = e.pattern {
                let (l, c) = lines.line_col(p.start);
                let out = positions
                    .get(&p)
                    .zip(out_lines.as_ref())
                    .map(|(p, lines)| {
                        let (l, c) = lines.line_col(p.start);
                        format!("{l}:{c}")
                    })
                    .unwrap_or_else(|| "unknown".into());
                reason.push_str(&format!(" old={l}:{c} out={out}"));
            }
            format!(
                "{} {} {path}:{line} r{} {reason}",
                e.category,
                e.owner,
                id + 1
            )
        })
        .collect()
}
