//! Name resolution of the syntax-three tree (`ilang_syntax::syn3::ast`),
//! with no lowering to the tree before it (docs/design/syn3-final-ast.md,
//! step 3). [`resolve3`] gives the HIR and the diagnostics that
//! `resolve(lower(module))` gives; the test `resolve3_differential` holds
//! the two paths equal.
//!
//! Every function here reads the new tree and is the twin of the function
//! of the same name in `resolve.rs`; what does not read a tree (scopes,
//! lookups, the rules about the whole program) is shared with it. The rows
//! of §1 of the note are here as code:
//!
//! - a sum is the union right after its member data definitions
//!   ([`pre::sums`], `type_shape`);
//! - a chain is a function, a function of `%arg` over a match, or a
//!   function of fresh parameters over a match (`chain`);
//! - a pipe into a chain is a match (`case`);
//! - `e <- (s1, s2)` is environment statements before `e` (`flatten_provide`);
//! - needs become rows in one place ([`R3::needs_row`]).
//!
//! The errors of the forms that this compiler cannot type yet are found by
//! [`pre::pre`], before anything is resolved.

mod pre;

use super::*;
use ilang_syntax::ast as o;
use ilang_syntax::syn3::ast as n;
use pre::{
    AndPlan, ChainKind, Inner, Pre, addr, and_plan, attribute_call, attribute_strings, chain_kind,
    head_params, id, is_core_never, is_match, keyed_plan, provide_apart,
};
use std::ops::{Deref, DerefMut};

/// The stack of the thread that resolves: trees made by macros can be far
/// deeper than a normal stack, and every pass here is plain recursion. The
/// pages are taken only when they are used.
const STACK: usize = 1 << 30;

/// The errors of a module that the lowering reported: the forms of syntax
/// three that this compiler cannot type yet. `lisp` is set for a module
/// read from the Lisp surface.
pub fn lowering_errors(module: &n::Module, lisp: bool) -> Vec<ilang_syntax::Diagnostic> {
    pre::pre(module, lisp).errors
}

/// Resolves the modules of a graph from their syntax-three trees. `parsed`
/// gives the place of each module in the graph and its expansion origins;
/// its `ast` is not read. The diagnostics start with the errors of
/// [`lowering_errors`], module by module.
pub fn resolve3(
    graph: &PackageGraph,
    parsed: &[Parsed],
    modules: &[n::Module],
) -> (Program, Vec<Diagnostic>) {
    assert_eq!(parsed.len(), modules.len());
    on_deep_stack(|| resolve_here(graph, parsed, modules))
}

/// Stub of `ilang_syntax::syn3::deep::deep` (the integrator's helper, not on
/// this branch yet): replace every call by it and delete this function.
fn on_deep_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .name("resolve".into())
            .stack_size(STACK)
            .spawn_scoped(scope, work)
            .expect("the resolver's thread starts");
        match handle.join() {
            Ok(out) => out,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

fn pres(graph: &PackageGraph, parsed: &[Parsed], modules: &[n::Module]) -> Vec<Pre> {
    parsed
        .iter()
        .zip(modules)
        .map(|(p, m)| pre::pre(m, graph.sources.get(p.source).display.ends_with(".is")))
        .collect()
}

fn resolve_here(
    graph: &PackageGraph,
    parsed: &[Parsed],
    modules: &[n::Module],
) -> (Program, Vec<Diagnostic>) {
    let pre = pres(graph, parsed, modules);
    let mut diags = Vec::new();
    for (p, pre) in parsed.iter().zip(&pre) {
        diags.extend(
            pre.errors.iter().map(|d| {
                Diagnostic::error(d.code.as_str(), Some(p.source), d.span, d.message.clone())
            }),
        );
    }
    let mut r = R3 {
        r: Resolver {
            graph,
            parsed,
            p: Program::default(),
            diags,
            generated_values: Vec::new(),
            generated_types: Vec::new(),
            defs_by_name: Vec::new(),
            macros_by_name: Vec::new(),
            types_by_name: Vec::new(),
            ctors_by_name: Vec::new(),
            slots: Vec::new(),
            scopes: Vec::new(),
            by_path: HashMap::new(),
            builtin_values: HashMap::new(),
            builtin_types: HashMap::new(),
            cur: ModuleId(0),
            locals: Vec::new(),
            fn_depth: 0,
            ty_frames: Vec::new(),
            ty_def: None,
            circular: HashSet::new(),
            pending: Vec::new(),
            reserved: Vec::new(),
            checked_inputs: None,
        },
        modules,
        pre: &pre,
    };
    let count = parsed.len() as u32;
    r.create_modules();
    for m in 0..count {
        r.collect(ModuleId(m));
    }
    r.builtins();
    for m in 0..count {
        r.imports(ModuleId(m));
    }
    r.check_generated_collisions();
    r.import_circles_of(&|m| {
        modules[m]
            .imports
            .iter()
            .map(|i| (i.path.iter().map(|p| p.name.clone()).collect(), i.span.0))
            .collect()
    });
    for m in 0..count {
        r.type_defs(ModuleId(m));
    }
    summarize_aliases(&mut r.p);
    r.alias_circles();
    r.early_type_checks();
    r.host_forms();
    for m in 0..count {
        r.values(ModuleId(m));
    }
    for m in 0..count {
        r.exported_types(ModuleId(m));
    }
    r.environment_reads();
    r.program_rules();
    let mut r = r.r;
    r.p.typed_expander = parsed.iter().find_map(|p| p.typed_macros.clone());
    if !r.p.typed_macros.is_empty() {
        r.p.expansion_names = Some(std::sync::Arc::new(ExpansionNames {
            generated_values: r.generated_values,
            generated_types: r.generated_types,
            defs_by_name: r.defs_by_name,
            macros_by_name: r.macros_by_name,
            types_by_name: r.types_by_name,
            ctors_by_name: r.ctors_by_name,
            slots: r.slots,
            scopes: r.scopes,
            by_path: r.by_path,
            builtin_values: r.builtin_values,
            builtin_types: r.builtin_types,
        }));
    }
    (r.p, r.diags)
}

/// [`typed_output`] for an expression of the syntax-three tree. The
/// expression stands alone: it is a macro result, so it has no fresh names
/// of a module and no binding around it.
#[allow(clippy::too_many_arguments)]
pub fn typed_output3(
    graph: &PackageGraph,
    parsed: &[Parsed],
    original: &Program,
    module: ModuleId,
    call: Span,
    expression: &n::Expr,
    var_count: u32,
    ty_var_count: u32,
    checked: &CheckedInputs,
) -> Result<(Expr, Program), Vec<Diagnostic>> {
    on_deep_stack(|| {
        typed_output_here(
            graph,
            parsed,
            original,
            module,
            call,
            expression,
            var_count,
            ty_var_count,
            checked,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn typed_output_here(
    graph: &PackageGraph,
    parsed: &[Parsed],
    original: &Program,
    module: ModuleId,
    call: Span,
    expression: &n::Expr,
    var_count: u32,
    ty_var_count: u32,
    checked: &CheckedInputs,
) -> Result<(Expr, Program), Vec<Diagnostic>> {
    let names = original
        .expansion_names
        .as_ref()
        .expect("typed module has resolution tables");
    let scope = original
        .expansion_scopes
        .get(&(module, call))
        .expect("typed call has lexical scope");
    let mut p = original.clone();
    p.var_count = var_count;
    p.ty_var_count = ty_var_count;
    p.ty_var_names
        .resize_with(ty_var_count as usize, || "Output".into());
    let reads = p.of_sites.len();
    // The fresh names are numbered as the lowering of the expression alone
    // numbered them.
    let mut pre: Vec<Pre> = (0..parsed.len()).map(|_| Pre::default()).collect();
    let own = pre::pre_expr(expression);
    if let Some(first) = own.errors.first() {
        return Err(vec![Diagnostic::error(
            first.code.as_str(),
            Some(parsed[module.0 as usize].source),
            first.span,
            first.message.clone(),
        )]);
    }
    pre[module.0 as usize] = own;
    let modules: Vec<n::Module> = Vec::new();
    let mut r = R3 {
        r: Resolver {
            graph,
            parsed,
            p,
            diags: Vec::new(),
            generated_values: names.generated_values.clone(),
            generated_types: names.generated_types.clone(),
            defs_by_name: names.defs_by_name.clone(),
            macros_by_name: names.macros_by_name.clone(),
            types_by_name: names.types_by_name.clone(),
            ctors_by_name: names.ctors_by_name.clone(),
            slots: names.slots.clone(),
            scopes: names.scopes.clone(),
            by_path: names.by_path.clone(),
            builtin_values: names.builtin_values.clone(),
            builtin_types: names.builtin_types.clone(),
            cur: module,
            locals: scope.locals.clone(),
            fn_depth: scope.fn_depth,
            ty_frames: scope.ty_frames.clone(),
            ty_def: None,
            circular: HashSet::new(),
            pending: scope.pending.clone(),
            reserved: scope.reserved.clone(),
            checked_inputs: Some(checked),
        },
        modules: &modules,
        pre: &pre,
    };
    let result = r.expr(expression);
    let mut r = r.r;
    if r.p.of_sites.len() != reads {
        r.error(
            code::VALUE_TYPE_PLACE,
            expression.span.0,
            "a typed macro result cannot add a direct value-type read (T5)",
        );
    }
    if r.diags.iter().any(Diagnostic::is_error) {
        Err(r.diags)
    } else {
        Ok((result, r.p))
    }
}

/// The resolver, reading the syntax-three trees of the modules.
struct R3<'a, 't> {
    r: Resolver<'a>,
    modules: &'t [n::Module],
    pre: &'t [Pre],
}

impl<'a> Deref for R3<'a, '_> {
    type Target = Resolver<'a>;
    fn deref(&self) -> &Resolver<'a> {
        &self.r
    }
}

impl DerefMut for R3<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.r
    }
}

fn opt_id(i: &Option<n::Ident>) -> Option<o::Ident> {
    i.as_ref().map(id)
}

fn ident(name: impl Into<String>, span: Span) -> o::Ident {
    o::Ident {
        name: name.into(),
        span,
    }
}

/// What a type definition is to the resolver: an alias pattern, an alias,
/// or a custom type with its constructors (one for a data type, none for an
/// opaque type, the members for a sum).
struct TypeShape<'t> {
    alias_pattern: bool,
    params: Vec<o::Ident>,
    /// The head, when it is not a list of plain parameters of an alias.
    head: &'t [n::Type],
    body: ShapeBody<'t>,
}

enum ShapeBody<'t> {
    Alias(&'t n::Type),
    Custom(Vec<Ctor<'t>>),
}

struct Ctor<'t> {
    name: &'t n::Ident,
    fields: &'t [n::TItem],
}

/// A pattern of the tree, or one that the tree does not write.
enum VPat<'x> {
    Pat(&'x n::Pattern),
    /// A fresh name.
    Fresh(String, Span),
    /// One alternative of each position of a branch, as data.
    Tuple(Vec<&'x n::Pattern>, Span),
}

impl VPat<'_> {
    fn span(&self) -> Span {
        match self {
            VPat::Pat(p) => p.span.0,
            VPat::Fresh(_, span) | VPat::Tuple(_, span) => *span,
        }
    }
}

/// A statement of a block after `e <- (s)` is read as environment
/// statements.
enum VStmt<'x> {
    Bind(&'x n::Binding),
    Expr(&'x n::Expr),
    /// `%serviceN = e`.
    Service(String, &'x n::Expr),
    /// The environment statement of the service `e`, whose value is `e` or
    /// the fresh local that holds it.
    Env(Option<String>, &'x n::Expr),
}

enum VSubject<'x> {
    Expr(&'x n::Expr),
    Name(String, Span),
    Tuple(Vec<(String, Span)>, Span),
}

struct VClause<'x> {
    patterns: Vec<VPat<'x>>,
    guard: Option<&'x n::Expr>,
    body: &'x n::Block,
    span: Span,
}

enum VBody<'x> {
    Block(&'x n::Block),
    Case {
        subject: VSubject<'x>,
        clauses: Vec<VClause<'x>>,
        span: Span,
    },
}

struct VFn<'x> {
    params: Vec<VPat<'x>>,
    envs: Vec<&'x n::SItem>,
    ret: Option<&'x n::Type>,
    body: VBody<'x>,
    /// The one parameter is `%arg` of a clause function: it has no label.
    unlabel: bool,
}

/// The pattern that a quote pattern is in the rows before syntax three.
/// 2c: when the rows flip, this reads the datum with
/// `syn3::lisp::pattern_from_datum` and the result is resolved as any other
/// pattern of the new tree.
fn quote_pattern(q: &n::Quoted, span: Span) -> Option<o::Pattern> {
    let old = ilang_syntax::lisp::parse_pattern(&ilang_syntax::syn3::lisp::unconvert_quote(&q.0));
    old.ok().map(|p| o::Pattern {
        kind: p.kind.clone(),
        span,
    })
}

/// `{ e }` with one expression is a group (§6.10): it is `e`. A provide of
/// no service is its target.
fn ungroup(e: &n::Expr) -> &n::Expr {
    match group_inner(e) {
        Some(inner) => ungroup(inner),
        None => e,
    }
}

fn group_inner(e: &n::Expr) -> Option<&n::Expr> {
    match &e.kind {
        n::ExprKind::Block(b) => block_single(b),
        n::ExprKind::Provide { target, services } => provide_single(target, services),
        _ => None,
    }
}

fn block_single(b: &n::Block) -> Option<&n::Expr> {
    match b.stmts.as_slice() {
        [
            n::Stmt::Expr(n::Expr {
                kind: n::ExprKind::Provide { target, services },
                ..
            }),
        ] => provide_single(target, services),
        [n::Stmt::Expr(inner)] => Some(inner),
        _ => None,
    }
}

fn provide_single<'x>(target: &'x n::Expr, services: &'x n::Expr) -> Option<&'x n::Expr> {
    let provides = match &services.kind {
        n::ExprKind::Data(items) => items.iter().any(|i| matches!(i, n::DItem::Pos(_))),
        _ => false,
    };
    if provides {
        return None;
    }
    match &target.kind {
        n::ExprKind::Block(b) => block_single(b),
        n::ExprKind::Provide { target, services } => provide_single(target, services),
        _ => Some(target),
    }
}

/// `C` or `m.C`: constructors are the only values with an `UpName`.
fn is_constructor(e: &n::Expr) -> bool {
    matches!(&e.kind, n::ExprKind::Name { name, .. } if is_upper(&name.name))
}

/// The call is not a record update `C(..base, …)`.
fn plain_call(args: &[n::DItem]) -> bool {
    !matches!(args.first(), Some(n::DItem::Spread(_)))
}

fn has_hole(t: &n::Type) -> bool {
    let items = |items: &[n::TItem]| {
        items.iter().any(|i| match i {
            n::TItem::Pos(t) | n::TItem::Label(_, t) | n::TItem::Spread(t) => has_hole(t),
        })
    };
    match &t.kind {
        n::TypeKind::Hole => true,
        n::TypeKind::Lit(_) | n::TypeKind::ValueOf { .. } => false,
        n::TypeKind::Named { args, .. } | n::TypeKind::Call { args, .. } => {
            args.iter().any(has_hole)
        }
        n::TypeKind::Data(i) => items(i),
        n::TypeKind::Union(ts) | n::TypeKind::Inter(ts) | n::TypeKind::Diff(ts) => {
            ts.iter().any(has_hole)
        }
        n::TypeKind::Fn(f) => {
            has_hole(&f.param) || has_hole(&f.ret) || f.services.as_ref().is_some_and(has_hole)
        }
    }
}

fn is_macro(a: &n::Attribute) -> bool {
    a.module.is_none() && a.name.name == "macro"
}

/// The syntax macros that a module defines: the name and whether it has a
/// typed signature; or the problems of their definitions.
/// 2d: this is `lisp::expansion::definitions` on the new tree.
fn definitions(
    module: &n::Module,
    pre: &Pre,
) -> Result<Vec<(String, bool)>, Vec<(Span, &'static str)>> {
    let live: Vec<&n::Item> = module
        .items
        .iter()
        .enumerate()
        .filter(|(i, _)| !pre.member_of.contains_key(i))
        .map(|(_, item)| item)
        .collect();
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for item in &live {
        let attributes: Vec<&n::Attribute> =
            item.attributes.iter().filter(|a| is_macro(a)).collect();
        if attributes.is_empty() {
            continue;
        }
        let attr = attributes[0];
        if !pre.lisp {
            errors.push((
                attr.span.0,
                "a syntax macro must be defined in a .is file",
            ));
            continue;
        }
        if attributes.len() != 1 || attr.args.is_some() {
            errors.push((attr.span.0, "@macro is given once without arguments"));
            continue;
        }
        if let n::ItemKind::Decl { name, .. } = &item.kind {
            let implemented = live.iter().any(|other| {
                matches!(&other.kind, n::ItemKind::Bind(b)
                    if matches!(&b.pattern.kind, n::PatternKind::Var(n) if n == &name.name))
                    && other.attributes.iter().any(is_macro)
            });
            if !implemented {
                errors.push((
                    item.span.0,
                    "a typed macro signature needs an @macro implementation",
                ));
            }
            continue;
        }
        let n::ItemKind::Bind(binding) = &item.kind else {
            errors.push((item.span.0, "@macro needs a binding"));
            continue;
        };
        let name = match &binding.pattern.kind {
            n::PatternKind::Var(name) => name,
            n::PatternKind::Typed { binder, .. } if !binder.discard => &binder.name,
            _ => {
                errors.push((binding.pattern.span.0, "a macro binding needs one name"));
                continue;
            }
        };
        if pre
            .type_params
            .get(&addr(binding))
            .is_some_and(|p| !p.is_empty())
        {
            errors.push((
                binding.pattern.span.0,
                "a macro binding cannot declare type variables",
            ));
            continue;
        }
        let function = match &binding.value.kind {
            n::ExprKind::Chain(fs) if chain_kind(fs) != ChainKind::Clauses => fs,
            _ => {
                errors.push((
                    binding.value.span.0,
                    "a macro binding needs a function expression",
                ));
                continue;
            }
        };
        let mut parameters = 0;
        let mut valid = true;
        if chain_kind(function) == ChainKind::Plain {
            for p in &function[0].params {
                match &p.choices[0].kind {
                    n::PatternKind::Var(_) => parameters += 1,
                    n::PatternKind::Typed { binder, .. } if !binder.discard => parameters += 1,
                    _ => {
                        errors.push((p.choices[0].span.0, "a macro parameter must be a name"));
                        valid = false;
                    }
                }
            }
        } else {
            parameters = function[0].params.len();
        }
        let signature = live.iter().position(
            |other| matches!(&other.kind, n::ItemKind::Decl { name: declared, .. } if declared.name == *name),
        );
        if let Some(index) = signature {
            let declaration = live[index];
            if declaration
                .attributes
                .iter()
                .any(|a| a.module.is_none() && matches!(a.name.name.as_str(), "external" | "js"))
            {
                errors.push((
                    declaration.span.0,
                    "a typed macro signature cannot have @external or @js",
                ));
                valid = false;
            }
            if !declaration.attributes.iter().any(is_macro) {
                errors.push((declaration.span.0, "a typed macro signature needs @macro"));
                valid = false;
            }
            if let n::ItemKind::Decl { ty, .. } = &declaration.kind {
                if has_hole(ty) {
                    errors.push((
                        declaration.span.0,
                        "a typed macro signature must be fully written without _",
                    ));
                    valid = false;
                }
                match &ty.kind {
                    n::TypeKind::Fn(f) => {
                        let count = match &f.param.kind {
                            n::TypeKind::Data(items) => items
                                .iter()
                                .filter(|i| !matches!(i, n::TItem::Spread(_)))
                                .count(),
                            _ => 0,
                        };
                        if count != parameters {
                            errors.push((item.span.0, "a typed macro implementation must have one parameter for each signature parameter"));
                            valid = false;
                        }
                    }
                    _ => {
                        errors.push((
                            declaration.span.0,
                            "a typed macro signature must be a full function type",
                        ));
                        valid = false;
                    }
                }
            }
        }
        if valid {
            found.push((name.clone(), signature.is_some()));
        }
    }
    if errors.is_empty() {
        Ok(found)
    } else {
        Err(errors)
    }
}

impl<'a, 't> R3<'a, 't> {
    fn pre(&self) -> &'t Pre {
        &self.pre[self.cur.0 as usize]
    }

    /// The items that the resolver sees, with their index in the module:
    /// every item but the member definitions of a sum.
    fn live(&self, m: ModuleId) -> Vec<(usize, &'t n::Item)> {
        let modules = self.modules;
        let pre = self.pre;
        modules[m.0 as usize]
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| !pre[m.0 as usize].member_of.contains_key(i))
            .collect()
    }

    fn type_shape(&self, m: ModuleId, index: usize, def: &'t n::TypeDef) -> TypeShape<'t> {
        let modules = self.modules;
        let pre = &self.pre[m.0 as usize];
        let items = &modules[m.0 as usize].items;
        let params = head_params(&def.head);
        let custom = |ctors: Vec<Ctor<'t>>| TypeShape {
            alias_pattern: false,
            params: params.clone().unwrap_or_default(),
            head: &[],
            body: ShapeBody::Custom(ctors),
        };
        let mut members: Vec<usize> = pre
            .member_of
            .iter()
            .filter(|(_, sum)| **sum == index)
            .map(|(member, _)| *member)
            .collect();
        if !members.is_empty() {
            members.sort();
            return custom(
                members
                    .into_iter()
                    .map(|i| match &items[i].kind {
                        n::ItemKind::TypeDef(n::TypeDef {
                            name,
                            rhs: n::TypeRhs::Data(fields),
                            ..
                        }) => Ctor { name, fields },
                        _ => unreachable!("a member of a sum is a data definition"),
                    })
                    .collect(),
            );
        }
        match &def.rhs {
            n::TypeRhs::Opaque => custom(Vec::new()),
            n::TypeRhs::Data(fields) => custom(vec![Ctor {
                name: &def.name,
                fields,
            }]),
            n::TypeRhs::Type(t) => {
                let alias_pattern =
                    !def.head.is_empty() && matches!(t.kind, n::TypeKind::ValueOf { .. });
                let (params, head): (Vec<o::Ident>, &'t [n::Type]) = match params {
                    Some(params) if !alias_pattern => (params, &[]),
                    _ => (Vec::new(), &def.head),
                };
                TypeShape {
                    alias_pattern,
                    params,
                    head,
                    body: ShapeBody::Alias(t),
                }
            }
        }
    }

    // ---- Phase 1: top-level definitions ----

    fn collect(&mut self, m: ModuleId) {
        self.cur = m;
        let mi = m.0 as usize;
        let parsed = self.parsed;
        let modules = self.modules;
        let ast = &modules[mi];
        let pre = self.pre();
        let found = definitions(ast, pre);
        if let Err(errors) = &found {
            for (span, message) in errors {
                self.error(code::ATTRIBUTE_PLACE, *span, *message);
            }
        }
        let placeholder_var = VarId(u32::MAX);
        let typed: HashSet<String> = found
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, signature)| *signature)
            .map(|(name, _)| name)
            .collect();
        for (index, item) in self.live(m) {
            let typed_declaration = matches!(&item.kind, n::ItemKind::Decl { name, .. } if typed.contains(&name.name));
            if item.attributes.iter().any(is_macro) && !typed_declaration {
                let mut remaining: Vec<&n::Attribute> =
                    item.attributes.iter().filter(|a| !is_macro(a)).collect();
                for attribute in &remaining {
                    if attribute.module.is_none() && attribute.name.name == "main" {
                        self.error(
                            code::ATTRIBUTE_PLACE,
                            attribute.span.0,
                            "`@main` cannot be put on a macro",
                        );
                    }
                }
                remaining
                    .retain(|attribute| attribute.module.is_some() || attribute.name.name != "main");
                self.attributes(item, &remaining);
                if let n::ItemKind::Bind(binding) = &item.kind {
                    let mut names = Vec::new();
                    self.binders(&binding.pattern, &mut names);
                    for (name, span) in names {
                        if !typed.contains(&name) && self.value_free(m, &name, span) {
                            self.macros_by_name[mi].insert(name, false);
                        }
                    }
                }
                self.slots[mi].push(Slot::None);
                continue;
            }
            let mut attrs = if typed_declaration {
                Attrs {
                    external: Some(("ilang:typed-macro".into(), "".into())),
                    ..Attrs::default()
                }
            } else {
                let all: Vec<&n::Attribute> = item.attributes.iter().collect();
                self.attributes(item, &all)
            };
            let here = self.p.module(m);
            let root = self.graph.root == Some(here.package);
            if root && let n::ItemKind::Bind(b) = &item.kind {
                let mut names = Vec::new();
                self.binders(&b.pattern, &mut names);
                let pkg = &self.graph.packages[here.package];
                let default = here.kind == ModuleKind::Src
                    && here.path == format!("{}/main", pkg.name)
                    && names.iter().any(|(n, span)| {
                        n == "main"
                            && self.name_key(n, *span).scope.marks.is_empty()
                            && self.name_key(n, *span).scope.module == parsed[mi].source.0
                    });
                let declared = pkg
                    .entries
                    .iter()
                    .filter(|(_, e)| {
                        e.module == here.path
                            && names.iter().any(|(n, span)| {
                                *n == e.function
                                    && self.name_key(n, *span).scope.marks.is_empty()
                                    && self.name_key(n, *span).scope.module == parsed[mi].source.0
                            })
                    })
                    .collect::<Vec<_>>();
                let selected = match &self.graph.entry {
                    Some(name) => declared.iter().any(|(n, _)| n == name),
                    None => default,
                };
                if default || !declared.is_empty() {
                    self.entry_form(b.pattern.span.0, b);
                }
                if self.graph.entry.is_some() {
                    attrs.main = selected;
                } else if selected {
                    attrs.main = true;
                }
            }
            let slot = match &item.kind {
                n::ItemKind::TypeDef(def) => {
                    let shape = self.type_shape(m, index, def);
                    if shape.alias_pattern {
                        let pid = ProjectionId(self.p.alias_patterns.len() as u32);
                        let mut binders = Vec::new();
                        for part in shape.head {
                            self.collect_pattern_binders(m, part, &attrs, &mut binders);
                        }
                        self.p.alias_patterns.push(AliasPattern {
                            heads: Vec::new(),
                            head_span: def.name.span.0,
                            module: m,
                            span: item.span.0,
                            source: Type {
                                kind: TypeKind::Error,
                                span: item.span.0,
                            },
                            pattern: TypePattern::Error,
                            binders,
                        });
                        Slot::Pattern(pid)
                    } else {
                        let body = match &shape.body {
                            ShapeBody::Alias(t) => Err(t.span.0),
                            ShapeBody::Custom(ctors) => {
                                Ok(ctors.iter().map(|c| id(c.name)).collect())
                            }
                        };
                        Slot::Type(self.collect_type(m, &id(&def.name), &shape.params, body, &attrs))
                    }
                }
                n::ItemKind::Bind(b) => {
                    let index = self.p.module(m).bindings.len() as u32;
                    let mut names = Vec::new();
                    self.binders(&b.pattern, &mut names);
                    let mut seen = HashSet::new();
                    let mut defs = Vec::new();
                    for (name, span) in names {
                        if !seen.insert(self.name_key(&name, span)) {
                            continue; // reported when the pattern is resolved
                        }
                        let kind = DefKind::Binding {
                            binding: index,
                            var: placeholder_var,
                        };
                        let before = self.p.module(m).defs.len();
                        self.add_def(m, &name, span, kind, &attrs);
                        if self.p.module(m).defs.len() > before {
                            defs.push(DefId {
                                module: m,
                                index: before as u32,
                            });
                        }
                    }
                    self.p.modules[mi].bindings.push(TopBinding {
                        binding: Binding {
                            pattern: Pattern {
                                kind: PatternKind::Error(Vec::new()),
                                span: b.pattern.span.0,
                            },
                            value: Expr {
                                kind: ExprKind::Error,
                                span: b.value.span.0,
                            },
                            direct: false,
                            ty_vars: Vec::new(),
                            span: b.span.0,
                        },
                        defs,
                        main: attrs.main,
                    });
                    Slot::Binding(index)
                }
                n::ItemKind::Macro { name, args, .. } => {
                    let message = if args.is_empty()
                        && matches!(name.name.as_str(), "import" | "export")
                    {
                        "a call at the top level is kept for macros; if this was an old empty header, omit it"
                    } else {
                        "a call at the top level is kept for macros"
                    };
                    self.error(code::MACRO_RESERVED, item.span.0, message);
                    Slot::None
                }
                n::ItemKind::Decl { name, ty } => {
                    let (module, function) = attrs.external.clone().unwrap_or_default();
                    let kind = DefKind::External {
                        module,
                        function,
                        ty: Type {
                            kind: TypeKind::Error,
                            span: ty.span.0,
                        },
                        ty_vars: Vec::new(),
                    };
                    let before = self.p.module(m).defs.len();
                    self.add_def(m, &name.name, name.span.0, kind, &attrs);
                    if self.p.module(m).defs.len() > before {
                        if typed_declaration {
                            self.p.typed_macros.insert(DefId {
                                module: m,
                                index: before as u32,
                            });
                        }
                        Slot::Decl(before as u32)
                    } else {
                        Slot::None
                    }
                }
            };
            self.slots[mi].push(slot);
        }
        // An export of a member of a sum alone is part of the export of
        // the sum.
        let exports: Vec<o::ExportItem> = ast
            .exports
            .iter()
            .filter(|e| e.constructors || !pre.absorbed.contains(&e.name.name))
            .map(|e| o::ExportItem {
                name: id(&e.name),
                constructors: e.constructors,
                span: e.span.0,
            })
            .collect();
        self.exports(m, &exports);
    }

    fn collect_pattern_binders(
        &mut self,
        m: ModuleId,
        t: &n::Type,
        attrs: &Attrs,
        out: &mut Vec<TypeId>,
    ) {
        match &t.kind {
            n::TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => {
                out.push(self.collect_type(m, &id(name), &[], Err(t.span.0), attrs));
            }
            n::TypeKind::Named { args, .. } => {
                for t in args {
                    self.collect_pattern_binders(m, t, attrs, out);
                }
            }
            n::TypeKind::Data(items)
                if !items.is_empty() && items.iter().all(|i| matches!(i, n::TItem::Pos(_))) =>
            {
                for i in items {
                    if let n::TItem::Pos(t) = i {
                        self.collect_pattern_binders(m, t, attrs, out);
                    }
                }
            }
            _ => {}
        }
    }

    /// `body` is the span of the right side of an alias, or the names of
    /// the constructors of a custom type.
    fn collect_type(
        &mut self,
        m: ModuleId,
        name: &o::Ident,
        def_params: &[o::Ident],
        body: Result<Vec<o::Ident>, Span>,
        attrs: &Attrs,
    ) -> TypeId {
        let id = TypeId(self.p.types.len() as u32);
        let mi = m.0 as usize;
        let key = self.name_key(&name.name, name.span);
        self.check_binder(&name.name, name.span, true, false);
        let occupied = if key.scope.marks.is_empty() {
            self.types_by_name[mi].contains_key(&name.name)
        } else {
            self.generated_types[mi].contains_key(&key)
        };
        if occupied {
            self.error(
                code::DEFINED_TWICE,
                name.span,
                format!(
                    "the type `{}` is defined more than once in this module",
                    name.name
                ),
            );
        } else if key.scope.marks.is_empty() {
            self.types_by_name[mi].insert(name.name.clone(), id);
        } else {
            self.generated_types[mi].insert(key, id);
        }
        // A repeated parameter is reported, but it keeps its place, so that
        // the parameters stay in line with the written ones and the number
        // of parameters is the written number. Its name refers to the first.
        let mut params: Vec<(String, TyVarId)> = Vec::new();
        for p in def_params {
            self.check_binder(&p.name, p.span, false, true);
            if def_params
                .iter()
                .take(params.len())
                .any(|q| self.name_key(&q.name, q.span) == self.name_key(&p.name, p.span))
            {
                self.error(
                    code::TYPE_PARAM_TWICE,
                    p.span,
                    format!("the type parameter `{}` is given more than once", p.name),
                );
            }
            let v = self.new_ty_var(&p.name);
            params.push((p.name.clone(), v));
        }
        let body = match body {
            Err(span) => TypeBody::Alias(Type {
                kind: TypeKind::Error,
                span,
            }),
            Ok(ctors) => {
                let mut out = Vec::new();
                for c in &ctors {
                    if !self.value_free(m, &c.name, c.span) {
                        continue;
                    }
                    let index = out.len() as u32;
                    let ctor = CtorRef { ty: id, index };
                    let key = self.name_key(&c.name, c.span);
                    if key.scope.marks.is_empty() {
                        self.ctors_by_name[mi].insert(c.name.clone(), ctor);
                    } else {
                        self.generated_values[mi].insert(key, Entry::Ctor(ctor));
                    }
                    out.push(CtorDef {
                        name: c.name.clone(),
                        span: c.span,
                        fields: Vec::new(),
                    });
                }
                TypeBody::Custom(out)
            }
        };
        self.p.types.push(TypeDef {
            name: name.name.clone(),
            span: name.span,
            module: m,
            params,
            body,
            exported: false,
            constructors_exported: false,
            host: attrs.host.clone(),
            deprecated: attrs.deprecated.clone(),
        });
        self.p.modules[mi].types.push(id);
        id
    }

    /// §2.2 and §10: the attributes `list` of an item.
    fn attributes(&mut self, item: &n::Item, list: &[&n::Attribute]) -> Attrs {
        let mut out = Attrs::default();
        let mut seen = HashSet::new();
        for a in list {
            let name = a.name.name.as_str();
            let written = a
                .module
                .as_ref()
                .map_or_else(|| format!("@{name}"), |m| format!("@{}.{name}", m.name));
            if a.module.is_some() || !matches!(name, "external" | "host" | "deprecated") {
                self.error(code::UNKNOWN_ATTRIBUTE, a.span.0, format!("the attribute `{written}` is kept for macros; the built-in attributes are `@external`, `@host`, and `@deprecated`"));
                continue;
            }
            if !seen.insert(name) {
                self.error(
                    code::ATTRIBUTE_TWICE,
                    a.span.0,
                    format!("`@{name}` is given more than once on this item"),
                );
                continue;
            }
            let strings = attribute_strings(a).unwrap_or_default();
            let args: Vec<&str> = strings.iter().map(|(s, _)| s.as_str()).collect();
            let (counts, place_ok, place): (&[usize], bool, &str) = match name {
                "external" => (
                    &[2],
                    matches!(item.kind, n::ItemKind::Decl { .. }),
                    "a declaration `name: Type`",
                ),
                "host" => (
                    &[1, 2],
                    matches!(item.kind, n::ItemKind::TypeDef(_)),
                    "a type definition",
                ),
                "deprecated" => (&[1], true, "any item"),
                _ => unreachable!(),
            };
            let form = match name {
                "external" => r#"`@external("module", "function")`"#,
                "host" => r#"`@host("namespace")` or `@host("namespace", "async")`"#,
                _ => r#"`@deprecated("note")`"#,
            };
            if attribute_call(a) {
                let message = format!(
                    "the arguments of `@{name}` are string literals given by position: {form}"
                );
                self.error(code::ATTRIBUTE_ARGS, a.span.0, message);
                continue;
            }
            if !counts.contains(&args.len())
                || (name == "host" && args.len() == 2 && !matches!(args[1], "async" | "request"))
            {
                self.error(
                    code::ATTRIBUTE_ARGS,
                    a.span.0,
                    format!("this attribute is written {form}"),
                );
                continue;
            }
            if !place_ok {
                self.error(
                    code::ATTRIBUTE_PLACE,
                    a.span.0,
                    format!("`@{name}` may only be on {place}"),
                );
                continue;
            }
            match name {
                "external" | "host" => {
                    if (args[0].starts_with("ilang:") || args[0].starts_with("i:compiler/"))
                        && !self.in_library()
                    {
                        self.error(
                            code::RESERVED_ILANG,
                            strings[0].1,
                            if args[0].starts_with("ilang:") {
                                "names that start with `ilang:` are kept for the runtime"
                            } else {
                                "names that start with `i:compiler/` are kept for the compiler"
                            },
                        );
                    }
                    if name == "external" {
                        out.external = Some((args[0].to_string(), args[1].to_string()));
                    } else {
                        out.host = Some(Host {
                            namespace: args[0].to_string(),
                            is_async: args.len() == 2,
                            is_request: args.get(1).is_some_and(|m| *m == "request"),
                        });
                    }
                }
                _ => out.deprecated = Some(args[0].to_string()),
            }
        }
        if let n::ItemKind::Decl { name, .. } = &item.kind
            && out.external.is_none()
            && !list
                .iter()
                .any(|a| a.module.is_none() && a.name.name == "external")
        {
            self.error(
                code::DECLARATION_WITHOUT_EXTERNAL,
                name.span.0,
                "a declaration with no `@external` is kept for macros",
            );
        }
        out
    }

    /// Entries bind one name to a function; compiled entries take no value parameters.
    fn entry_form(&mut self, span: Span, b: &n::Binding) -> bool {
        let mut names = Vec::new();
        self.binders(&b.pattern, &mut names);
        let written = "a program entry";
        let function = match &ungroup(&b.value).kind {
            n::ExprKind::Chain(fs) if chain_kind(fs) != ChainKind::Clauses => fs,
            _ => {
                self.error(
                    code::ENTRY_FORM,
                    span,
                    format!("{written} is on a binding whose value is a function expression"),
                );
                return false;
            }
        };
        if names.len() != 1 {
            self.error(
                code::ENTRY_FORM,
                span,
                format!("{written} is on a binding whose pattern binds one name"),
            );
            return false;
        }
        if !function[0].params.is_empty() {
            self.error(
                code::ENTRY_PARAMS,
                span,
                format!("an entry point has no parameters but environment parameters, so {written} may not be on this function"),
            );
            return false;
        }
        true
    }

    // ---- Phase 2: imports (§2.3) ----

    fn imports(&mut self, m: ModuleId) {
        self.cur = m;
        let mi = m.0 as usize;
        let mut scope = Scope::default();
        for (name, &d) in &self.defs_by_name[mi] {
            let entry = Entry::Def(DefId {
                module: m,
                index: d,
            });
            scope.values.insert(name.clone(), (entry, false));
        }
        for (name, &exported) in &self.macros_by_name[mi] {
            scope
                .values
                .insert(name.clone(), (Entry::Macro { exported }, false));
        }
        for (name, &c) in &self.ctors_by_name[mi] {
            scope.values.insert(name.clone(), (Entry::Ctor(c), false));
        }
        for (name, &t) in &self.types_by_name[mi] {
            scope
                .types
                .insert(name.clone(), (TypeRef::Custom(t), false));
        }
        let modules = self.modules;
        for item in &modules[mi].imports {
            let item = o::ImportItem {
                alias: opt_id(&item.alias),
                names: item.names.iter().map(id).collect(),
                path: item.path.iter().map(id).collect(),
                span: item.span.0,
            };
            let alias = item.alias.as_ref().unwrap_or(item.path.last().unwrap());
            let Some(target) = self.import_target(&item) else {
                for n in item.names.iter().chain([alias]) {
                    scope
                        .values
                        .entry(n.name.clone())
                        .or_insert((Entry::Broken, true));
                    if is_upper(&n.name) {
                        scope.broken_types.insert(n.name.clone());
                    }
                }
                continue;
            };
            self.p.modules[mi].imports.push(target);
            self.p.modules[mi]
                .import_names
                .push((alias.name.clone(), target));
            self.put_value(&mut scope, &alias.name, alias.span, Entry::Module(target));
            for n in &item.names {
                self.import_name(&mut scope, target, n);
            }
        }
        self.scopes[mi] = scope;
    }

    // ---- Phase 3: type definitions (§4.3) ----

    fn type_defs(&mut self, m: ModuleId) {
        self.cur = m;
        let slots = self.slots[m.0 as usize].clone();
        for ((index, item), slot) in self.live(m).into_iter().zip(slots) {
            let n::ItemKind::TypeDef(def) = &item.kind else {
                continue;
            };
            let shape = self.type_shape(m, index, def);
            if let Slot::Pattern(pid) = slot {
                self.resolve_alias_pattern(def, &shape, pid);
                continue;
            }
            let Slot::Type(id) = slot else {
                continue;
            };
            // §4.3: read a reserved head in full, but do not resolve its body.
            if !shape.head.is_empty() {
                self.error(
                    code::TYPE_PARAM_FORM,
                    shape.head[0].span.0,
                    "a type parameter is an `UpName` with no module name and no arguments",
                );
                if shape
                    .head
                    .iter()
                    .any(|t| matches!(t.kind, n::TypeKind::ValueOf { module: None, .. }))
                {
                    self.diags.last_mut().unwrap().message.push_str("; old lowercase type parameters are now declared as UpNames, as in `Box(A) { Box(A) }` or `Id(A) = A`");
                }
                continue;
            }
            let mut clashes = false;
            for param in &shape.params {
                let key = self.name_key(&param.name, param.span);
                if is_upper(&param.name) && self.conflicts_with_type(&key) {
                    clashes = true;
                    self.error(
                        code::TYPE_VAR_IS_TYPE,
                        param.span,
                        "a type parameter may not have the name of a type",
                    );
                }
            }
            if clashes {
                continue;
            }
            let params = self.p.ty(id).params.clone();
            let mut by_name = HashMap::new();
            for ((n, v), ident) in params.iter().zip(&shape.params) {
                by_name.entry(self.name_key(n, ident.span)).or_insert(*v);
            }
            self.ty_def = Some(TyDefMode {
                params: by_name,
                used: HashSet::new(),
            });
            match &shape.body {
                ShapeBody::Alias(t) => {
                    let t = self.ty(t);
                    self.p.types[id.0 as usize].body = TypeBody::Alias(t);
                }
                ShapeBody::Custom(ctors) => {
                    for c in ctors {
                        let fields: Vec<FieldDef> = c
                            .fields
                            .iter()
                            .map(|f| match f {
                                n::TItem::Pos(t) | n::TItem::Spread(t) => FieldDef {
                                    label: None,
                                    ty: self.ty(t),
                                },
                                n::TItem::Label(l, t) => FieldDef {
                                    label: Some(pre::id(l)),
                                    ty: self.ty(t),
                                },
                            })
                            .collect();
                        if let TypeBody::Custom(defs) = &mut self.p.types[id.0 as usize].body
                            && let Some(d) = defs.iter_mut().find(|d| d.span == c.name.span.0)
                        {
                            d.fields = fields;
                        }
                    }
                }
            }
            let mode = self.ty_def.take().unwrap();
            for (i, (p, ident)) in params.iter().zip(&shape.params).enumerate() {
                let repeated = shape.params[..i].iter().any(|q| {
                    self.name_key(&q.name, q.span) == self.name_key(&ident.name, ident.span)
                });
                if !repeated && !mode.used.contains(&p.1) {
                    self.error(
                        code::TYPE_PARAM_UNUSED,
                        ident.span,
                        format!("the type parameter `{}` is not used", p.0),
                    );
                }
            }
        }
    }

    /// §9.7: the form of the host services.
    fn host_forms(&mut self) {
        for mi in 0..self.parsed.len() {
            let m = ModuleId(mi as u32);
            self.cur = m;
            let slots = self.slots[mi].clone();
            for ((index, item), slot) in self.live(m).into_iter().zip(slots) {
                if let (n::ItemKind::TypeDef(def), Slot::Type(id)) = (&item.kind, slot)
                    && self.p.ty(id).host.is_some()
                    && !matches!(&self.p.ty(id).body, TypeBody::Alias(t) if value_dependent(&self.p,t))
                    && !matches!(&self.p.ty(id).body,TypeBody::Custom(cs) if cs.iter().flat_map(|c|&c.fields).any(|f|value_dependent(&self.p,&f.ty)))
                {
                    let shape = self.type_shape(m, index, def);
                    let ok = match &shape.body {
                        ShapeBody::Custom(ctors) if ctors.len() == 1 => {
                            ctors[0].fields.iter().all(|f| {
                                matches!(f, n::TItem::Label(_, t) if matches!(t.kind, n::TypeKind::Fn(_)))
                            })
                        }
                        _ => false,
                    };
                    if !ok {
                        self.error(
                            code::HOST_FORM,
                            def.name.span.0,
                            "a host service has exactly one constructor, and every field has a label and a function type (§9.7)",
                        );
                    }
                }
            }
        }
    }

    fn resolve_alias_pattern(&mut self, def: &n::TypeDef, shape: &TypeShape, pid: ProjectionId) {
        let ids = self.p.alias_patterns[pid.0 as usize].binders.clone();
        for id in &ids {
            let name = self.p.ty(*id).name.clone();
            let conflict = self.scopes[self.cur.0 as usize]
                .types
                .get(&name)
                .is_some_and(|(_, imported)| *imported)
                || self.builtin_types.contains_key(&name);
            if conflict {
                let at = self.p.ty(*id).span;
                self.error(
                    code::TYPE_PATTERN_BINDER_IS_TYPE,
                    at,
                    format!("the new alias `{name}` conflicts with an imported or built-in type"),
                );
            }
        }
        let ShapeBody::Alias(source) = &shape.body else {
            unreachable!()
        };
        let mut parts = Vec::new();
        let mut heads = Vec::new();
        for t in shape.head {
            parts.push(self.type_pattern(t, &ids, &mut heads));
        }
        let name = id(&def.name);
        let head = self.lookup_type(None, &name);
        let pattern = match head {
            Some(ty) if self.arity(ty) == parts.len() => {
                heads.push((name.span, ty));
                TypePattern::Named(ty, parts)
            }
            Some(_) => {
                self.error(
                    code::TYPE_ARITY,
                    name.span,
                    "alias pattern head has the wrong number of arguments",
                );
                TypePattern::Error
            }
            None => TypePattern::Error,
        };
        self.p.alias_patterns[pid.0 as usize].heads = heads;
        self.p.alias_patterns[pid.0 as usize].pattern = pattern;
        let source = self.ty(source);
        self.p.alias_patterns[pid.0 as usize].source = source.clone();
        let TypeKind::Of { value, site, .. } = source.kind else {
            return;
        };
        self.p.of_sites[site.0 as usize].permanent = true;
        self.p.of_sites[site.0 as usize].projection = Some(pid);
        for id in &ids {
            let span = self.p.ty(*id).span;
            let t = self.new_of(value, span, true, Some(pid));
            self.p.types[id.0 as usize].body = TypeBody::Alias(t);
        }
        let slot = &mut self.p.alias_patterns[pid.0 as usize];
        slot.source = source;
    }

    fn type_pattern(
        &mut self,
        t: &n::Type,
        ids: &[TypeId],
        heads: &mut Vec<(Span, TypeRef)>,
    ) -> TypePattern {
        match &t.kind {
            n::TypeKind::Hole => TypePattern::Skip,
            n::TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => ids
                .iter()
                .copied()
                .find(|id| self.p.ty(*id).span == name.span.0)
                .map_or(TypePattern::Error, TypePattern::Bind),
            n::TypeKind::Named { module, name, args } => {
                let parts = args
                    .iter()
                    .map(|t| self.type_pattern(t, ids, heads))
                    .collect::<Vec<_>>();
                match self.lookup_type(opt_id(module).as_ref(), &id(name)) {
                    Some(ty) if self.arity(ty) == parts.len() => {
                        heads.push((name.span.0, ty));
                        TypePattern::Named(ty, parts)
                    }
                    Some(_) => {
                        self.error(
                            code::TYPE_ARITY,
                            t.span.0,
                            "alias pattern has the wrong number of arguments",
                        );
                        TypePattern::Error
                    }
                    None => TypePattern::Error,
                }
            }
            n::TypeKind::Data(items)
                if !items.is_empty() && items.iter().all(|i| matches!(i, n::TItem::Pos(_))) =>
            {
                TypePattern::Tuple(
                    items
                        .iter()
                        .map(|i| match i {
                            n::TItem::Pos(t) => self.type_pattern(t, ids, heads),
                            _ => unreachable!(),
                        })
                        .collect(),
                )
            }
            _ => {
                self.error(
                    code::TYPE_PATTERN_FORM,
                    t.span.0,
                    "a type pattern is a binder, `_`, a named type, or a tuple",
                );
                TypePattern::Error
            }
        }
    }

    // ---- Types ----

    fn ty(&mut self, t: &n::Type) -> Type {
        use n::TypeKind as T;
        let span = t.span.0;
        if let Some(origin) = self.parsed[self.cur.0 as usize]
            .scoped
            .as_ref()
            .and_then(|s| s.origin(span))
        {
            let handle = match origin.syntax.node() {
                ilang_syntax::lisp::syntax::Node::TypeView(h) => Some(h.index),
                _ => match origin.syntax.scope_identity() {
                    Some(ScopeIdentity::TypeVariable { variable, .. }) => Some(*variable as u32),
                    _ => None,
                },
            };
            if let Some(index) = handle {
                if self.checked_inputs.is_none() {
                    self.error(code::UNKNOWN_TYPE,span,"a frozen type handle is valid only during its module check's typed expansion");
                    return Type {
                        kind: TypeKind::Error,
                        span,
                    };
                }
                return Type {
                    kind: TypeKind::Var(TyVarId(0x8000_0000 | index)),
                    span,
                };
            }
        }
        let kind = match &t.kind {
            T::Lit(v) => TypeKind::Lit(v.clone()),
            T::Named { module, name, args } => {
                let key = self.name_key(&name.name, name.span.0);
                let variable = if module.is_none() {
                    let r = &mut self.r;
                    r.ty_frames
                        .iter()
                        .rev()
                        .find_map(|f| f.names.get(&key).copied())
                        .or_else(|| {
                            r.ty_def.as_mut().and_then(|mode| {
                                let var = mode.params.get(&key).copied()?;
                                mode.used.insert(var);
                                Some(var)
                            })
                        })
                } else {
                    None
                };
                if let Some(var) = variable {
                    let kind = if args.is_empty() {
                        TypeKind::Var(var)
                    } else {
                        self.error(
                            code::TYPE_VAR_ARGS,
                            span,
                            "a type variable takes no arguments",
                        );
                        TypeKind::Error
                    };
                    return Type { kind, span };
                }
                let before = self.diags.len();
                let found = self.lookup_type(opt_id(module).as_ref(), &id(name));
                if found.is_none() && module.is_none() && args.is_empty() {
                    let hint = if self.ty_def.is_some() {
                        "to use a type parameter, declare it in the head, as in `Box(A) { … }`"
                    } else {
                        "to use a type variable, declare it after the name of the binding, as in `f(A) = …`"
                    };
                    for d in &mut self.diags[before..] {
                        if d.code == code::UNKNOWN_TYPE {
                            d.message.push_str("; ");
                            d.message.push_str(hint);
                        }
                    }
                }
                let args: Vec<Type> = args.iter().map(|a| self.ty(a)).collect();
                match found {
                    Some(ty) if self.arity(ty) != args.len() => {
                        let want = self.arity(ty);
                        let tname = self.type_name(ty);
                        self.error(
                            code::TYPE_ARITY,
                            span,
                            format!(
                                "the type `{tname}` has {want} type parameter{}, but {} {} given",
                                if want == 1 { "" } else { "s" },
                                args.len(),
                                if args.len() == 1 { "is" } else { "are" }
                            ),
                        );
                        TypeKind::Error
                    }
                    Some(ty) => TypeKind::Named { ty, args },
                    None => TypeKind::Error,
                }
            }
            T::ValueOf { module, name } => {
                self.value_type(opt_id(module).as_ref(), &id(name), span)
            }
            T::Call { .. } => {
                self.error(
                    code::MACRO_RESERVED,
                    span,
                    "a call in a type is kept for macros",
                );
                TypeKind::Error
            }
            T::Hole => TypeKind::Hole,
            T::Data(items) => {
                if !items.is_empty() && items.iter().all(|i| matches!(i, n::TItem::Pos(_))) {
                    TypeKind::Tuple(
                        items
                            .iter()
                            .map(|i| match i {
                                n::TItem::Pos(t) => self.ty(t),
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                } else {
                    // Data with labels: only the labelled items (the others
                    // are an error of this stage).
                    let fields: Vec<(&n::Ident, &n::Type)> = items
                        .iter()
                        .filter_map(|i| match i {
                            n::TItem::Label(l, t) => Some((l, t)),
                            _ => None,
                        })
                        .collect();
                    let mut labels = HashSet::new();
                    for (label, _) in &fields {
                        if !labels.insert(&label.name) {
                            self.error(
                                code::DUPLICATE_RECORD_LABEL,
                                label.span.0,
                                format!("the record label `{}` is repeated", label.name),
                            );
                        }
                    }
                    TypeKind::Record {
                        base: None,
                        fields: fields
                            .into_iter()
                            .map(|(label, ty)| {
                                (
                                    RecordLabel {
                                        ident: id(label),
                                        parameter: None,
                                    },
                                    self.ty(ty),
                                )
                            })
                            .collect(),
                        removed: Vec::new(),
                    }
                }
            }
            T::Fn(f) => self.fn_type(f, span),
            T::Union(ts) => TypeKind::Union(ts.iter().map(|x| self.ty(x)).collect()),
            T::Inter(ts) => TypeKind::Inter(ts.iter().map(|x| self.ty(x)).collect()),
            T::Diff(ts) => TypeKind::Diff(ts.iter().map(|x| self.ty(x)).collect()),
        };
        Type { kind, span }
    }

    fn fn_type(&mut self, f: &n::FnType, span: Span) -> TypeKind {
        let mut params = Vec::new();
        if let n::TypeKind::Data(items) = &f.param.kind {
            for item in items {
                match item {
                    n::TItem::Pos(t) => params.push(FnParamType {
                        label: None,
                        ty: self.ty(t),
                    }),
                    n::TItem::Label(l, t) => {
                        if crate::syn3_facts::on() {
                            crate::syn3_facts::record(crate::syn3_facts::Fact::FnTypeLabel {
                                file: crate::syn3_facts::File::of(
                                    &self.graph.sources,
                                    self.p.module(self.cur).source,
                                ),
                                label: l.span.0,
                                ty: t.span.0,
                            });
                        }
                        params.push(FnParamType {
                            label: Some(id(l)),
                            ty: self.ty(t),
                        })
                    }
                    n::TItem::Spread(_) => {}
                }
            }
        }
        let ret = Box::new(self.ty(&f.ret));
        let (err, env) = self.needs_row(f, span);
        TypeKind::Fn {
            params,
            ret,
            err: err.map(Box::new),
            env: env.map(Box::new),
        }
    }

    /// The needs written after `<-` as the two rows of today's function
    /// type: the error channel and the environment. This is the only place
    /// where a need becomes a row: a `Fail(E)` entry is the error `E`, every
    /// other entry is a member of the environment, and `..N` (or `<- N`) is
    /// the row variable `N`, which also carries the errors when the
    /// signature writes no `Fail` beside it (§7.3): its error is then the
    /// type parameter `N%E` of the binding ([`Pre::row_err`]).
    fn needs_row(&mut self, f: &n::FnType, span: Span) -> (Option<Type>, Option<Type>) {
        fn need<'x>(
            t: &'x n::Type,
            spread: bool,
            err: &mut Option<&'x n::Type>,
            env: &mut Vec<&'x n::Type>,
        ) {
            if let n::TypeKind::Named {
                module: None,
                name,
                args,
            } = &t.kind
                && name.name == "Fail"
                && args.len() == 1
                && !spread
            {
                *err = Some(&args[0]);
                return;
            }
            env.push(t);
        }
        let mut err: Option<&n::Type> = None;
        let mut env: Vec<&n::Type> = Vec::new();
        match f.services.as_ref().map(|s| (&s.kind, s)) {
            None => {}
            Some((n::TypeKind::Data(items), _)) => {
                for item in items {
                    match item {
                        n::TItem::Pos(t) => need(t, false, &mut err, &mut env),
                        n::TItem::Spread(t) => need(t, true, &mut err, &mut env),
                        n::TItem::Label(..) => {}
                    }
                }
            }
            // `<- N`: a row variable.
            Some((_, t)) => need(t, true, &mut err, &mut env),
        }
        let err = match err {
            Some(t) => Some(self.ty(t)),
            // The errors that a row variable carries (§7.3).
            None => self.pre().row_err.get(&addr(f)).map(|row| {
                let carried = n::Type::new(
                    n::TypeKind::Named {
                        module: None,
                        name: n::Ident::new(format!("{row}%E"), span),
                        args: Vec::new(),
                    },
                    span,
                );
                self.ty(&carried)
            }),
        };
        let env = match env.len() {
            0 => None,
            1 => Some(self.ty(env[0])),
            _ => Some(Type {
                kind: TypeKind::Union(env.iter().map(|t| self.ty(t)).collect()),
                span,
            }),
        };
        (err, env)
    }

    // ---- Phase 4: values ----

    fn values(&mut self, m: ModuleId) {
        self.cur = m;
        let slots = self.slots[m.0 as usize].clone();
        for ((_, item), slot) in self.live(m).into_iter().zip(slots) {
            match (&item.kind, slot) {
                (n::ItemKind::Decl { ty, .. }, Slot::Decl(d)) => {
                    let type_params: Vec<o::Ident> = match &ty.kind {
                        n::TypeKind::Fn(f) => f.generics.iter().map(id).collect(),
                        _ => Vec::new(),
                    };
                    let vars = self.push_declared_frame(&type_params);
                    let t = self.ty(ty);
                    self.ty_frames.pop();
                    // Intrinsic library operations are not host imports (§2.1).
                    let intrinsic = self.p.typed_macros.contains(&DefId {
                        module: m,
                        index: d,
                    }) || self.in_library()
                        && matches!(&self.p.modules[m.0 as usize].defs[d as usize].kind,
                            DefKind::External { module, .. } if module.starts_with("ilang:"));
                    if !intrinsic {
                        let before = self.diags.len();
                        self.external_type(&t);
                        if self.diags.len() == before
                            && let Some(f) = self.host_fn(&t, false)
                        {
                            self.p.external_fns.insert(
                                DefId {
                                    module: m,
                                    index: d,
                                },
                                f,
                            );
                        }
                    }
                    if let DefKind::External {
                        ty: slot_ty,
                        ty_vars,
                        ..
                    } = &mut self.p.modules[m.0 as usize].defs[d as usize].kind
                    {
                        *slot_ty = t;
                        *ty_vars = vars;
                    }
                }
                (n::ItemKind::Bind(b), Slot::Binding(index)) => {
                    let (binding, binders) = self.binding(b, true);
                    let mi = m.0 as usize;
                    for binder in binders {
                        let def = self.p.modules[mi].bindings[index as usize]
                            .defs
                            .iter()
                            .find(|d| self.p.def(**d).span == binder.span)
                            .map(|d| d.index);
                        if let Some(d) = def
                            && let DefKind::Binding { binding: bi, var } =
                                &mut self.p.modules[mi].defs[d as usize].kind
                            && *bi == index
                        {
                            *var = binder.var;
                        }
                    }
                    self.p.modules[mi].bindings[index as usize].binding = binding;
                }
                _ => {}
            }
        }
    }

    /// A binding, at the top level or in a block. Gives back the binders of
    /// its pattern; in a block, the caller makes them visible from the next
    /// statement (§6.10).
    fn binding(&mut self, b: &n::Binding, top: bool) -> (Binding, Vec<Binder>) {
        let type_params = self
            .pre()
            .type_params
            .get(&addr(b))
            .cloned()
            .unwrap_or_default();
        self.binding_of(
            &VPat::Pat(&b.pattern),
            &b.value,
            b.span.0,
            &type_params,
            top,
        )
    }

    fn binding_of(
        &mut self,
        pattern: &VPat,
        value: &n::Expr,
        span: Span,
        type_params: &[o::Ident],
        top: bool,
    ) -> (Binding, Vec<Binder>) {
        let ty_vars = self.push_declared_frame(type_params);
        let direct = self.is_direct(value);
        let mut binders = Vec::new();
        let mut names = Vec::new();
        self.vpat_binders(pattern, &mut names);
        let names: HashSet<NameKey> = names
            .into_iter()
            .map(|(n, span)| self.name_key(&n, span))
            .collect();
        self.pending.push(names.clone());
        let pattern = self.vpat(pattern, &mut binders, None);
        self.pending.pop();
        let value = if direct && !top {
            // §6.10: the names may be used inside the function bodies of a
            // direct value.
            let frame = Frame {
                names: binders
                    .iter()
                    .map(|b| (self.name_key(&b.name, b.span), b.var))
                    .collect(),
                fn_only: Some(self.fn_depth),
            };
            self.locals.push(frame);
            let depth = self.locals.len();
            self.reserved.push((depth, names.clone()));
            let v = self.expr(value);
            self.reserved.pop();
            self.locals.pop();
            v
        } else {
            let depth = self.locals.len();
            self.reserved.push((depth, names));
            let value = self.expr(value);
            self.reserved.pop();
            value
        };
        self.ty_frames.pop();
        let binding = Binding {
            pattern,
            value,
            direct,
            ty_vars,
            span,
        };
        self.check_label_binding(&binding);
        (binding, binders)
    }

    /// §6.10: a direct value is made without running code.
    fn is_direct(&self, e: &n::Expr) -> bool {
        use n::ExprKind as E;
        let e = ungroup(e);
        let core_free = self.pre().core_free;
        let literal = |e: &n::Expr| matches!(e.kind, E::Int(_) | E::Float(_) | E::String(_));
        let items = |items: &[n::DItem]| {
            items.iter().all(|i| match i {
                n::DItem::Pos(e) | n::DItem::Label(_, e) | n::DItem::Spread(e) => self.is_direct(e),
            })
        };
        match &e.kind {
            E::Chain(_) | E::Int(_) | E::Float(_) | E::String(_) | E::Name { .. } => true,
            // `core.never` is the function with no branch.
            E::Field { .. } if is_core_never(core_free, e) => true,
            E::Unary { expr, .. } => literal(ungroup(expr)),
            E::Field { target, .. } => match &target.kind {
                // `m.x` is a name when `m` is a module.
                E::Name { module: None, name } => {
                    matches!(
                        self.find(&name.name, name.span.0),
                        Some(Found::Entry(Entry::Module(_)))
                    )
                }
                _ => false,
            },
            E::Data(all) => {
                if all.iter().all(|i| matches!(i, n::DItem::Pos(_))) {
                    items(all)
                } else {
                    // Data with labels: the base and the labelled items.
                    let (base, rest) = match all.split_first() {
                        Some((n::DItem::Spread(b), rest)) => (Some(b), rest),
                        _ => (None, all.as_slice()),
                    };
                    base.is_none_or(|b| self.is_direct(b))
                        && rest.iter().all(|i| match i {
                            n::DItem::Label(_, e) => self.is_direct(e),
                            _ => true,
                        })
                }
            }
            E::List {
                items: parts, rest, ..
            } => parts
                .iter()
                .chain(rest.as_deref())
                .all(|x| self.is_direct(x)),
            // Constructors are the only values with an `UpName`.
            E::Call { callee, args } => {
                !(is_core_never(core_free, callee) && args.len() == 1)
                    && plain_call(args)
                    && is_constructor(callee)
                    && items(args)
            }
            // §6.9: `a |> C` is `C(a)`, and `a |> C(b)` is `C(a, b)`.
            E::Pipe { lhs, rhs } => {
                !matches!(&rhs.kind, E::Chain(fs) if is_match(fs, self.pre().lisp))
                    && self.is_direct(lhs)
                    && match &rhs.kind {
                        E::Name { .. } => is_constructor(rhs),
                        E::Call { callee, args } => {
                            !(is_core_never(core_free, callee) && args.len() == 1)
                                && plain_call(args)
                                && is_constructor(callee)
                                && items(args)
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    // ---- Expressions ----

    fn name_expr(&mut self, name: &str, span: Span) -> Expr {
        Expr {
            kind: self.r.name(None, &ident(name, span)),
            span,
        }
    }

    fn expr(&mut self, e: &n::Expr) -> Expr {
        use n::ExprKind as E;
        let span = e.span.0;
        if let Some(old) = self.checked_inputs.and_then(|c| c.expressions.get(&span)) {
            let mut free = HashSet::new();
            let mut tree = old.clone();
            visit_expr_mut(&mut tree, &mut |e| {
                if let ExprKind::Name(ValueRef::Local(v)) = e.kind {
                    free.insert(v);
                }
                true
            });
            for binder in expression_binders(old) {
                free.remove(&binder.var);
            }
            if free.iter().any(|v| {
                !self.locals.iter().any(|frame| {
                    frame.fn_only.is_none_or(|depth| self.fn_depth > depth)
                        && frame.names.values().any(|w| w == v)
                })
            }) {
                self.error(
                    code::MACRO_RESERVED,
                    span,
                    "typed macro output cannot escape an argument binder",
                );
                return Expr {
                    kind: ExprKind::Error,
                    span,
                };
            }
            return old.clone();
        }
        let core_free = self.pre().core_free;
        let lisp = self.pre().lisp;
        // A call or a pipe may be the call of a typed macro: its scope is
        // kept for the expansion.
        let call = match &e.kind {
            E::Call { callee, args } => {
                !(is_core_never(core_free, callee) && args.len() == 1) && plain_call(args)
            }
            E::Pipe { rhs, .. } => !matches!(&rhs.kind, E::Chain(fs) if is_match(fs, lisp)),
            _ => false,
        };
        if call && !self.p.typed_macros.is_empty() {
            let scope = ExpansionScope {
                locals: self.locals.clone(),
                ty_frames: self.ty_frames.clone(),
                fn_depth: self.fn_depth,
                pending: self.pending.clone(),
                reserved: self.reserved.clone(),
            };
            let cur = self.cur;
            self.p.expansion_scopes.insert((cur, span), scope);
        }
        let kind = match &e.kind {
            E::Quote(_) => {
                self.error(
                    code::MACRO_RESERVED,
                    span,
                    "a quote must be evaluated inside a macro",
                );
                ExprKind::Error
            }
            E::Int(n) => ExprKind::Int(n.clone()),
            E::Float(f) => ExprKind::Float(*f),
            E::String(s) => ExprKind::String(s.clone()),
            // A string with parts is the parts joined by `<>`.
            E::Interp(parts) => {
                let mut operands: Vec<Expr> = parts
                    .iter()
                    .map(|p| match p {
                        n::InterpPart::Text(s) => Expr {
                            kind: ExprKind::String(s.clone()),
                            span,
                        },
                        n::InterpPart::Expr(e) => self.expr(e),
                    })
                    .collect();
                if operands.len() < 2 {
                    operands.insert(
                        0,
                        Expr {
                            kind: ExprKind::String(String::new()),
                            span,
                        },
                    );
                }
                let mut operands = operands.into_iter();
                let mut acc = operands.next().unwrap();
                for next in operands {
                    acc = Expr {
                        kind: ExprKind::Binary {
                            op: o::BinOp::Concat,
                            lhs: Box::new(acc),
                            rhs: Box::new(next),
                        },
                        span,
                    };
                }
                return acc;
            }
            E::Name { module, name } => {
                if self.parsed[self.cur.0 as usize]
                    .scoped
                    .as_ref()
                    .and_then(|s| s.origin(span))
                    .is_some_and(|o| {
                        matches!(
                            o.syntax.node(),
                            ilang_syntax::lisp::syntax::Node::TypeView(_)
                        )
                    })
                {
                    self.error(
                        code::MACRO_RESERVED,
                        span,
                        "a TypeView is valid only in type position",
                    );
                    ExprKind::Error
                } else {
                    self.r.name(opt_id(module).as_ref(), &id(name))
                }
            }
            E::Data(items) => {
                // `()` is the empty data: an empty tuple, as its pattern is.
                if items.iter().all(|i| matches!(i, n::DItem::Pos(_))) {
                    ExprKind::Tuple(
                        items
                            .iter()
                            .map(|i| match i {
                                n::DItem::Pos(e) => self.expr(e),
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                } else {
                    let (base, rest) = match items.split_first() {
                        Some((n::DItem::Spread(b), rest)) => (Some(Box::new(self.expr(b))), rest),
                        _ => (None, items.as_slice()),
                    };
                    ExprKind::Record {
                        base,
                        fields: self.fields(rest),
                    }
                }
            }
            E::List { items, rest } => ExprKind::List {
                items: items.iter().map(|x| self.expr(x)).collect(),
                rest: rest.as_ref().map(|r| Box::new(self.expr(r))),
            },
            E::BitArray(segs) => ExprKind::BitArray(
                segs.iter()
                    .map(|s| Segment {
                        value: self.expr(&s.value),
                        format: self.format(s.format.as_ref()),
                        span: s.value.span.0,
                    })
                    .collect(),
            ),
            E::Block(b) => ExprKind::Block(self.block(b)),
            E::Chain(fs) => self.chain(fs, span),
            // `core.never(x)`: the match of `x` with no branch (§2.13).
            E::Call { callee, args } if is_core_never(core_free, callee) && args.len() == 1 => {
                let subject = match &args[0] {
                    n::DItem::Pos(e) | n::DItem::Label(_, e) | n::DItem::Spread(e) => self.expr(e),
                };
                ExprKind::Case {
                    subject: Box::new(subject),
                    clauses: Vec::new(),
                }
            }
            E::Call { callee, args } => match args.split_first() {
                // `C(..base, f: e)`: a record update.
                Some((n::DItem::Spread(base), rest)) => {
                    let ctor = match &callee.kind {
                        E::Name { module, name } => {
                            self.r.ctor(opt_id(module).as_ref(), &id(name))
                        }
                        _ => {
                            self.error(
                                code::NOT_A_CONSTRUCTOR,
                                callee.span.0,
                                "a record update starts with a constructor",
                            );
                            None
                        }
                    };
                    let base = Box::new(self.expr(base));
                    let fields = self.fields(rest);
                    match ctor {
                        Some(ctor) => ExprKind::RecordUpdate {
                            ctor,
                            ctor_span: callee.span.0,
                            base,
                            fields,
                        },
                        None => ExprKind::Error,
                    }
                }
                _ => ExprKind::Call {
                    callee: Box::new(self.expr(callee)),
                    args: args
                        .iter()
                        .map(|a| match a {
                            n::DItem::Pos(e) | n::DItem::Spread(e) => Arg {
                                label: None,
                                value: self.expr(e),
                            },
                            n::DItem::Label(l, e) => Arg {
                                label: Some(id(l)),
                                value: self.expr(e),
                            },
                        })
                        .collect(),
                },
            },
            E::Provide { target, services } => {
                let mut stmts = Vec::new();
                self.flatten_provide(target, services, &mut stmts);
                ExprKind::Block(Block {
                    stmts: self.stmts(stmts),
                    span,
                })
            }
            // `core.never`: the function with no branch (§2.13).
            E::Field { .. } if is_core_never(core_free, e) => {
                let synthetic = Span::new(span.start, span.start);
                let function = VFn {
                    params: vec![VPat::Fresh("%arg".into(), synthetic)],
                    envs: Vec::new(),
                    ret: None,
                    body: VBody::Case {
                        subject: VSubject::Name("%arg".into(), synthetic),
                        clauses: Vec::new(),
                        span: Span::new(span.start + 1, span.start + 1),
                    },
                    unlabel: true,
                };
                ExprKind::Fn(Box::new(self.function(&function, span)))
            }
            E::Field { target, name } => {
                let module = match &target.kind {
                    E::Name { module: None, name } => {
                        match self.find(&name.name, name.span.0) {
                            Some(Found::Entry(Entry::Module(t))) => {
                                Some(Some((t, self.compiler_reference(&id(name)))))
                            }
                            Some(Found::Entry(Entry::Broken)) => Some(None),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                match module {
                    Some(Some((t, trusted))) => self.member(t, &id(name), trusted),
                    Some(None) => ExprKind::Error,
                    None => ExprKind::Field {
                        target: Box::new(self.expr(target)),
                        name: id(name),
                    },
                }
            }
            E::Index { target, index } => ExprKind::TupleIndex {
                target: Box::new(self.expr(target)),
                index: index.clone(),
                index_span: Span::new(span.end.saturating_sub(1), span.end),
            },
            E::Unary { op, expr } => ExprKind::Unary {
                op: *op,
                expr: Box::new(self.expr(expr)),
            },
            E::Binary { op, lhs, rhs } => ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.expr(lhs)),
                rhs: Box::new(self.expr(rhs)),
            },
            E::Pipe { lhs, rhs } => match &rhs.kind {
                // A match (§2.11): one position per branch.
                E::Chain(fs) if is_match(fs, lisp) => {
                    let clauses: Vec<VClause> = fs.iter().map(|f| self.vclause(f, 1)).collect();
                    self.case(&VSubject::Expr(lhs), &clauses, span)
                }
                _ => ExprKind::Binary {
                    op: o::BinOp::Pipe,
                    lhs: Box::new(self.expr(lhs)),
                    rhs: Box::new(self.expr(rhs)),
                },
            },
        };
        Expr { kind, span }
    }

    /// The labelled items of data with labels; the others are an error of
    /// this stage.
    fn fields(&mut self, items: &[n::DItem]) -> Vec<(o::Ident, Expr)> {
        let mut out = Vec::new();
        for i in items {
            if let n::DItem::Label(l, e) = i {
                out.push((id(l), self.expr(e)));
            }
        }
        out
    }

    fn format(&mut self, f: Option<&n::Format>) -> Option<Format> {
        f.map(|f| Format {
            name: id(&f.name),
            arg: f.arg.as_ref().map(|a| Box::new(self.expr(a))),
        })
    }

    /// A chain as a value (§2.5): a function, or a function over a match of
    /// its branches.
    fn chain(&mut self, fs: &[n::Function], span: Span) -> ExprKind {
        let synthetic = Span::new(span.start, span.start);
        fn services(f: &n::Function) -> Vec<&n::SItem> {
            f.services.iter().flatten().filter(|s| !s.rest).collect()
        }
        let function = match chain_kind(fs) {
            ChainKind::Plain => {
                let f = &fs[0];
                VFn {
                    params: f.params.iter().map(|p| VPat::Pat(&p.choices[0])).collect(),
                    envs: services(f),
                    ret: f.ret.as_ref(),
                    body: VBody::Block(&f.body),
                    unlabel: false,
                }
            }
            ChainKind::Clauses => VFn {
                params: vec![VPat::Fresh("%arg".into(), synthetic)],
                envs: Vec::new(),
                ret: None,
                body: VBody::Case {
                    subject: VSubject::Name("%arg".into(), synthetic),
                    clauses: fs.iter().map(|f| self.vclause(f, 1)).collect(),
                    span: fs[0].span.0.to(fs[fs.len() - 1].span.0),
                },
                unlabel: true,
            },
            ChainKind::Wrapped => {
                // Each fresh parameter has the span of the pattern of its
                // position in the first branch, and the match a span of its
                // own: the checker keeps facts by span.
                let count = fs[0].params.len();
                let fresh = self.pre().args.get(&addr(&fs[0]));
                let names: Vec<(String, Span)> = (0..count)
                    .map(|i| {
                        let at = fs[0].params[i].choices[0].span.0;
                        let name = fresh
                            .and_then(|names| names.get(i).cloned())
                            .unwrap_or_else(|| format!("%arg{}", i + 1));
                        (name, at)
                    })
                    .collect();
                let inside = Span::new(span.start + 1, span.end.max(span.start + 1));
                let mut envs = Vec::new();
                let mut seen = HashSet::new();
                for f in fs {
                    for s in services(f) {
                        if seen.insert(s.name.name.clone()) {
                            envs.push(s);
                        }
                    }
                }
                let subject = if count == 1 {
                    VSubject::Name(names[0].0.clone(), names[0].1)
                } else {
                    VSubject::Tuple(names.clone(), synthetic)
                };
                VFn {
                    params: names
                        .into_iter()
                        .map(|(name, at)| VPat::Fresh(name, at))
                        .collect(),
                    envs,
                    ret: fs.iter().find_map(|f| f.ret.as_ref()),
                    body: VBody::Case {
                        subject,
                        clauses: fs.iter().map(|f| self.vclause(f, count)).collect(),
                        span: inside,
                    },
                    unlabel: false,
                }
            }
        };
        ExprKind::Fn(Box::new(self.function(&function, span)))
    }

    /// A branch as a clause over one subject: the pattern of its one
    /// position, or the data of the patterns of its `count` positions.
    fn vclause<'x>(&self, f: &'x n::Function, count: usize) -> VClause<'x> {
        let span = f.span.0;
        let patterns = if count == 1 && f.params.len() == 1 {
            f.params[0].choices.iter().map(VPat::Pat).collect()
        } else {
            // Every way to take one alternative of each position.
            let mut rows: Vec<Vec<&n::Pattern>> = vec![Vec::new()];
            for p in &f.params {
                rows = rows
                    .into_iter()
                    .flat_map(|row| {
                        p.choices.iter().map(move |c| {
                            let mut row = row.clone();
                            row.push(c);
                            row
                        })
                    })
                    .collect();
            }
            rows.into_iter()
                .map(|row| VPat::Tuple(row, span))
                .collect()
        };
        VClause {
            patterns,
            guard: f.guard.as_ref(),
            body: &f.body,
            span,
        }
    }

    /// The body of a branch: its one expression, or its block.
    fn body(&mut self, b: &n::Block) -> Expr {
        match b.stmts.as_slice() {
            [n::Stmt::Expr(e)] => self.expr(e),
            _ => Expr {
                kind: ExprKind::Block(self.block(b)),
                span: b.span.0,
            },
        }
    }

    fn subject(&mut self, subject: &VSubject) -> Expr {
        match subject {
            VSubject::Expr(e) => self.expr(e),
            VSubject::Name(name, span) => self.name_expr(name, *span),
            VSubject::Tuple(names, span) => Expr {
                kind: ExprKind::Tuple(
                    names
                        .iter()
                        .map(|(name, at)| self.name_expr(name, *at))
                        .collect(),
                ),
                span: *span,
            },
        }
    }

    /// A match of `subject`; `span` is the span of the whole.
    fn case(&mut self, subject: &VSubject, clauses: &Vec<VClause>, span: Span) -> ExprKind {
        if let [clause] = clauses.as_slice()
            && clause.guard.is_none()
            && clause.patterns.len() == 1
        {
            // §3.9: the subject precedes the new scope; no type frame,
            // no generalization, and the binding stops at the pattern.
            let value = self.subject(subject);
            let mut binders = Vec::new();
            let pattern = self.vpat(&clause.patterns[0], &mut binders, None);
            let frame = Frame {
                names: binders
                    .into_iter()
                    .map(|b| (self.name_key(&b.name, b.span), b.var))
                    .collect(),
                fn_only: None,
            };
            self.locals.push(frame);
            let body = self.body(clause.body);
            self.locals.pop();
            let at = pattern.span;
            return ExprKind::Block(Block {
                stmts: vec![
                    Stmt::Bind(Binding {
                        pattern,
                        value,
                        direct: false,
                        ty_vars: vec![],
                        span: at,
                    }),
                    Stmt::Expr(body),
                ],
                span,
            });
        }
        ExprKind::Case {
            subject: Box::new(self.subject(subject)),
            clauses: clauses.iter().map(|c| self.clause(c)).collect(),
        }
    }

    /// The statements of a block; a provide that ends it is environment
    /// statements before the rest of the block.
    fn flatten_block<'x>(&self, b: &'x n::Block, out: &mut Vec<VStmt<'x>>) {
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                n::Stmt::Expr(n::Expr {
                    kind: n::ExprKind::Provide { target, services },
                    ..
                }) if i + 1 == b.stmts.len() => self.flatten_provide(target, services, out),
                n::Stmt::Bind(b) => out.push(VStmt::Bind(b)),
                n::Stmt::Expr(e) => out.push(VStmt::Expr(e)),
            }
        }
    }

    /// `e <- (s1, s2)`: the environment statements and then `e`. Every
    /// service is worked out in the outer environment (§2.10.5): when there
    /// are several and one is not a name, each is bound to a fresh local
    /// first, so that no service sees another.
    fn flatten_provide<'x>(
        &self,
        target: &'x n::Expr,
        services: &'x n::Expr,
        out: &mut Vec<VStmt<'x>>,
    ) {
        let mut envs = Vec::new();
        if let n::ExprKind::Data(items) = &services.kind {
            let apart = provide_apart(items);
            for item in items {
                let n::DItem::Pos(e) = item else {
                    continue;
                };
                if apart {
                    let name = self
                        .pre()
                        .services
                        .get(&addr(e))
                        .cloned()
                        .unwrap_or_else(|| "%service".into());
                    out.push(VStmt::Service(name.clone(), e));
                    envs.push(VStmt::Env(Some(name), e));
                } else {
                    envs.push(VStmt::Env(None, e));
                }
            }
        }
        out.extend(envs);
        match &target.kind {
            n::ExprKind::Block(b) => self.flatten_block(b, out),
            n::ExprKind::Provide {
                target: inner,
                services: more,
            } => self.flatten_provide(inner, more, out),
            _ => out.push(VStmt::Expr(target)),
        }
    }

    fn block(&mut self, b: &n::Block) -> Block {
        let mut stmts = Vec::new();
        self.flatten_block(b, &mut stmts);
        Block {
            stmts: self.stmts(stmts),
            span: b.span.0,
        }
    }

    fn stmts(&mut self, list: Vec<VStmt>) -> Vec<Stmt> {
        let mark = self.locals.len();
        let mut stmts = Vec::new();
        for s in list {
            let bound = match s {
                VStmt::Bind(binding) => Some(self.binding(binding, false)),
                VStmt::Service(name, e) => {
                    Some(self.binding_of(&VPat::Fresh(name, e.span.0), e, e.span.0, &[], false))
                }
                VStmt::Env(name, e) => {
                    // The key of the service is read from the type of its
                    // value: the type of the statement is `_`.
                    let span = e.span.0;
                    let value = match name {
                        None => self.expr(e),
                        Some(name) => self.name_expr(&name, span),
                    };
                    stmts.push(Stmt::Env {
                        ty: Type {
                            kind: TypeKind::Hole,
                            span,
                        },
                        value,
                        span,
                    });
                    None
                }
                VStmt::Expr(e) => {
                    stmts.push(Stmt::Expr(self.expr(e)));
                    None
                }
            };
            if let Some((binding, binders)) = bound {
                let frame = Frame {
                    names: binders
                        .into_iter()
                        .map(|b| (self.name_key(&b.name, b.span), b.var))
                        .collect(),
                    fn_only: None,
                };
                self.locals.push(frame);
                stmts.push(Stmt::Bind(binding));
            }
        }
        self.locals.truncate(mark);
        stmts
    }

    /// §6.8 and §6.11.
    fn function(&mut self, f: &VFn, span: Span) -> Function {
        let mut params = Vec::new();
        let mut env_heads: Vec<TypeRef> = Vec::new();
        let mut all: Vec<Binder> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        let mut pending = Vec::new();
        for p in &f.params {
            self.vpat_binders(p, &mut pending);
        }
        for s in &f.envs {
            pending.push((s.name.name.clone(), s.name.span.0));
        }
        let pending = pending
            .into_iter()
            .map(|(n, span)| self.name_key(&n, span))
            .collect();
        self.pending.push(pending);
        for pat in &f.params {
            let mut binders = Vec::new();
            let pattern = self.vpat(pat, &mut binders, None);
            for b in binders {
                self.param_name(&all, &b.name, b.span);
                all.push(b);
            }
            let label = self.vpat_label(pat);
            if let Some(l) = &label {
                if labels.contains(l) {
                    self.error(
                        code::PARAMS_SAME_LABEL,
                        pat.span(),
                        format!("two parameters of this function have the label `{l}`"),
                    );
                }
                labels.push(l.clone());
            }
            params.push(Param::Pattern { pattern, label });
        }
        // The service pattern: `name: T` is an environment parameter.
        for s in &f.envs {
            let name = id(&s.name);
            let ty = self.ty(&s.ty);
            // §9.2: two environment parameters of one function may
            // not have the same type constructor.
            if let Some(head) = self.env_type(&ty) {
                if env_heads.contains(&head) {
                    let tname = self.type_name(head);
                    self.error(
                        code::ENV_PARAMS_SAME,
                        ty.span,
                        format!("two environment parameters of this function have the type constructor `{tname}` (§9.2)"),
                    );
                }
                env_heads.push(head);
            }
            let var = self
                .checked_inputs
                .and_then(|c| c.binders.get(&name.span))
                .copied()
                .unwrap_or_else(|| self.new_var());
            self.check_binder(&name.name, name.span, false, false);
            self.param_name(&all, &name.name, name.span);
            all.push(Binder {
                var,
                name: name.name.clone(),
                span: name.span,
            });
            params.push(Param::Env { var, name, ty });
        }
        // The written `-> R`: the needs are left to inference.
        let ret = f.ret.map(|r| {
            let hole = Type {
                kind: TypeKind::Hole,
                span: r.span.0,
            };
            RetType {
                value: self.ty(r),
                err: Some(hole.clone()),
                env: Some(hole),
            }
        });
        self.pending.pop();
        self.fn_depth += 1;
        let frame = Frame {
            names: all
                .into_iter()
                .map(|b| (self.name_key(&b.name, b.span), b.var))
                .collect(),
            fn_only: None,
        };
        self.locals.push(frame);
        let body = match &f.body {
            VBody::Block(b) => self.block(b),
            VBody::Case {
                subject,
                clauses,
                span: at,
            } => Block {
                stmts: vec![Stmt::Expr(Expr {
                    kind: self.case(subject, clauses, *at),
                    span: *at,
                })],
                span,
            },
        };
        self.locals.pop();
        self.fn_depth -= 1;
        if f.unlabel
            && let Some(Param::Pattern { label, .. }) = params.first_mut()
        {
            *label = None;
        }
        Function {
            params,
            ret,
            body,
            span,
        }
    }

    fn clause(&mut self, c: &VClause) -> Clause {
        let mut first: Vec<Binder> = Vec::new();
        let mut patterns = Vec::new();
        let mut pending = Vec::new();
        for p in &c.patterns {
            self.vpat_binders(p, &mut pending);
        }
        let pending = pending
            .into_iter()
            .map(|(n, span)| self.name_key(&n, span))
            .collect();
        self.pending.push(pending);
        for (i, p) in c.patterns.iter().enumerate() {
            if i == 0 {
                patterns.push(self.vpat(p, &mut first, None));
                continue;
            }
            let shared: HashMap<NameKey, VarId> = first
                .iter()
                .map(|b| (self.name_key(&b.name, b.span), b.var))
                .collect();
            let mut binders = Vec::new();
            patterns.push(self.vpat(p, &mut binders, Some(&shared)));
            let a: HashSet<NameKey> = first
                .iter()
                .map(|b| self.name_key(&b.name, b.span))
                .collect();
            let b: HashSet<NameKey> = binders
                .iter()
                .map(|b| self.name_key(&b.name, b.span))
                .collect();
            if a != b {
                let mut diff: Vec<&str> = a
                    .symmetric_difference(&b)
                    .map(|k| k.text.as_str())
                    .collect();
                diff.sort();
                self.error(
                    code::CHOICES_DIFFER,
                    p.span(),
                    format!(
                        "every choice of a clause binds the same names, but not all bind {}",
                        diff.iter()
                            .map(|n| format!("`{n}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
        }
        self.pending.pop();
        let frame = Frame {
            names: first
                .into_iter()
                .map(|b| (self.name_key(&b.name, b.span), b.var))
                .collect(),
            fn_only: None,
        };
        self.locals.push(frame);
        let guard = c.guard.map(|g| self.expr(g));
        let body = self.body(c.body);
        self.locals.pop();
        Clause {
            patterns,
            guard,
            body,
            span: c.span,
        }
    }

    // ---- Patterns ----

    fn keyed<'x>(&self, l: &'x n::Ident, p: &'x n::Pattern) -> AndPlan<'x> {
        let pre = self.pre();
        keyed_plan(l, p, || {
            pre.fields
                .get(&addr(p))
                .cloned()
                .unwrap_or_else(|| "%field".into())
        })
    }

    /// The names a pattern binds, in order, with repeats (§7.2).
    fn binders(&self, p: &n::Pattern, out: &mut Vec<(String, Span)>) {
        use n::PatternKind as P;
        match &p.kind {
            P::Quote(q) => {
                if let Some(old) = quote_pattern(q, p.span.0) {
                    ast_binders(&old, out);
                }
            }
            P::Var(n) => out.push((n.clone(), p.span.0)),
            P::Typed { binder, .. } if !binder.discard => {
                out.push((binder.name.clone(), binder.span.0))
            }
            P::Prefix { rest, .. } if !rest.discard => out.push((rest.name.clone(), rest.span.0)),
            P::Ctor { args, .. } => {
                for item in args.iter().flatten() {
                    match item {
                        n::DPItem::Pos(p) => self.binders(p, out),
                        n::DPItem::Key(l, p) => self.plan_binders(&self.keyed(l, p), out),
                        n::DPItem::Rest(_) => {}
                    }
                }
            }
            P::Data(items) => {
                for item in items {
                    if let n::DPItem::Pos(p) = item {
                        self.binders(p, out);
                    }
                }
            }
            P::List { items, rest } => {
                items.iter().for_each(|a| self.binders(a, out));
                if let Some(Some(b)) = rest
                    && !b.discard
                {
                    out.push((b.name.clone(), b.span.0));
                }
            }
            P::BitArray(segs) => segs.iter().for_each(|s| self.binders(&s.value, out)),
            P::And(parts) => self.plan_binders(&and_plan(parts, None, p.span.0), out),
            _ => {}
        }
    }

    fn plan_binders(&self, plan: &AndPlan, out: &mut Vec<(String, Span)>) {
        out.extend(plan.names.iter().cloned());
        match &plan.inner {
            Inner::Pat(p) => self.binders(p, out),
            Inner::Typed { label, .. } => out.push((label.name.clone(), label.span.0)),
            Inner::Var(name, span) => out.push((name.clone(), *span)),
        }
    }

    fn vpat_binders(&self, p: &VPat, out: &mut Vec<(String, Span)>) {
        match p {
            VPat::Pat(p) => self.binders(p, out),
            VPat::Fresh(name, span) => out.push((name.clone(), *span)),
            VPat::Tuple(row, _) => row.iter().for_each(|p| self.binders(p, out)),
        }
    }

    /// The label of a parameter (§6.11): the last name in the `=` chain at
    /// the top of its pattern.
    fn vpat_label(&self, p: &VPat) -> Option<String> {
        fn of(p: &n::Pattern) -> Option<String> {
            use n::PatternKind as P;
            match &p.kind {
                P::Quote(q) => quote_pattern(q, p.span.0).and_then(|old| label_of(&old)),
                P::Var(n) => Some(n.clone()),
                P::Typed { binder, .. } if !binder.discard => Some(binder.name.clone()),
                P::And(parts) => {
                    let plan = and_plan(parts, None, p.span.0);
                    let inner = match &plan.inner {
                        Inner::Pat(p) => of(p),
                        Inner::Typed { label, .. } => Some(label.name.clone()),
                        Inner::Var(name, _) => Some(name.clone()),
                    };
                    inner.or_else(|| plan.names.last().map(|n| n.0.clone()))
                }
                _ => None,
            }
        }
        match p {
            VPat::Pat(p) => of(p),
            VPat::Fresh(name, _) => Some(name.clone()),
            VPat::Tuple(..) => None,
        }
    }

    fn vpat(
        &mut self,
        p: &VPat,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<NameKey, VarId>>,
    ) -> Pattern {
        match p {
            VPat::Pat(p) => self.pattern(p, binders, shared),
            VPat::Fresh(name, span) => Pattern {
                kind: PatternKind::Var(self.bind(name, *span, binders, shared)),
                span: *span,
            },
            VPat::Tuple(row, span) => Pattern {
                kind: PatternKind::Tuple(
                    row.iter()
                        .map(|p| self.pattern(p, binders, shared))
                        .collect(),
                ),
                span: *span,
            },
        }
    }

    /// The names of a plan from `from` on, around its inner pattern.
    fn plan_pattern(
        &mut self,
        plan: &AndPlan,
        from: usize,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<NameKey, VarId>>,
    ) -> Pattern {
        if let Some((name, at)) = plan.names.get(from) {
            let binder = self.bind(name, *at, binders, shared);
            let inner = self.plan_pattern(plan, from + 1, binders, shared);
            return Pattern {
                kind: PatternKind::Named {
                    binder,
                    pattern: Box::new(inner),
                },
                span: plan.span,
            };
        }
        match &plan.inner {
            Inner::Pat(p) => self.pattern(p, binders, shared),
            Inner::Typed { label, ty, span } => {
                let binder = Some(self.bind(&label.name, label.span.0, binders, shared));
                Pattern {
                    kind: PatternKind::Typed {
                        binder,
                        ty: self.ty(ty),
                    },
                    span: *span,
                }
            }
            Inner::Var(name, span) => Pattern {
                kind: PatternKind::Var(self.bind(name, *span, binders, shared)),
                span: *span,
            },
        }
    }

    /// A part of a constructor pattern. §7.1: `label = label` writes the
    /// label of a part and binds the name once.
    fn ctor_part(
        &mut self,
        plan: &AndPlan,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<NameKey, VarId>>,
    ) -> Pattern {
        let same = match plan.names.as_slice() {
            [(name, _)] => match &plan.inner {
                Inner::Var(inner, _) => inner == name,
                Inner::Typed { label, .. } => label.name == *name,
                Inner::Pat(p) => match &p.kind {
                    n::PatternKind::Var(inner) => inner == name,
                    n::PatternKind::Typed { binder, .. } => {
                        !binder.discard && binder.name == *name
                    }
                    _ => false,
                },
            },
            _ => false,
        };
        if same {
            Pattern {
                span: plan.span,
                ..self.plan_pattern(plan, 1, binders, shared)
            }
        } else {
            self.plan_pattern(plan, 0, binders, shared)
        }
    }

    fn pattern(
        &mut self,
        p: &n::Pattern,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<NameKey, VarId>>,
    ) -> Pattern {
        use n::PatternKind as P;
        let span = p.span.0;
        let kind = match &p.kind {
            // 2c: see `quote_pattern`.
            P::Quote(q) => match quote_pattern(q, span) {
                Some(old) => return self.r.pattern(&old, binders, shared),
                None => PatternKind::Discard,
            },
            P::Var(n) => PatternKind::Var(self.bind(n, span, binders, shared)),
            P::Discard(_) => PatternKind::Discard,
            P::Typed { binder, ty } => {
                let binder = (!binder.discard)
                    .then(|| self.bind(&binder.name, binder.span.0, binders, shared));
                PatternKind::Typed {
                    binder,
                    ty: self.ty(ty),
                }
            }
            P::Int { negative, value } => PatternKind::Int {
                negative: *negative,
                value: value.clone(),
            },
            P::Float { negative, value } => PatternKind::Float {
                negative: *negative,
                value: *value,
            },
            P::String(s) => PatternKind::String(s.clone()),
            P::Prefix {
                prefix,
                suffix,
                rest,
            } => PatternKind::Prefix {
                prefix: prefix.clone(),
                suffix: suffix.clone(),
                rest: (!rest.discard)
                    .then(|| self.bind(&rest.name, rest.span.0, binders, shared)),
            },
            P::Ctor { module, name, args } => {
                let ctor = self.r.ctor(opt_id(module).as_ref(), &id(name));
                let mut parts = Vec::new();
                let mut spread = false;
                for item in args.iter().flatten() {
                    match item {
                        n::DPItem::Pos(a) => parts.push(match &a.kind {
                            P::And(inner) => {
                                let plan = and_plan(inner, None, a.span.0);
                                self.ctor_part(&plan, binders, shared)
                            }
                            P::Quote(q) => match quote_pattern(q, a.span.0) {
                                Some(old) => match same_name_part(&old) {
                                    Some(inner) => Pattern {
                                        span: old.span,
                                        ..self.r.pattern(inner, binders, shared)
                                    },
                                    None => self.r.pattern(&old, binders, shared),
                                },
                                None => self.pattern(a, binders, shared),
                            },
                            _ => self.pattern(a, binders, shared),
                        }),
                        n::DPItem::Key(l, a) => {
                            let plan = self.keyed(l, a);
                            parts.push(self.ctor_part(&plan, binders, shared));
                        }
                        n::DPItem::Rest(None) => spread = true,
                        // A named rest is an error of this stage.
                        n::DPItem::Rest(Some(_)) => {}
                    }
                }
                match ctor {
                    Some(ctor) => PatternKind::Constructor {
                        ctor,
                        args: parts,
                        spread,
                    },
                    None => PatternKind::Error(parts),
                }
            }
            // Keys and a rest in a data pattern are errors of this stage.
            P::Data(items) => PatternKind::Tuple(
                items
                    .iter()
                    .filter_map(|item| match item {
                        n::DPItem::Pos(a) => Some(self.pattern(a, binders, shared)),
                        _ => None,
                    })
                    .collect(),
            ),
            P::List { items, rest } => {
                let items = items
                    .iter()
                    .map(|a| self.pattern(a, binders, shared))
                    .collect();
                let rest = rest.as_ref().map(|r| {
                    r.as_ref()
                        .filter(|b| !b.discard)
                        .map(|b| self.bind(&b.name, b.span.0, binders, shared))
                });
                PatternKind::List { items, rest }
            }
            P::BitArray(segs) => {
                let mut out = Vec::new();
                for s in segs {
                    let value = self.pattern(&s.value, binders, shared);
                    // §8.4: a length may use names of earlier segments.
                    let frame = Frame {
                        names: binders
                            .iter()
                            .map(|b| (self.name_key(&b.name, b.span), b.var))
                            .collect(),
                        fn_only: None,
                    };
                    self.locals.push(frame);
                    let format = self.format(s.format.as_ref());
                    self.locals.pop();
                    out.push(Segment {
                        value,
                        format,
                        span: s.value.span.0,
                    });
                }
                PatternKind::BitArray(out)
            }
            P::And(parts) => {
                let plan = and_plan(parts, None, span);
                return self.plan_pattern(&plan, 0, binders, shared);
            }
        };
        Pattern { kind, span }
    }
}
