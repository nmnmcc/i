//! Name resolution for target provider plans (§2.6.1).
//!
//! A plan is resolved using the package which declared each cell, not the
//! package whose entry eventually uses it.

use crate::hir::{DefId, ModuleId, Program, TypeId};
use ilang_package::{ModuleKind, PackageGraph, PackageId};

/// The failure to resolve a provider-plan path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathError {
    InvalidPath,
    UnknownPackage,
    UnknownModule,
    UnknownName,
    Private,
}

fn module(
    graph: &PackageGraph,
    program: &Program,
    authority: PackageId,
    path: &str,
) -> Result<(ModuleId, String), PathError> {
    let (module, name) = path.rsplit_once('.').ok_or(PathError::InvalidPath)?;
    let (prefix, rest) = module.split_once('/').ok_or(PathError::InvalidPath)?;
    if name.is_empty() || rest.is_empty() {
        return Err(PathError::InvalidPath);
    }
    let package = &graph.packages[authority];
    let target = if prefix == "i" {
        graph.library
    } else if prefix == package.name {
        authority
    } else {
        package
            .dependencies
            .iter()
            .find(|(name, _)| name == prefix)
            .and_then(|(_, id)| *id)
            .ok_or(PathError::UnknownPackage)?
    };
    let canonical = format!("{}/{rest}", graph.packages[target].name);
    let id = program
        .modules
        .iter()
        .position(|m| m.package == target && m.path == canonical && m.kind == ModuleKind::Src)
        .ok_or(PathError::UnknownModule)?;
    Ok((ModuleId(id as u32), name.to_owned()))
}

/// Resolve a service constructor under the declaring package's authority.
pub fn resolve_service(
    graph: &PackageGraph,
    program: &Program,
    authority: PackageId,
    path: &str,
) -> Result<TypeId, PathError> {
    let (module, name) = module(graph, program, authority, path)?;
    let id = program
        .module(module)
        .types
        .iter()
        .copied()
        .find(|id| program.ty(*id).name == name)
        .ok_or(PathError::UnknownName)?;
    if program.module(module).package != authority && !program.ty(id).exported {
        return Err(PathError::Private);
    }
    Ok(id)
}

/// Resolve a factory or adapter binding under the declaring package's authority.
pub fn resolve_binding(
    graph: &PackageGraph,
    program: &Program,
    authority: PackageId,
    path: &str,
) -> Result<DefId, PathError> {
    let (module, name) = module(graph, program, authority, path)?;
    let (index, def) = program
        .module(module)
        .defs
        .iter()
        .enumerate()
        .find(|(_, def)| def.name == name)
        .ok_or(PathError::UnknownName)?;
    if program.module(module).package != authority && !def.exported {
        return Err(PathError::Private);
    }
    Ok(DefId {
        module,
        index: index as u32,
    })
}

use crate::{
    Checked, code,
    hir::{self, BuiltinType, TypeRef},
    types::{ShownType, Typed},
};
use ilang_package::{
    Diagnostic,
    providers::{Argument, Cell, ProviderRow, Role, Target},
};
use ilang_syntax::span::Span;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const PLAN_ERROR: &str = "E0523";

/// Provider instances in different adapter scopes never share a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    Outer,
    Body,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Service {
    pub constructor: TypeId,
    pub arguments: Vec<ShownType>,
    /// Fully qualified path with normalized closed arguments, for stable ordering.
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Supply {
    Host,
    Factory(DefId),
    Constructor(hir::CtorRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedArgument {
    /// None means []; Some means [the adapter's supplied service].
    Seed(Option<DefId>),
    EntryRootKey,
    /// An ordinary captured service, by its construction position in steps.
    Service(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionStep {
    pub service: Service,
    pub supply: Supply,
    pub arguments: Vec<ResolvedArgument>,
    pub scope: Scope,
    pub role: Role,
    pub request_mode: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedAdapter {
    pub binding: DefId,
    pub supplies: Vec<Service>,
    pub requires: Vec<usize>,
    /// Enter the adapter immediately before this construction position.
    pub enter_at: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPlan {
    pub entry: DefId,
    pub target: Target,
    pub steps: Vec<ConstructionStep>,
    pub adapter: Option<ResolvedAdapter>,
    pub needs_entry_root_key: bool,
    pub is_async: bool,
}

/// Build each entry's complete chosen provider graph. This is a target-build
/// phase, separate from source checking and lowering. The temporary E0522
/// lowering gate remains in the build/compile callers until plan application.
/// Source E0522s are superseded here by resolved supplies or plan diagnostics.
pub fn resolve_plans(
    checked: &Checked,
    graph: &PackageGraph,
    target: Target,
) -> BTreeMap<DefId, Result<ResolvedPlan, Vec<Diagnostic>>> {
    let (Some(p), Some(t)) = (&checked.program, &checked.types) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    let schema = Schema::resolve(p, t, graph, target);
    for (&entry, needs) in &t.entry_provider_needs {
        let mut errors: Vec<_> = checked
            .diags
            .iter()
            .filter(|d| d.is_error() && d.code != code::ENTRY_ENV)
            .cloned()
            .collect();
        errors.extend(schema.errors.iter().map(|(authority, span, msg)| {
            let mut d = diagnostic(p, entry, target, *span, msg);
            d.source = graph.packages[*authority].manifest_source;
            d
        }));
        if !errors.is_empty() {
            out.insert(entry, Err(errors));
            continue;
        }
        let mut builder = Builder {
            schema: &schema,
            p,
            t,
            entry,
            target,
            nodes: vec![],
            indices: HashMap::new(),
            adapter: None,
            adapter_requires: vec![],
            adapter_origin: None,
        };
        let result = builder
            .build(needs)
            .map_err(|msg| vec![diagnostic(p, entry, target, p.def(entry).span, &msg)]);
        out.insert(entry, result);
    }
    out
}

fn diagnostic(p: &Program, entry: DefId, target: Target, span: Span, msg: &str) -> Diagnostic {
    Diagnostic::error(
        PLAN_ERROR,
        Some(p.module(entry.module).source),
        span,
        format!(
            "provider target {} entry {}.{}: {msg}",
            target_name(target),
            p.module(entry.module).path,
            p.def(entry).name
        ),
    )
}
fn target_name(t: Target) -> &'static str {
    match t {
        Target::Js => "js",
        Target::Nodejs => "nodejs",
        Target::Native => "native",
        Target::Wasi => "wasi",
        Target::Host => "host",
    }
}
fn type_name(p: &Program, r: TypeRef) -> String {
    match r {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(id) => format!("{}.{}", p.module(p.ty(id).module).path, p.ty(id).name),
    }
}
fn canonical(p: &Program, t: &ShownType) -> String {
    t.text_with(&mut |r| type_name(p, r), &mut |s| s.to_owned())
}
fn normalize(p: &Program, t: ShownType) -> ShownType {
    let union = matches!(t, ShownType::Union(_));
    match t {
        ShownType::Union(xs) | ShownType::Inter(xs) => {
            let mut out = vec![];
            for x in xs {
                let x = normalize(p, x);
                match x {
                    ShownType::Union(ys) if union => out.extend(ys),
                    ShownType::Inter(ys) if !union => out.extend(ys),
                    x => out.push(x),
                }
            }
            out.sort_by_cached_key(|x| canonical(p, x));
            out.dedup();
            if out.len() == 1 {
                out.pop().unwrap()
            } else if union {
                ShownType::Union(out)
            } else {
                ShownType::Inter(out)
            }
        }
        ShownType::Named(r, xs) => {
            ShownType::Named(r, xs.into_iter().map(|x| normalize(p, x)).collect())
        }
        ShownType::Tuple(xs) => ShownType::Tuple(xs.into_iter().map(|x| normalize(p, x)).collect()),
        ShownType::Diff(a, xs) => {
            let mut xs: Vec<_> = xs.into_iter().map(|x| normalize(p, x)).collect();
            xs.sort_by_cached_key(|x| canonical(p, x));
            xs.dedup();
            ShownType::Diff(Box::new(normalize(p, *a)), xs)
        }
        x => x,
    }
}
fn closed(t: &ShownType) -> bool {
    match t {
        ShownType::Var(_) | ShownType::Hole => false,
        ShownType::Named(_, xs)
        | ShownType::Tuple(xs)
        | ShownType::Union(xs)
        | ShownType::Inter(xs) => xs.iter().all(closed),
        ShownType::Diff(a, xs) => closed(a) && xs.iter().all(closed),
        ShownType::Fn(f) => {
            f.params.iter().all(|(_, x)| closed(x))
                && closed(&f.ret)
                && f.err.as_ref().is_none_or(closed)
                && f.env.as_ref().is_none_or(closed)
        }
        ShownType::Graph(g) => g.graph_is_closed(),
        _ => true,
    }
}
fn service(p: &Program, ty: ShownType) -> Result<Service, String> {
    let ty = normalize(p, ty);
    if !closed(&ty) {
        return Err(format!("service {} must be closed", canonical(p, &ty)));
    }
    let ShownType::Named(TypeRef::Custom(id), args) = &ty else {
        return Err(format!("{} is not a nominal service", canonical(p, &ty)));
    };
    if !matches!(p.ty(*id).body, hir::TypeBody::Custom(_)) {
        return Err(format!("{} is not a nominal service", canonical(p, &ty)));
    }
    if args.len() != p.ty(*id).params.len() {
        return Err(format!(
            "ill-kinded type arguments for {}",
            canonical(p, &ty)
        ));
    }
    Ok(Service {
        constructor: *id,
        arguments: args.clone(),
        name: canonical(p, &ty),
    })
}
fn as_type(s: &Service) -> ShownType {
    ShownType::Named(TypeRef::Custom(s.constructor), s.arguments.clone())
}
fn members(p: &Program, ty: &ShownType) -> Result<Vec<Service>, String> {
    let xs = match ty {
        ShownType::Never => return Ok(vec![]),
        ShownType::Union(xs) => xs.clone(),
        x => vec![x.clone()],
    };
    let mut out = vec![];
    for x in xs {
        let s = service(p, x)?;
        if p.module(p.ty(s.constructor).module).kind == ModuleKind::Prelude
            && p.ty(s.constructor).name == "Assert"
        {
            continue;
        }
        out.push(s);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    for pair in out.windows(2) {
        if pair[0].constructor == pair[1].constructor {
            return Err(format!(
                "duplicate or ambiguous supply {} and {}",
                pair[0].name, pair[1].name
            ));
        }
    }
    Ok(out)
}

#[derive(Clone)]
struct Row {
    declaration: ProviderRow,
    authority: PackageId,
    cell: Option<Cell>,
    factory: Option<DefId>,
    generated: Option<(hir::CtorRef, Vec<ShownType>)>,
    adapter: Option<DefId>,
}
#[derive(Clone)]
struct Adapter {
    binding: DefId,
    supplies: Vec<Service>,
    requires: Vec<Service>,
}
struct Schema {
    rows: HashMap<TypeId, Row>,
    adapters: HashMap<DefId, Adapter>,
    errors: Vec<(PackageId, Span, String)>,
}

// Parse plan type data using the same nominal identities and alias expansions
// as source inference. Plan names require qualified paths; prelude names are
// also allowed in closed arguments.
fn plan_type(
    p: &Program,
    t: &Typed,
    g: &PackageGraph,
    authority: PackageId,
    text: &str,
) -> Result<ShownType, String> {
    let text = text.trim();
    let split = |sep: char| -> Vec<&str> {
        let mut depth = 0;
        let mut start = 0;
        let mut out = vec![];
        for (i, c) in text.char_indices() {
            match c {
                '(' | '{' => depth += 1,
                ')' | '}' => depth -= 1,
                _ => {}
            }
            if c == sep && depth == 0 {
                out.push(text[start..i].trim());
                start = i + 1;
            }
        }
        out.push(text[start..].trim());
        out
    };
    for (sep, union) in [('|', true), ('&', false)] {
        let xs = split(sep);
        if xs.len() > 1 {
            let xs = xs
                .into_iter()
                .map(|x| plan_type(p, t, g, authority, x))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(normalize(
                p,
                if union {
                    ShownType::Union(xs)
                } else {
                    ShownType::Inter(xs)
                },
            ));
        }
    }
    let (name, args) = if let Some(open) = text.find('(') {
        if !text.ends_with(')') {
            return Err(format!("invalid closed type {text}"));
        }
        let inner = &text[open + 1..text.len() - 1];
        let mut depth = 0;
        let mut start = 0;
        let mut args = vec![];
        for (i, c) in inner.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    args.push(plan_type(p, t, g, authority, &inner[start..i])?);
                    start = i + 1;
                }
                _ => {}
            }
        }
        if !inner.is_empty() {
            args.push(plan_type(p, t, g, authority, &inner[start..])?);
        }
        (&text[..open], args)
    } else {
        (text, vec![])
    };
    if name == "_" || name.is_empty() {
        return Err(format!("type {text} must be closed"));
    }
    if let Some(b) = BuiltinType::ALL.into_iter().find(|b| b.name() == name) {
        if args.len() != b.arity() {
            return Err(format!("ill-kinded type arguments for {text}"));
        }
        return Ok(if b == BuiltinType::Never {
            ShownType::Never
        } else {
            ShownType::Named(TypeRef::Builtin(b), args)
        });
    }
    let id = if name.contains('/') {
        resolve_service(g, p, authority, name).map_err(|e| format!("{name}: {e:?}"))?
    } else {
        p.prelude
            .and_then(|m| {
                p.module(m)
                    .types
                    .iter()
                    .find(|id| p.ty(**id).name == name)
                    .copied()
            })
            .ok_or_else(|| format!("type {text} must be closed and resolved"))?
    };
    if args.len() != p.ty(id).params.len() {
        return Err(format!("ill-kinded type arguments for {text}"));
    }
    if let Some(alias) = t.alias_types.get(&id) {
        let vars: HashMap<_, _> = p
            .ty(id)
            .params
            .iter()
            .map(|(n, _)| n.clone())
            .zip(args)
            .collect();
        return Ok(normalize(p, substitute(alias, &vars)));
    }
    Ok(ShownType::Named(TypeRef::Custom(id), args))
}
fn substitute(t: &ShownType, vars: &HashMap<String, ShownType>) -> ShownType {
    match t {
        ShownType::Var(v) => vars.get(v).cloned().unwrap_or_else(|| t.clone()),
        ShownType::Named(r, xs) => {
            ShownType::Named(*r, xs.iter().map(|x| substitute(x, vars)).collect())
        }
        ShownType::Union(xs) => ShownType::Union(xs.iter().map(|x| substitute(x, vars)).collect()),
        ShownType::Inter(xs) => ShownType::Inter(xs.iter().map(|x| substitute(x, vars)).collect()),
        ShownType::Tuple(xs) => ShownType::Tuple(xs.iter().map(|x| substitute(x, vars)).collect()),
        ShownType::Diff(a, xs) => ShownType::Diff(
            Box::new(substitute(a, vars)),
            xs.iter().map(|x| substitute(x, vars)).collect(),
        ),
        _ => t.clone(),
    }
}
fn direct_binding(p: &Program, mut id: DefId) -> Result<DefId, String> {
    let mut seen = HashSet::new();
    while seen.insert(id) {
        let hir::DefKind::Binding { binding, .. } = p.def(id).kind else {
            return Err("adapter must name a top-level function binding".into());
        };
        let e = &p.module(id.module).bindings[binding as usize].binding.value;
        match e.kind {
            hir::ExprKind::Fn(_) => return Ok(id),
            hir::ExprKind::Name(hir::ValueRef::Def(next)) => id = next,
            _ => {
                return Err(
                    "adapter must name a top-level function or direct lexical alias".into(),
                );
            }
        }
    }
    Err("recursive adapter alias".into())
}

impl Schema {
    fn resolve(p: &Program, t: &Typed, g: &PackageGraph, target: Target) -> Self {
        let mut out = Self {
            rows: HashMap::new(),
            adapters: HashMap::new(),
            errors: vec![],
        };
        // Dependencies never participate in selection. Each inherited cell retains
        // the authority of its declaration, just as a layer retains its closure.
        let authorities = std::iter::once(g.library).chain(g.root.filter(|r| *r != g.library));
        for authority in authorities {
            let plan = &g.packages[authority].provider_plan;
            let mut adapters_seen = HashSet::new();
            for a in &plan.adapters {
                let result = (|| {
                    let binding = resolve_binding(g, p, authority, &a.function)
                        .map_err(|e| format!("{}: {e:?}", a.function))?;
                    let binding = direct_binding(p, binding)?;
                    if !adapters_seen.insert(binding) {
                        return Err(format!("duplicate adapter {}", a.function));
                    }
                    let parse = |xs: &[String]| -> Result<Vec<Service>, String> {
                        let mut services = vec![];
                        let mut seen = HashSet::new();
                        for x in xs {
                            let s = service(p, plan_type(p, t, g, authority, x)?)?;
                            if !seen.insert(s.constructor) {
                                return Err(format!("duplicate or ambiguous supply {}", s.name));
                            }
                            services.push(s);
                        }
                        Ok(services)
                    };
                    let supplies = parse(&a.supplies)?;
                    let requires = parse(&a.requires)?;
                    t.provider_adapter(
                        binding,
                        &supplies.iter().map(as_type).collect::<Vec<_>>(),
                        &requires.iter().map(as_type).collect::<Vec<_>>(),
                    )?;
                    if let Some(previous) = out.adapters.get(&binding) {
                        if previous.supplies != supplies || previous.requires != requires {
                            return Err(format!("ambiguous adapter declaration {}", a.function));
                        }
                    }
                    out.adapters.insert(
                        binding,
                        Adapter {
                            binding,
                            supplies,
                            requires,
                        },
                    );
                    Ok(())
                })();
                if let Err(e) = result {
                    out.errors
                        .push((authority, a.span, format!("adapter {}: {e}", a.function)));
                }
            }
            let mut seen = HashSet::new();
            for row in &plan.rows {
                let result = (|| {
                    let mut id = resolve_service(g, p, authority, &row.service)
                        .map_err(|e| format!("{}: {e:?}", row.service))?;
                    if let Some(ShownType::Named(TypeRef::Custom(alias), args)) =
                        t.alias_types.get(&id)
                    {
                        if args.len() != p.ty(id).params.len()
                            || args.iter().any(|arg| !matches!(arg, ShownType::Var(_)))
                        {
                            return Err(format!(
                                "row {} must name a service constructor",
                                row.service
                            ));
                        }
                        id = *alias;
                    }
                    if !matches!(p.ty(id).body, hir::TypeBody::Custom(_)) {
                        return Err(format!(
                            "row {} must name a nominal service constructor",
                            row.service
                        ));
                    }
                    if !seen.insert(id) {
                        return Err(format!("duplicate supply {}", row.service));
                    }
                    let adapter = row
                        .adapter
                        .as_ref()
                        .map(|name| {
                            resolve_binding(g, p, authority, name)
                                .map_err(|e| format!("{name}: {e:?}"))
                                .and_then(|id| direct_binding(p, id))
                        })
                        .transpose()?;
                    if (row.role == Role::None) != adapter.is_none() {
                        return Err(format!(
                            "row {} role requires exactly one declared adapter",
                            row.service
                        ));
                    }
                    let cell = row.cells.get(&target).cloned();
                    let mut factory = None;
                    for (&column, candidate) in &row.cells {
                        match candidate {
                            Cell::Factory {
                                factory: path,
                                arguments,
                                ..
                            } => {
                                let binding = resolve_binding(g, p, authority, path)
                                    .map_err(|e| format!("{path}: {e:?}"))?;
                                t.provider_factory_schema(
                                    binding,
                                    id,
                                    p.ty(id).params.len(),
                                    arguments,
                                    p,
                                )?;
                                if column == target {
                                    factory = Some(binding);
                                }
                            }
                            Cell::Host if p.ty(id).host.is_none() => {
                                return Err(format!(
                                    "host cell {} must name a declared host service",
                                    row.service
                                ));
                            }
                            Cell::Host => {}
                        }
                    }
                    let mut selected = Row {
                        declaration: row.clone(),
                        authority,
                        cell,
                        factory,
                        generated: None,
                        adapter,
                    };
                    if let Some(inherited) = out.rows.get(&id) {
                        let meta = &inherited.declaration;
                        if (row.role_declared && row.role != meta.role)
                            || (adapter.is_some() && adapter != inherited.adapter)
                            || (!row.discriminant.is_empty()
                                && row.discriminant != meta.discriminant)
                        {
                            return Err(format!(
                                "row {} changes inherited role, adapter or discriminant",
                                row.service
                            ));
                        }
                        selected.declaration.role = meta.role;
                        selected.declaration.adapter = meta.adapter.clone();
                        selected.declaration.discriminant = meta.discriminant.clone();
                        selected.adapter = inherited.adapter;
                        selected.authority = inherited.authority;
                        if selected.cell.is_none() {
                            selected.cell = inherited.cell.clone();
                            selected.factory = inherited.factory;
                        }
                    }
                    out.rows.insert(id, selected);
                    Ok(())
                })();
                if let Err(e) = result {
                    out.errors
                        .push((authority, row.span, format!("service {}: {e}", row.service)));
                }
            }
        }
        // Validate discriminants even when a target cell overrides generation.
        for (&id, row) in &mut out.rows {
            for (&column, path) in &row.declaration.discriminant {
                let result = (|| {
                    let (m, name) =
                        module(g, p, row.authority, path).map_err(|e| format!("{path}: {e:?}"))?;
                    if m != p.ty(id).module {
                        return Err(format!(
                            "constructor {path} does not belong to {}",
                            row.declaration.service
                        ));
                    }
                    let hir::TypeBody::Custom(ctors) = &p.ty(id).body else {
                        unreachable!("resolved nominal row")
                    };
                    let (index, ctor) = ctors
                        .iter()
                        .enumerate()
                        .find(|(_, c)| c.name == name)
                        .ok_or_else(|| format!("unknown constructor {path}"))?;
                    if p.module(m).package != row.authority && !p.ty(id).constructors_exported {
                        return Err(format!("constructor {path}: Private"));
                    }
                    let mut fields = vec![];
                    for f in &ctor.fields {
                        fields.push(as_type(&service(p, hir_type(p, t, &f.ty)?)?));
                    }
                    if column == target && row.cell.is_none() {
                        row.generated = Some((
                            hir::CtorRef {
                                ty: id,
                                index: index as u32,
                            },
                            fields,
                        ));
                    }
                    Ok(())
                })();
                if let Err(e) = result {
                    out.errors.push((row.authority, row.declaration.span, e));
                }
            }
            if let Some(adapter) = row.adapter
                && !out.adapters.contains_key(&adapter)
            {
                out.errors.push((
                    row.authority,
                    row.declaration.span,
                    format!(
                        "service {} names undeclared adapter {}.{}",
                        row.declaration.service,
                        p.module(adapter.module).path,
                        p.def(adapter).name
                    ),
                ));
            }
        }
        out
    }
}

fn hir_type(p: &Program, t: &Typed, ty: &hir::Type) -> Result<ShownType, String> {
    Ok(match &ty.kind {
        hir::TypeKind::Named { ty, args } => {
            let args = args
                .iter()
                .map(|a| hir_type(p, t, a))
                .collect::<Result<Vec<_>, _>>()?;
            if let TypeRef::Custom(id) = ty
                && let Some(alias) = t.alias_types.get(id)
            {
                let vars = p
                    .ty(*id)
                    .params
                    .iter()
                    .map(|(n, _)| n.clone())
                    .zip(args)
                    .collect();
                substitute(alias, &vars)
            } else {
                ShownType::Named(*ty, args)
            }
        }
        hir::TypeKind::Union(xs) => ShownType::Union(
            xs.iter()
                .map(|x| hir_type(p, t, x))
                .collect::<Result<_, _>>()?,
        ),
        hir::TypeKind::Inter(xs) => ShownType::Inter(
            xs.iter()
                .map(|x| hir_type(p, t, x))
                .collect::<Result<_, _>>()?,
        ),
        hir::TypeKind::Tuple(xs) => ShownType::Tuple(
            xs.iter()
                .map(|x| hir_type(p, t, x))
                .collect::<Result<_, _>>()?,
        ),
        hir::TypeKind::Var(v) => ShownType::Var(p.ty_var_names[v.0 as usize].clone()),
        _ => return Err("generated constructor fields must be closed service types".into()),
    })
}

struct Node {
    step: ConstructionStep,
    deps: Vec<usize>,
    seed: bool,
}
struct Builder<'a> {
    schema: &'a Schema,
    p: &'a Program,
    t: &'a Typed,
    entry: DefId,
    target: Target,
    nodes: Vec<Node>,
    indices: HashMap<(Scope, TypeId, Vec<ShownType>), usize>,
    adapter: Option<DefId>,
    adapter_requires: Vec<usize>,
    adapter_origin: Option<String>,
}
impl Builder<'_> {
    fn add(&mut self, s: Service, scope: Scope, chain: &mut Vec<Service>) -> Result<usize, String> {
        let trail = |chain: &[Service]| {
            chain
                .iter()
                .map(|s| s.name.as_str())
                .chain([s.name.as_str()])
                .collect::<Vec<_>>()
                .join(" -> ")
        };
        if chain
            .iter()
            .any(|parent| parent.constructor == s.constructor)
        {
            return Err(format!("recursive dependency: {}", trail(chain)));
        }
        if let Some(&id) = self
            .indices
            .get(&(scope, s.constructor, s.arguments.clone()))
        {
            return Ok(id);
        }
        let row = self.schema.rows.get(&s.constructor);
        let host = self.p.ty(s.constructor).host.as_ref();
        let role = row.map_or(Role::None, |r| r.declaration.role);
        let mut generated_args = vec![];
        let mut seed = false;
        let (supply, needs) = match row {
            Some(row) => match &row.cell {
                Some(Cell::Host) => (Supply::Host, vec![]),
                Some(Cell::Factory {
                    arguments,
                    seed_required,
                    ..
                }) => {
                    let factory = row.factory.expect("validated factory cell");
                    let mut args = vec![];
                    for arg in arguments {
                        match arg {
                            Argument::Seed => {
                                // The complete adapter is discovered before seed edges
                                // are finalized, since another root may select its role.
                                generated_args.push(ResolvedArgument::Seed(None));
                                args.push(ShownType::Named(
                                    TypeRef::Builtin(BuiltinType::List),
                                    vec![as_type(&s)],
                                ));
                                seed = true;
                            }
                            Argument::EntryRootKey => {
                                generated_args.push(ResolvedArgument::EntryRootKey);
                                let nil = self
                                    .p
                                    .prelude
                                    .and_then(|m| {
                                        self.p
                                            .module(m)
                                            .types
                                            .iter()
                                            .find(|id| self.p.ty(**id).name == "Nil")
                                    })
                                    .expect("prelude Nil");
                                args.push(ShownType::Named(
                                    TypeRef::Builtin(BuiltinType::Key),
                                    vec![ShownType::Named(TypeRef::Custom(*nil), vec![])],
                                ));
                            }
                        }
                    }
                    if *seed_required && scope == Scope::Outer {
                        return Err(format!(
                            "required seed is unavailable in outer scope: {}",
                            trail(chain)
                        ));
                    }
                    let env = self
                        .t
                        .provider_factory(factory, &as_type(&s), &args)
                        .map_err(|e| format!("{}: {e}; needs {}", s.name, trail(chain)))?;
                    (
                        Supply::Factory(factory),
                        members(self.p, &env)
                            .map_err(|e| format!("{e}; needs {}", trail(chain)))?,
                    )
                }
                None => {
                    if let Some((ctor, fields)) = &row.generated {
                        (
                            Supply::Constructor(*ctor),
                            fields
                                .iter()
                                .cloned()
                                .map(|x| service(self.p, x))
                                .collect::<Result<_, _>>()?,
                        )
                    } else {
                        return Err(format!(
                            "missing service {}; needs {}",
                            s.name,
                            trail(chain)
                        ));
                    }
                }
            },
            None if host.is_some() => (Supply::Host, vec![]),
            None => {
                return Err(format!(
                    "missing service {}; needs {}",
                    s.name,
                    trail(chain)
                ));
            }
        };
        let id = self.nodes.len();
        self.indices
            .insert((scope, s.constructor, s.arguments.clone()), id);
        self.nodes.push(Node {
            step: ConstructionStep {
                service: s.clone(),
                supply,
                arguments: generated_args,
                scope,
                role,
                request_mode: host.is_some_and(|h| h.is_request || h.is_async),
            },
            deps: vec![],
            seed,
        });
        if let Some(adapter) = row.and_then(|r| r.adapter) {
            if let Some(previous) = self.adapter {
                if previous != adapter {
                    return Err(format!(
                        "ambiguous adapters: {} selects {}.{}, {} selects {}.{}; needs {}",
                        self.adapter_origin.as_ref().unwrap(),
                        self.p.module(previous.module).path,
                        self.p.def(previous).name,
                        s.name,
                        self.p.module(adapter.module).path,
                        self.p.def(adapter).name,
                        trail(chain)
                    ));
                }
            } else {
                self.adapter = Some(adapter);
                self.adapter_origin = Some(s.name.clone());
            }
        }
        chain.push(s);
        let deps = needs
            .into_iter()
            .map(|need| self.add(need, scope, chain))
            .collect::<Result<Vec<_>, _>>()?;
        chain.pop();
        self.nodes[id].deps = deps;
        Ok(id)
    }
    fn build(&mut self, needs: &ShownType) -> Result<ResolvedPlan, String> {
        for s in members(self.p, needs)? {
            self.add(s, Scope::Body, &mut vec![])?;
        }
        if let Some(binding) = self.adapter {
            let adapter = &self.schema.adapters[&binding];
            for s in adapter.requires.clone() {
                let id = self.add(s, Scope::Outer, &mut vec![])?;
                self.adapter_requires.push(id);
            }
        }
        // Completed nodes are shared, but a repeated constructor must still be
        // rejected on every path, including paths through a memoized diamond.
        fn visit(nodes: &[Node], id: usize, path: &mut Vec<usize>) -> Result<(), String> {
            if path
                .iter()
                .any(|&i| nodes[i].step.service.constructor == nodes[id].step.service.constructor)
            {
                return Err(format!(
                    "recursive dependency: {}",
                    path.iter()
                        .copied()
                        .chain([id])
                        .map(|i| nodes[i].step.service.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" -> ")
                ));
            }
            path.push(id);
            for &dep in &nodes[id].deps {
                visit(nodes, dep, path)?;
            }
            path.pop();
            Ok(())
        }
        for id in 0..self.nodes.len() {
            visit(&self.nodes, id, &mut vec![])?;
        }
        let mut seeded = HashSet::new();
        if let Some(a) = self.adapter {
            seeded.extend(
                self.schema.adapters[&a]
                    .supplies
                    .iter()
                    .map(|s| (s.constructor, s.arguments.clone())),
            );
        }
        for n in &mut self.nodes {
            let supplied = n.step.scope == Scope::Body
                && seeded.contains(&(n.step.service.constructor, n.step.service.arguments.clone()));
            if n.seed {
                let required = self
                    .schema
                    .rows
                    .get(&n.step.service.constructor)
                    .and_then(|r| r.cell.as_ref())
                    .is_some_and(|c| {
                        matches!(
                            c,
                            Cell::Factory {
                                seed_required: true,
                                ..
                            }
                        )
                    });
                if required && !supplied {
                    return Err(format!(
                        "required seed unavailable for {}",
                        n.step.service.name
                    ));
                }
                for arg in &mut n.step.arguments {
                    if let ResolvedArgument::Seed(a) = arg {
                        *a = if supplied { self.adapter } else { None };
                    }
                }
            }
            n.seed = n.seed && supplied;
        }
        let mut done = HashSet::new();
        let mut order = vec![];
        let mut enter_at = None;
        loop {
            let mut ready: Vec<_> = (0..self.nodes.len())
                .filter(|i| {
                    !done.contains(i)
                        && self.nodes[*i].deps.iter().all(|d| done.contains(d))
                        && (!self.nodes[*i].seed || enter_at.is_some())
                })
                .collect();
            ready.sort_by(|a, b| {
                let a = &self.nodes[*a].step;
                let b = &self.nodes[*b].step;
                (!matches!(a.supply, Supply::Host), &a.service.name, a.scope).cmp(&(
                    !matches!(b.supply, Supply::Host),
                    &b.service.name,
                    b.scope,
                ))
            });
            if let Some(&i) = ready.first() {
                done.insert(i);
                order.push(i);
                continue;
            }
            if enter_at.is_none()
                && self.adapter.is_some()
                && self.adapter_requires.iter().all(|d| done.contains(d))
            {
                enter_at = Some(order.len());
                continue;
            }
            if order.len() == self.nodes.len() {
                break;
            }
            return Err(format!(
                "provider cycle through adapter/seed: {}",
                self.nodes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !done.contains(i))
                    .map(|(_, n)| n.step.service.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ));
        }
        let positions: HashMap<_, _> = order.iter().enumerate().map(|(at, &id)| (id, at)).collect();
        let mut steps = vec![];
        for id in order {
            let n = &self.nodes[id];
            let mut step = n.step.clone();
            step.arguments.extend(
                n.deps
                    .iter()
                    .map(|d| ResolvedArgument::Service(positions[d])),
            );
            steps.push(step);
        }
        let adapter = self.adapter.map(|a| ResolvedAdapter {
            binding: self.schema.adapters[&a].binding,
            supplies: self.schema.adapters[&a].supplies.clone(),
            requires: self.adapter_requires.iter().map(|i| positions[i]).collect(),
            enter_at: enter_at.expect("selected adapter entered"),
        });
        let is_async = self.target != Target::Wasi
            && steps.iter().any(|s| s.request_mode || s.role != Role::None);
        let needs_entry_root_key = steps
            .iter()
            .any(|s| s.arguments.contains(&ResolvedArgument::EntryRootKey));
        Ok(ResolvedPlan {
            entry: self.entry,
            target: self.target,
            steps,
            adapter,
            needs_entry_root_key,
            is_async,
        })
    }
}
