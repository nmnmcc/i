//! Name resolution, the order of top-level values, and type inference: from
//! the parsed modules of a package graph to the resolved program (HIR) and
//! its types.
//!
//! The only authority is `docs/spec.md`; section numbers refer to it.

pub mod hir;
pub mod order;
pub mod query;
pub mod resolve;
pub mod types;

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
    // Attributes and entry points.
    pub const UNKNOWN_ATTRIBUTE: &str = "E0301";
    pub const ATTRIBUTE_ARGS: &str = "E0302";
    pub const ATTRIBUTE_PLACE: &str = "E0303";
    pub const ENTRY_FORM: &str = "E0304";
    pub const TEST_OUTSIDE_TEST: &str = "E0305";
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
    pub const BIT_PATTERN: &str = "E0519";
    pub const EQ_DISJOINT: &str = "E0520";
    pub const ENTRY_TYPE: &str = "E0521";
    pub const ENTRY_ENV: &str = "E0522";
    pub const NEVER_USED: &str = "W0501";
}

/// A parsed module of the graph.
pub struct Parsed {
    pub package: PackageId,
    pub path: String,
    pub source: SourceId,
    pub kind: ModuleKind,
    pub ast: ast::Module,
}

/// Parses every module of the graph. Syntax errors keep their codes.
pub fn parse(graph: &PackageGraph) -> (Vec<Parsed>, Vec<Diagnostic>) {
    let mut out = Vec::new();
    let mut diags = Vec::new();
    for (package, p) in graph.packages.iter().enumerate() {
        for m in &p.modules {
            let (ast, errors) = ilang_syntax::parse_module(&graph.sources.get(m.source).text);
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
            });
        }
    }
    (out, diags)
}

/// Parses, resolves, and orders the whole program. Resolution is skipped
/// when a module has a syntax error, so that one mistake is not reported
/// many times.
pub fn resolve_program(graph: &PackageGraph) -> (Option<hir::Program>, Vec<Diagnostic>) {
    let (parsed, mut diags) = parse(graph);
    if diags.iter().any(Diagnostic::is_error) {
        return (None, diags);
    }
    let (mut program, more) = resolve::resolve(graph, &parsed);
    diags.extend(more);
    order::order(&mut program, &mut diags);
    (Some(program), diags)
}

/// The result of `check`.
pub struct Checked {
    /// `None` when a module has a syntax error.
    pub program: Option<hir::Program>,
    /// `None` when the program could not be resolved.
    pub types: Option<types::Typed>,
    pub diags: Vec<Diagnostic>,
}

/// Parses, resolves, orders, and type checks the whole program. Types are
/// only inferred when resolution found no errors, so that one mistake is
/// not reported many times.
pub fn check(graph: &PackageGraph) -> Checked {
    let (program, mut diags) = resolve_program(graph);
    let types = match &program {
        Some(p) if !diags.iter().any(Diagnostic::is_error) => {
            let (typed, more) = types::check(p, &graph.sources);
            diags.extend(more);
            Some(typed)
        }
        _ => None,
    };
    Checked {
        program,
        types,
        diags,
    }
}
