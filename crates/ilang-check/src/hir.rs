//! The resolved program (HIR). It has the shape of the AST, but every name
//! use is resolved: to a local variable, a top-level definition, a
//! constructor, a type, or a type variable. Module members `m.x` are resolved
//! to the definition they name, so the HIR has no modules in expressions.
//! Every node keeps its span; spans are byte ranges in the file of the
//! module that holds the node (`Module::source`).
//!
//! §2.4 for the inferred type of an exported binding, and the rules of
//! §10.2 and §10.3 that need the inferred type of an entry point, are
//! checked by type inference (`types/infer.rs`); resolution checks the
//! written types.

use ilang_package::{ModuleKind, PackageId, SourceId};
use ilang_syntax::ast::{BinOp, Ident, UnOp};
use ilang_syntax::span::Span;
use num_bigint::BigUint;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u32);

/// A custom type or an alias, from any module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

/// A name bound by a pattern or an environment parameter. Unique in the
/// program. Top-level names are bound by patterns too: see `DefKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub u32);

/// A type variable. Unique in the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TyVarId(pub u32);

/// A top-level value name: the module and the index in `Module::defs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefId {
    pub module: ModuleId,
    pub index: u32,
}

/// A constructor: its type and its index in that type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CtorRef {
    pub ty: TypeId,
    pub index: u32,
}

/// A top-level binding: the module and the index in `Module::bindings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingRef {
    pub module: ModuleId,
    pub index: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    /// Reads in invalid dependency components, before query recovery.
    pub failed_reads: std::collections::HashSet<OfId>,
    /// Expression SCC members invalidated by a type-dependency cycle.
    pub failed_type_groups: std::collections::HashSet<BindingRef>,
    /// Written value reads and structural extraction items (§4.3, §4.7).
    pub of_sites: Vec<OfSite>,
    pub alias_patterns: Vec<AliasPattern>,
    /// Direct alias dependencies and their reverse closure, computed once.
    pub alias_dependencies: Vec<Vec<TypeId>>,
    pub alias_sources: Vec<Vec<OfId>>,
    pub value_aliases: Option<std::collections::HashSet<TypeId>>,
    pub modules: Vec<Module>,
    pub types: Vec<TypeDef>,
    /// The module of the built-in names (§12), in the package `i`.
    pub prelude: Option<ModuleId>,
    /// The package that is compiled: its `@main` and `@test` bindings are
    /// entry points (§10.2).
    pub root: Option<PackageId>,
    /// The host exports (§10.2): the names in `[wasm].exports` of the
    /// compiled package that are exported functions of a module.
    pub host_exports: Vec<DefId>,
    /// The number of `VarId`s and `TyVarId`s given out.
    pub var_count: u32,
    pub ty_var_count: u32,
    /// The written name of each `TyVarId`, by index.
    pub ty_var_names: Vec<String>,
    /// Filled by `order.rs`.
    pub order: Order,
    /// The functions of the host services (§9.7), by the service type and
    /// the index of the field, when their types are correct.
    pub host_fns: std::collections::HashMap<(TypeId, u32), HostFn>,
    /// The external functions outside the package `i` (§10.1), when their
    /// types are correct.
    pub external_fns: std::collections::HashMap<DefId, HostFn>,
}

/// A function at the host boundary (§9.7, §10.1), with the parts of B.2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFn {
    pub params: Vec<crate::types::HostType>,
    pub result: crate::types::HostType,
    /// The members of the errors, in the canonical order of B.2.
    pub errors: Vec<crate::types::HostType>,
}

impl Program {
    pub fn module(&self, id: ModuleId) -> &Module {
        &self.modules[id.0 as usize]
    }

    pub fn def(&self, id: DefId) -> &Def {
        &self.module(id.module).defs[id.index as usize]
    }

    pub fn ty(&self, id: TypeId) -> &TypeDef {
        &self.types[id.0 as usize]
    }

    pub fn binding(&self, b: BindingRef) -> &TopBinding {
        &self.module(b.module).bindings[b.index as usize]
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub path: String,
    pub package: PackageId,
    pub source: SourceId,
    pub kind: ModuleKind,
    /// The modules that this module imports.
    pub imports: Vec<ModuleId>,
    /// The name that each import gives to its module (§2.3).
    pub import_names: Vec<(String, ModuleId)>,
    /// The types that imports put into this module by name (§2.3).
    pub imported_types: Vec<TypeId>,
    /// The top-level value names: names bound by top-level bindings, and
    /// declarations.
    pub defs: Vec<Def>,
    /// The types defined in this module.
    pub types: Vec<TypeId>,
    /// The top-level bindings, in source order.
    pub bindings: Vec<TopBinding>,
}

#[derive(Clone, Debug)]
pub struct Def {
    pub name: String,
    pub span: Span,
    pub kind: DefKind,
    pub exported: bool,
    pub deprecated: Option<String>,
}

#[derive(Clone, Debug)]
pub enum DefKind {
    /// A name bound by the pattern of `Module::bindings[binding]`. `var` is
    /// the variable of that name in the pattern.
    Binding { binding: u32, var: VarId },
    /// `@external("module", "function") name: Type` (§10.1). Modules that
    /// start with `ilang:` are operations of the runtime, only in package `i`.
    External {
        module: String,
        function: String,
        ty: Type,
        /// The type variables of `ty`.
        ty_vars: Vec<TyVarId>,
    },
}

#[derive(Clone, Debug)]
pub struct TopBinding {
    pub binding: Binding,
    /// The names it binds, in pattern order.
    pub defs: Vec<DefId>,
    /// `@main` (§10.2).
    pub main: bool,
    /// `@test` (§10.2).
    pub test: bool,
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub name: String,
    pub span: Span,
    pub module: ModuleId,
    /// The type parameters, in order.
    pub params: Vec<(String, TyVarId)>,
    pub body: TypeBody,
    pub exported: bool,
    /// Exported as `T(..)` (§2.4).
    pub constructors_exported: bool,
    /// `@host("namespace")` or `@host("namespace", "async")` (§9.7).
    pub host: Option<Host>,
    pub deprecated: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Host {
    pub namespace: String,
    pub is_async: bool,
}

#[derive(Clone, Debug)]
pub enum TypeBody {
    Custom(Vec<CtorDef>),
    Alias(Type),
}

#[derive(Clone, Debug)]
pub struct CtorDef {
    pub name: String,
    pub span: Span,
    pub fields: Vec<FieldDef>,
}

#[derive(Clone, Debug)]
pub struct FieldDef {
    pub label: Option<Ident>,
    pub ty: Type,
}

// ---- Types (§3) ----

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinType {
    Int,
    Float,
    String,
    List,
    BitArray,
    UtfCodepoint,
    Never,
    Boundary,
    Continuation,
}

impl BuiltinType {
    pub const ALL: [BuiltinType; 9] = [
        BuiltinType::Int,
        BuiltinType::Float,
        BuiltinType::String,
        BuiltinType::List,
        BuiltinType::BitArray,
        BuiltinType::UtfCodepoint,
        BuiltinType::Never,
        BuiltinType::Boundary,
        BuiltinType::Continuation,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BuiltinType::Int => "Int",
            BuiltinType::Float => "Float",
            BuiltinType::String => "String",
            BuiltinType::List => "List",
            BuiltinType::BitArray => "BitArray",
            BuiltinType::UtfCodepoint => "UtfCodepoint",
            BuiltinType::Never => "Never",
            BuiltinType::Boundary => "Boundary",
            BuiltinType::Continuation => "Continuation",
        }
    }

    /// The number of type parameters (§4.2, §4.5 rule 7).
    pub fn arity(self) -> usize {
        match self {
            BuiltinType::List => 1,
            BuiltinType::Boundary => 3,
            BuiltinType::Continuation => 4,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeRef {
    Custom(TypeId),
    Builtin(BuiltinType),
}

#[derive(Clone, Debug)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    Lit(crate::types::LitValue),
    Of {
        value: ValueRef,
        site: OfId,
        projection: Option<ProjectionId>,
    },
    /// A named type with the right number of arguments. An alias is kept as
    /// itself; it is not expanded.
    Named {
        ty: TypeRef,
        args: Vec<Type>,
    },
    Var(TyVarId),
    /// `_`.
    Hole,
    Tuple(Vec<Type>),
    Fn {
        params: Vec<FnParamType>,
        ret: Box<Type>,
        /// `None` is `? Never` (§3).
        err: Option<Box<Type>>,
        /// `None` is `! Never` (§3).
        env: Option<Box<Type>>,
    },
    Union(Vec<Type>),
    Inter(Vec<Type>),
    Diff(Vec<Type>),
    /// A type that could not be resolved; the problem has been reported.
    Error,
}

#[derive(Clone, Debug)]
pub struct FnParamType {
    pub label: Option<Ident>,
    pub ty: Type,
}

// ---- Expressions (§6) ----

/// `pattern = value` (§6.10), at the top level or in a block.
#[derive(Clone, Debug)]
pub struct Binding {
    pub pattern: Pattern,
    pub value: Expr,
    /// The value is a direct value (§6.10): the binding is generalized
    /// (§4.7), and its names may be used inside the function bodies of the
    /// value.
    pub direct: bool,
    /// The type variables written in the annotations of this binding that
    /// are not variables of an outer binding (§4.7). They are generalized
    /// with it.
    pub ty_vars: Vec<TyVarId>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Bind(Binding),
    /// `! T = e` (§6.10, §9.2). `ty` is one named type.
    Env {
        ty: Type,
        value: Expr,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// What a name in an expression is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueRef {
    /// A name bound by a pattern or a parameter in a function or a block.
    Local(VarId),
    /// A top-level name: a binding or a declaration, in this module or
    /// another (as an imported name, or as `module.name`). Built-in values
    /// (`panic`, `assert`) are top-level names of the prelude.
    Def(DefId),
    /// A constructor, used as a value or a function (§6.3).
    Ctor(CtorRef),
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(BigUint),
    Float(f64),
    String(String),
    Name(ValueRef),
    Tuple(Vec<Expr>),
    List {
        items: Vec<Expr>,
        rest: Option<Box<Expr>>,
    },
    BitArray(Vec<Segment<Expr>>),
    Block(Block),
    Fn(Box<Function>),
    Case {
        subject: Box<Expr>,
        clauses: Vec<Clause>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    /// `v.f`. Never a module member: those are `Name`.
    Field {
        target: Box<Expr>,
        name: Ident,
    },
    TupleIndex {
        target: Box<Expr>,
        index: BigUint,
        index_span: Span,
    },
    /// `C(..base, f: e)` (§6.7).
    RecordUpdate {
        ctor: CtorRef,
        ctor_span: Span,
        base: Box<Expr>,
        fields: Vec<(Ident, Expr)>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// An expression that could not be resolved; the problem has been
    /// reported.
    Error,
}

#[derive(Clone, Debug)]
pub struct Arg {
    pub label: Option<Ident>,
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub struct Clause {
    /// The choices `p1 | p2 | …`. They bind the same names, with the same
    /// `VarId`s.
    pub patterns: Vec<Pattern>,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// `(params) -> A ? E ! N { body }` (§6.8).
#[derive(Clone, Debug)]
pub struct Function {
    pub params: Vec<Param>,
    pub ret: Option<RetType>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct RetType {
    pub value: Type,
    pub err: Option<Type>,
    pub env: Option<Type>,
}

#[derive(Clone, Debug)]
pub enum Param {
    Pattern {
        pattern: Pattern,
        /// The label (§6.11): the last name in the `=` chain at the top.
        label: Option<String>,
    },
    /// `name ! T` (§6.11). `ty` is one named type.
    Env { var: VarId, name: Ident, ty: Type },
}

#[derive(Clone, Debug)]
pub struct Segment<T> {
    pub value: T,
    pub format: Option<Format>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Format {
    pub name: Ident,
    pub arg: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

/// A name that a pattern binds.
#[derive(Clone, Debug)]
pub struct Binder {
    pub var: VarId,
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatternKind {
    Var(Binder),
    Discard,
    /// `x: T` or `_: T`.
    Typed {
        binder: Option<Binder>,
        ty: Type,
    },
    /// `x = p`.
    Named {
        binder: Binder,
        pattern: Box<Pattern>,
    },
    Int {
        negative: bool,
        value: BigUint,
    },
    Float {
        negative: bool,
        value: f64,
    },
    String(String),
    /// `"lit" <> rest`.
    Prefix {
        prefix: String,
        rest: Option<Binder>,
    },
    /// `C(p, …)`, `C(p, ..)`. The parts are as written; matching them to
    /// fields by position and label (§7.1) is left to type checking.
    Constructor {
        ctor: CtorRef,
        args: Vec<Pattern>,
        spread: bool,
    },
    Tuple(Vec<Pattern>),
    /// `rest`: `None` is no rest; `Some(None)` is `..` or `.._`.
    List {
        items: Vec<Pattern>,
        rest: Option<Option<Binder>>,
    },
    BitArray(Vec<Segment<Pattern>>),
    /// A pattern that could not be resolved, with its resolved parts.
    Error(Vec<Pattern>),
}

// ---- Order (§2.5, §4.7) ----

#[derive(Clone, Debug, Default)]
pub struct Order {
    /// The groups of top-level bindings (§4.7), each after the groups it
    /// uses. Modules come after the modules they import.
    pub groups: Vec<Vec<BindingRef>>,
    /// The order in which top-level values are worked out (§2.5).
    pub init: Vec<BindingRef>,
}

/// One written read, retained for readiness and diagnostics, never a solver head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OfId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProjectionId(pub u32);
#[derive(Clone, Debug)]
pub struct OfSite {
    pub module: ModuleId,
    pub span: Span,
    pub value: ValueRef,
    pub projection: Option<ProjectionId>,
    pub permanent: bool,
}
#[derive(Clone, Debug)]
pub struct AliasPattern {
    pub heads: Vec<(Span, TypeRef)>,
    pub head_span: Span,
    pub module: ModuleId,
    pub span: Span,
    pub source: Type,
    pub pattern: TypePattern,
    pub binders: Vec<TypeId>,
}
#[derive(Clone, Debug)]
pub enum TypePattern {
    Bind(TypeId),
    Skip,
    Named(TypeRef, Vec<TypePattern>),
    Tuple(Vec<TypePattern>),
    Exact(Type),
    Error,
}

/// Visits every type node, preserving value-reference provenance.
pub fn walk_type(t: &Type, f: &mut impl FnMut(&Type)) {
    f(t);
    match &t.kind {
        TypeKind::Named { args, .. }
        | TypeKind::Tuple(args)
        | TypeKind::Union(args)
        | TypeKind::Inter(args)
        | TypeKind::Diff(args) => {
            for t in args {
                walk_type(t, f);
            }
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            for p in params {
                walk_type(&p.ty, f);
            }
            walk_type(ret, f);
            for t in err.iter().chain(env) {
                walk_type(t, f);
            }
        }
        TypeKind::Lit(_)
        | TypeKind::Of { .. }
        | TypeKind::Var(_)
        | TypeKind::Hole
        | TypeKind::Error => {}
    }
}

/// Whether a type transitively depends on a value. This survives completion.
pub fn value_dependent(p: &Program, t: &Type) -> bool {
    if p.of_sites.is_empty() {
        return false;
    }
    if let Some(aliases) = &p.value_aliases {
        let mut found = false;
        walk_type(t, &mut |t| match &t.kind {
            TypeKind::Of { .. } => found = true,
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } => found |= aliases.contains(id),
            _ => {}
        });
        return found;
    }
    fn go(p: &Program, t: &Type, seen: &mut std::collections::HashSet<TypeId>) -> bool {
        let mut found = false;
        walk_type(t, &mut |t| match &t.kind {
            TypeKind::Of { .. } => found = true,
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } if seen.insert(*id) => {
                if let TypeBody::Alias(body) = &p.ty(*id).body {
                    found |= go(p, body, seen);
                }
            }
            _ => {}
        });
        found
    }
    go(p, t, &mut std::collections::HashSet::new())
}

/// Summaries use only direct written edges; propagating provenance visits
/// each alias and edge once plus the materialized source sets (O1).
pub fn summarize_aliases(p: &mut Program) {
    let mut dependents = vec![Vec::new(); p.types.len()];
    let mut dependencies = vec![Vec::new(); p.types.len()];
    let mut affected = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    for (i, d) in p.types.iter().enumerate() {
        let TypeBody::Alias(body) = &d.body else {
            continue;
        };
        walk_type(body, &mut |t| match &t.kind {
            TypeKind::Of { .. } => {
                if affected.insert(TypeId(i as u32)) {
                    queue.push_back(TypeId(i as u32));
                }
            }
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } if matches!(p.ty(*id).body, TypeBody::Alias(_)) => {
                dependencies[i].push(*id);
            }
            _ => {}
        });
        dependencies[i].sort();
        dependencies[i].dedup();
        for id in &dependencies[i] {
            dependents[id.0 as usize].push(TypeId(i as u32));
        }
    }
    while let Some(id) = queue.pop_front() {
        for next in &dependents[id.0 as usize] {
            if affected.insert(*next) {
                queue.push_back(*next);
            }
        }
    }
    let mut sources = vec![Vec::new(); p.types.len()];
    let mut remaining = dependencies.iter().map(Vec::len).collect::<Vec<_>>();
    let mut ready = std::collections::VecDeque::new();
    for (i, d) in p.types.iter().enumerate() {
        if let TypeBody::Alias(t) = &d.body {
            walk_type(t, &mut |t| {
                if let TypeKind::Of { site, .. } = t.kind {
                    sources[i].push(site);
                }
            });
            if remaining[i] == 0 {
                ready.push_back(i);
            }
        }
    }
    while let Some(i) = ready.pop_front() {
        for dep in &dependencies[i] {
            let extra = sources[dep.0 as usize].clone();
            sources[i].extend(extra);
        }
        sources[i].sort_by_key(|site| site.0);
        sources[i].dedup();
        for next in &dependents[i] {
            let k = next.0 as usize;
            remaining[k] -= 1;
            if remaining[k] == 0 {
                ready.push_back(k);
            }
        }
    }
    p.alias_sources = sources;
    p.alias_dependencies = dependencies;
    p.value_aliases = Some(affected);
}

/// All written types in a binding, including nested bodies and patterns.
/// Runtime-use analysis deliberately does not call this visitor.
pub fn binding_types(b: &Binding, f: &mut impl FnMut(&Type)) {
    fn pattern(p: &Pattern, f: &mut impl FnMut(&Type)) {
        match &p.kind {
            PatternKind::Typed { ty, .. } => f(ty),
            PatternKind::Named { pattern: p, .. } => pattern(p, f),
            PatternKind::Constructor { args, .. }
            | PatternKind::Tuple(args)
            | PatternKind::Error(args) => {
                for p in args {
                    pattern(p, f);
                }
            }
            PatternKind::List { items, .. } => {
                for p in items {
                    pattern(p, f);
                }
            }
            PatternKind::BitArray(segs) => {
                for seg in segs {
                    pattern(&seg.value, f);
                    if let Some(arg) = seg.format.as_ref().and_then(|s| s.arg.as_deref()) {
                        expr(arg, f);
                    }
                }
            }
            _ => {}
        }
    }
    fn block(b: &Block, f: &mut impl FnMut(&Type)) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => binding_types(b, f),
                Stmt::Env { ty, value, .. } => {
                    f(ty);
                    expr(value, f);
                }
                Stmt::Expr(e) => expr(e, f),
            }
        }
    }
    fn expr(e: &Expr, f: &mut impl FnMut(&Type)) {
        match &e.kind {
            ExprKind::Fn(fun) => {
                for p in &fun.params {
                    match p {
                        Param::Pattern { pattern: p, .. } => pattern(p, f),
                        Param::Env { ty, .. } => f(ty),
                    }
                }
                if let Some(ret) = &fun.ret {
                    for t in [Some(&ret.value), ret.err.as_ref(), ret.env.as_ref()]
                        .into_iter()
                        .flatten()
                    {
                        f(t);
                    }
                }
                block(&fun.body, f);
            }
            ExprKind::Block(b) => block(b, f),
            ExprKind::Case { subject, clauses } => {
                expr(subject, f);
                for c in clauses {
                    for p in &c.patterns {
                        pattern(p, f);
                    }
                    if let Some(g) = &c.guard {
                        expr(g, f);
                    }
                    expr(&c.body, f);
                }
            }
            ExprKind::Tuple(ts) => {
                for e in ts {
                    expr(e, f);
                }
            }
            ExprKind::List { items, rest } => {
                for e in items.iter().chain(rest.as_deref()) {
                    expr(e, f);
                }
            }
            ExprKind::Call { callee, args } => {
                expr(callee, f);
                for a in args {
                    expr(&a.value, f);
                }
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => expr(target, f),
            ExprKind::RecordUpdate { base, fields, .. } => {
                expr(base, f);
                for (_, e) in fields {
                    expr(e, f);
                }
            }
            ExprKind::Unary { expr: e, .. } => expr(e, f),
            ExprKind::Binary { lhs, rhs, .. } => {
                expr(lhs, f);
                expr(rhs, f);
            }
            ExprKind::BitArray(segs) => {
                for seg in segs {
                    expr(&seg.value, f);
                    if let Some(arg) = seg.format.as_ref().and_then(|s| s.arg.as_deref()) {
                        expr(arg, f);
                    }
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name(_)
            | ExprKind::Error => {}
        }
    }
    pattern(&b.pattern, f);
    expr(&b.value, f);
}
