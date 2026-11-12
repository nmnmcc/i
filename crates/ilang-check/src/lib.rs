//! Name resolution, the order of top-level values, and type inference: from
//! the parsed modules of a package graph to the resolved program (HIR) and
//! its types.
//!
//! The only authority is `docs/spec.md`; section numbers refer to it.

mod capabilities;
pub mod hir;
pub mod hygiene;
pub mod identity;
pub mod macro_sandbox;
pub mod order;
pub mod providers;
pub mod query;
pub mod resolve;
pub mod types;
mod unused;

use ilang_package::{Diagnostic, ModuleKind, PackageGraph, PackageId, SourceId};
use ilang_syntax::ast;

/// The stable codes of the problems found by this crate.
pub mod code {
    // Names, imports, exports, patterns, and types.
    pub const UNKNOWN_NAME: &str = "E0201";
    pub const UNKNOWN_TYPE: &str = "E0202";
    pub const NO_MEMBER: &str = "E0203";
    pub const NOT_EXPORTED: &str = "E0204";
    pub const MODULE_AS_VALUE: &str = "E0205";
    pub const NOT_A_CONSTRUCTOR: &str = "E0206";
    pub const NOT_A_MODULE: &str = "E0207";
    pub const DEFINED_TWICE: &str = "E0208";
    pub const UNKNOWN_PACKAGE: &str = "E0209";
    pub const NO_MODULE: &str = "E0210";
    pub const SRC_IMPORTS_TEST: &str = "E0211";
    pub const IMPORTED_TWICE: &str = "E0212";
    pub const IMPORT_IS_TOP_LEVEL: &str = "E0213";
    pub const IMPORT_CIRCLE: &str = "E0214";
    pub const EXPORT_UNDEFINED: &str = "E0215";
    // E0216 is no longer given: `Alias(..)` is the same as `Alias` (§2.4).
    pub const PATTERN_BINDS_TWICE: &str = "E0217";
    pub const PARAMS_BIND_TWICE: &str = "E0218";
    pub const PARAMS_SAME_LABEL: &str = "E0219";
    pub const CHOICES_DIFFER: &str = "E0220";
    pub const TYPE_PARAM_UNUSED: &str = "E0221";
    pub const TYPE_VAR_NOT_PARAM: &str = "E0222";
    pub const ALIAS_CIRCLE: &str = "E0223";
    pub const TYPE_ARITY: &str = "E0224";
    pub const ENV_TYPE: &str = "E0225";
    pub const TYPE_PARAM_TWICE: &str = "E0226";
    pub const EXPORT_HIDDEN_TYPE: &str = "E0227";
    pub const ENV_PARAMS_SAME: &str = "E0228";
    pub const TYPE_VAR_ARGS: &str = "E0230";
    pub const TYPE_VAR_IS_TYPE: &str = "E0231";
    pub const TYPE_PARAM_FORM: &str = "E0232";
    pub const MACRO_RESERVED: &str = "E0234";
    pub const MACRO_BINDER: &str = "E0235";
    pub const MANY_CONSTRUCTORS: &str = "E0238";
    // E0229, E0233 and E0237 were temporary value-type reservations.
    pub const VALUE_TYPE_CIRCLE: &str = "E0530";
    pub const VALUE_TYPE_PLACE: &str = "E0531";
    pub const TYPE_PATTERN_FORM: &str = "E0532";
    pub const VALUE_TYPE_OPEN: &str = "E0533";
    pub const VALUE_TYPE_NOT_FIXED: &str = "E0534";
    pub const TYPE_PATTERN_MISMATCH: &str = "E0535";
    pub const VALUE_TYPE_OWN_SCOPE: &str = "E0536";
    pub const TYPE_PATTERN_BINDER_IS_TYPE: &str = "E0537";
    // Attributes and entry points.
    pub const UNKNOWN_ATTRIBUTE: &str = "E0301";
    pub const ATTRIBUTE_ARGS: &str = "E0302";
    pub const ATTRIBUTE_PLACE: &str = "E0303";
    pub const ENTRY_FORM: &str = "E0304";
    pub const RESERVED_ILANG: &str = "E0306";
    pub const MAIN_TWICE: &str = "E0307";
    pub const DECLARATION_WITHOUT_EXTERNAL: &str = "E0308";
    pub const ATTRIBUTE_TWICE: &str = "E0309";
    pub const HOST_FORM: &str = "E0310";
    pub const HOST_TWICE: &str = "E0311";
    pub const ENTRY_PARAMS: &str = "E0312";
    pub const WASM_EXPORT_UNKNOWN: &str = "E0313";
    pub const WASM_EXPORT_ENTRY: &str = "E0314";
    pub const EXTERNAL_NOT_FN: &str = "E0315";
    pub const EXTERNAL_HOLE: &str = "E0316";
    pub const EXTERNAL_TYPE_VAR: &str = "E0317";
    pub const EXTERNAL_HOST_TYPE: &str = "E0318";
    pub const EXTERNAL_EFFECTS: &str = "E0319";
    pub const COMPILER_CAPABILITY: &str = "E0322";
    pub const HOST_TYPE: &str = "E0320";
    pub const EXPORT_NAME_TWICE: &str = "E0321";
    pub const DEPRECATED: &str = "W0301";
    // Order.
    pub const NEEDS_ITSELF: &str = "E0401";
    // Types (§4, §9).
    pub const TYPE_MISMATCH: &str = "E0501";
    pub const FN_ARITY: &str = "E0502";
    pub const TYPE_INVARIANT: &str = "E0503";
    pub const TYPE_AMBIGUOUS: &str = "E0504";
    pub const TYPE_OCCURS: &str = "E0505";
    pub const NEEDS_KNOWN_TYPE: &str = "E0506";
    pub const CALL_LABELS: &str = "E0507";
    // E0508 is no longer given: every value with a known type (§4.8) may
    // use `.f` and `.k`, and E0506 is given for one with no known type.
    pub const TOP_LEVEL_EFFECT: &str = "E0509";
    pub const PATTERN_FIELDS: &str = "E0510";
    pub const CASE_TYPE_VAR: &str = "E0511";
    pub const NO_FIELD: &str = "E0512";
    pub const TYPE_DIFF_VAR: &str = "E0513";
    pub const BAD_FORMAT: &str = "E0514";
    pub const ENV_FORMS: &str = "E0515";
    pub const CASE_NOT_COVERED: &str = "E0516";
    pub const NOT_KNOWN_CTOR: &str = "E0517";
    pub const TYPE_PLACE: &str = "E0518";
    pub const TYPE_RESOURCE_LIMIT: &str = "E0546";
    pub const LABEL_MAY_BE: &str = "E0544";
    pub const DUPLICATE_RECORD_LABEL: &str = "E0540";
    pub const INVALID_LABEL_PARAMETER: &str = "E0543";
    pub const INVALID_RECORD_KEY: &str = "E0541";
    pub const LABEL_PARAMETER_VALUE: &str = "E0542";
    pub const LABEL_ESCAPES: &str = "E0545";
    pub const BIT_PATTERN: &str = "E0519";
    // E0520 became warning W0503: disjoint equality is well defined (§6.9).
    pub const ENTRY_TYPE: &str = "E0521";
    pub const NEVER_USED: &str = "W0501";
    pub const VALUE_NOT_USED: &str = "W0502";
    pub const EQ_DISJOINT: &str = "W0503";
    pub const UNUSED_BINDING: &str = "W0504";
    pub const PATTERN_DISJOINT: &str = "W0505";
}

/// A parsed module of the graph.
#[derive(Clone)]
pub struct Parsed {
    pub package: PackageId,
    pub path: String,
    pub source: SourceId,
    pub kind: ModuleKind,
    pub ast: ast::Module,
    /// Origins of names introduced by expansion in this module.
    pub scoped: Option<ilang_syntax::lisp::scoped::ScopedSource>,
    /// This module was built by the compiler and may call compiler internals.
    pub compiler_generated: bool,
}

/// Parses every module of the graph. Syntax errors keep their codes.
pub fn parse(graph: &PackageGraph) -> (Vec<Parsed>, Vec<Diagnostic>) {
    let mut out = Vec::new();
    let mut diags = Vec::new();
    for (package, p) in graph.packages.iter().enumerate() {
        for m in &p.modules {
            let source = graph.sources.get(m.source);
            let (ast, errors) = if source.display.ends_with(".is") {
                ilang_syntax::lisp::parse_source(&source.text)
            } else {
                ilang_syntax::parse_module(&source.text)
            };
            diags.extend(
                errors
                    .into_iter()
                    .map(|d| Diagnostic::error(d.code.as_str(), Some(m.source), d.span, d.message)),
            );
            out.push(Parsed {
                package,
                path: m.path.clone(),
                source: m.source,
                kind: m.kind,
                ast,
                scoped: None,
                compiler_generated: false,
            });
        }
    }
    (out, diags)
}

/// Parses, resolves, and orders the whole program. Resolution is skipped
/// when a module has a syntax error, so that one mistake is not reported
/// many times.
pub fn resolve_program(graph: &PackageGraph) -> (Option<hir::Program>, Vec<Diagnostic>) {
    let (mut parsed, diags) = parse(graph);
    if diags.iter().any(Diagnostic::is_error) {
        return (None, diags);
    }
    // Direct checking resolves quotes in the same form as the expansion driver.
    let mut templates = Vec::new();
    for module in &mut parsed {
        ilang_syntax::lisp::quote_lower::lower_expression_quotes(
            &mut module.ast,
            module.source.0,
            &mut templates,
        );
    }
    let (program, more) = resolve_parsed(graph, &parsed);
    (program, diags.into_iter().chain(more).collect())
}

/// Resolve an already parsed module set. The expansion driver uses this entry
/// point after transforming syntax, keeping source identities and spans intact.
pub fn resolve_parsed(
    graph: &PackageGraph,
    parsed: &[Parsed],
) -> (Option<hir::Program>, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let (mut program, more) = resolve::resolve(graph, parsed);
    diags.extend(more);
    order::order(&mut program, &mut diags);
    capabilities::check(&program, graph, &mut diags);
    (Some(program), diags)
}

/// Resolve a compiler-owned program from the exact import closure of a macro
/// module. The supplied entries are compiler-generated wrappers: they expose
/// host-compatible Syntax values and install the allowed compiler services.
/// Ordinary program capability checks do not apply to these wrappers; the
/// complete dependency closure is checked for foreign externals instead.
pub fn check_compiler_exports(
    graph: &PackageGraph,
    parsed: &[Parsed],
    entries: &[(&str, &str)],
) -> Checked {
    let (mut program, mut diags) = resolve::resolve(graph, parsed);
    order::order(&mut program, &mut diags);
    for module in &mut program.modules {
        for binding in &mut module.bindings {
            binding.main = false;
        }
    }
    program.host_exports.clear();
    for &(path, name) in entries {
        let (module, m) = program
            .modules
            .iter()
            .enumerate()
            .find(|(_, m)| m.path == path)
            .expect("compiler export module must be in the selected import closure");
        let index = m
            .defs
            .iter()
            .position(|d| d.name == name)
            .expect("compiler export must be a resolved wrapper binding");
        program.host_exports.push(hir::DefId {
            module: hir::ModuleId(module as u32),
            index: index as u32,
        });
    }
    let modules = (0..program.modules.len())
        .map(|i| hir::ModuleId(i as u32))
        .collect::<Vec<_>>();
    for id in
        macro_sandbox::foreign_externals(&program, graph.library, &modules, &program.host_exports)
    {
        let def = program.def(id);
        diags.push(Diagnostic::error(
            code::COMPILER_CAPABILITY,
            Some(program.module(id.module).source),
            def.span,
            "a macro program cannot execute an external declaration outside package i (§8)",
        ));
    }
    let mut checked = check_resolved(graph, Some(program), diags);
    if let (Some(program), Some(typed)) = (&checked.program, &checked.types) {
        for &entry in &program.host_exports {
            if typed
                .entry_services
                .get(&entry)
                .is_some_and(|services| !services.is_empty())
            {
                checked.diags.push(Diagnostic::error(
                    code::COMPILER_CAPABILITY,
                    Some(program.module(entry.module).source),
                    program.def(entry).span,
                    "a compiler wrapper must install its allowed services and cannot request a host service (§8)",
                ));
            }
        }
    }
    checked
}

/// The result of `check`.
pub struct Checked {
    /// `None` when a module has a syntax error.
    pub program: Option<hir::Program>,
    /// `None` when the program could not be resolved, or the equivalence
    /// audit needs a known type before it can supply lowering data.
    pub types: Option<types::Typed>,
    pub diags: Vec<Diagnostic>,
}

/// Parses, resolves, orders, and type checks the whole program. Types are
/// only inferred when resolution found no errors, so that one mistake is
/// not reported many times.
pub fn check(graph: &PackageGraph) -> Checked {
    let (program, diags) = resolve_program(graph);
    check_resolved(graph, program, diags)
}

/// Resolve and type-check the parsed modules after syntax expansion.
pub fn check_parsed(graph: &PackageGraph, parsed: &[Parsed]) -> Checked {
    let (program, diags) = resolve_parsed(graph, parsed);
    check_resolved(graph, program, diags)
}

/// Resolve and infer a project for collection discovery, without selecting
/// configured program entries or host exports as tool execution roots.
pub fn check_parsed_for_tools(graph: &PackageGraph, parsed: &[Parsed]) -> Checked {
    let (mut program, mut diags) = resolve::resolve(graph, parsed);
    order::order(&mut program, &mut diags);
    for module in &mut program.modules {
        for binding in &mut module.bindings {
            binding.main = false;
        }
    }
    program.host_exports.clear();
    check_resolved(graph, Some(program), diags)
}

fn check_resolved(
    graph: &PackageGraph,
    program: Option<hir::Program>,
    diags: Vec<Diagnostic>,
) -> Checked {
    check_resolved_selected(graph, program, diags, &[])
}

pub fn check_resolved_selected(
    graph: &PackageGraph,
    program: Option<hir::Program>,
    mut diags: Vec<Diagnostic>,
    selected: &[(hir::DefId, bool)],
) -> Checked {
    if let Some(p) = &program {
        let targets: Vec<_> = selected.iter().map(|(target, _)| *target).collect();
        capabilities::check_selected(p, graph, &targets, &mut diags);
    }
    let types = match &program {
        Some(p) if !diags.iter().any(Diagnostic::is_error) => {
            let (typed, more) = types::check_selected(p, &graph.sources, selected);
            diags.extend(more);
            Some(typed)
        }
        _ => None,
    };
    if let Some(p) = &program {
        map_expansion_diagnostics(p, &graph.sources, &mut diags);
    }
    Checked {
        program,
        types,
        diags,
    }
}

/// Validate a consumer-selected entry after the project has been checked.
/// The returned type data includes the selected entry's inferred services.
pub fn check_selected(
    graph: &PackageGraph,
    program: &mut hir::Program,
    target: hir::DefId,
    require_nil: bool,
) -> (types::Typed, Vec<Diagnostic>) {
    check_selected_many(graph, program, &[(target, require_nil)])
}

pub fn check_selected_many(
    graph: &PackageGraph,
    program: &mut hir::Program,
    selected: &[(hir::DefId, bool)],
) -> (types::Typed, Vec<Diagnostic>) {
    let (typed, mut diags) = types::check_selected(program, &graph.sources, selected);
    let targets: Vec<_> = selected.iter().map(|(target, _)| *target).collect();
    capabilities::check_selected(program, graph, &targets, &mut diags);
    map_expansion_diagnostics(program, &graph.sources, &mut diags);
    (typed, diags)
}

/// Map diagnostics from the checker's virtual expansion text to written
/// source. Warnings from templates are not emitted (§9).
pub fn map_expansion_diagnostics(
    program: &hir::Program,
    sources: &ilang_package::Sources,
    diags: &mut Vec<Diagnostic>,
) {
    diags.retain_mut(|d| {
        let Some(source) = d.source else { return true };
        let Some(index) = program.modules.iter().position(|m| m.source == source) else {
            return true;
        };
        let Some(origin) = program.origin(hir::ModuleId(index as u32), d.span) else {
            return true;
        };
        if origin.template.is_some() && d.severity == ilang_package::Severity::Warning {
            return false;
        }
        if let Some((template_source, template_span)) = origin.template {
            let virtual_text = &sources.get(source).text;
            let expanded = virtual_text
                .get(d.span.start as usize..d.span.end as usize)
                .expect("diagnostic span is inside expanded source");
            let name = origin
                .expansion
                .call_chain
                .last()
                .map_or("macro", |call| call.macro_name.as_str());
            d.message
                .push_str(&format!(" (in the expansion of `{name}`: `{expanded}`)"));
            d.related.push(ilang_package::DiagnosticNote {
                source: template_source,
                span: template_span,
                message: "expanded from this template".into(),
            });
            for call in origin.expansion.call_chain.iter().skip(1) {
                d.related.push(ilang_package::DiagnosticNote {
                    source: SourceId(call.syntax.scope().module),
                    span: call.syntax.span(),
                    message: format!("in the expansion of `{}`", call.macro_name),
                });
            }
        }
        d.source = Some(origin.source);
        d.span = origin.span;
        true
    });
}
