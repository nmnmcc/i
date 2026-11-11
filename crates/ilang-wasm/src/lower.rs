//! From the resolved program (HIR) and its types to the intermediate form
//! of `ir.rs`.
//!
//! - Every function expression becomes a function of the intermediate form
//!   with explicit captures (§6.8). A top-level function is one value (§2.5,
//!   §11.4): a constant closure, and its known calls are direct.
//! - Constructors (§6.3) and library functions used as values are functions
//!   with one constant closure each, so they have one identity (§11.4).
//! - Evaluation order is that of §6.2, and only the tail positions of §11.6
//!   become tail calls.
//! - `case` (§6.12) tests the clauses one after the other, each pattern with
//!   plain tests (§7.2, §8.4).
//! - Every call, operator, and `<<` of the program source that may stop has a
//!   site on the shadow stack (§11.7). A call in tail position, also of a
//!   library function, has a tail site: it is only the first position.
//! - Top-level values are worked out in `ilang_start`, in `order.init` (§2.5).
//! - Every call passes the environment of its place (§9.2): environment
//!   parameters read a slot of it, and environment statements make a new one
//!   for the rest of their block.
//! - `fail.raise`, `fail.catch`, `resource.ensure`, and the four functions
//!   of `i/control` (§9.4) are operations of the code generator (`emit.rs`);
//!   `fail.from_result`, `fail.map_error`, `resource.acquire`,
//!   `assertion.catch`, and `assert` are i code.
//! - A binding or a parameter whose pattern may fail (the checker decides,
//!   §7.3) calls `fail` of the `Assert` service when it does not match
//!   (§6.10, §6.11).
//! - The host boundary (§9.7, §10, Appendix B): each external function
//!   outside the library, and each field of a host service that an entry
//!   point needs, is an import with a function that calls it
//!   ([`Value::HostCall`]). The host services are records of these functions
//!   in the root environment. A function of an async host service starts
//!   its import and suspends to the root boundary of its async entry point
//!   ([`Value::HostStart`]); when the host answers, it reads the answer
//!   ([`Value::HostReply`]). An entry point is async when an async host
//!   service is in its environment.
//!
//! The calls that may suspend are found and split afterwards (`control.rs`).

use crate::ir::Stmt;
use crate::ir::{self, *};
use ilang_check::hir::{self, *};
use ilang_check::types::{Access, Head, HostType, LitValue, TestMember, TestValues, Typed};
use ilang_package::{Diagnostic, ModuleKind, PackageGraph, PackageId};
use ilang_syntax::ast::{BinOp, UnOp};
use ilang_syntax::span::{LineIndex, Span};
use num_bigint::BigInt;
use std::collections::{BTreeSet, HashMap, HashSet};

/// The code of the problems found by the code generator.
pub mod code {
    /// A feature that the compiler does not support yet.
    pub const NOT_SUPPORTED: &str = "E0701";
}

/// The first constructor id and type id that the code generator gives.
const FIRST_ID: u32 = crate::runtime::FIRST_CONSTRUCTOR;

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Compile the tests of the compiled package and its dev-dependencies.
    pub tests: bool,
    /// The host the module is made for (B.7).
    pub target: ir::Target,
}

/// The WASI functions that the runtime imports on the WASI target (B.7).
const WASI: &str = "wasi_snapshot_preview1";
const WASI_RUNTIME: [&str; 2] = ["fd_write", "proc_exit"];

/// How a constructor is represented (`runtime/mod.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CtorRepr {
    Bool(bool),
    Nil,
    Record { ctor: u32, ty: u32 },
}

#[derive(Clone, Debug)]
struct CtorInfo {
    repr: CtorRepr,
    labels: Vec<Option<String>>,
}

/// How a custom type is tested at run time (§7.1).
#[derive(Clone, Copy, Debug)]
enum TypeRepr {
    Bool,
    Nil,
    Record(u32),
    /// A type with no constructors: its values are host references.
    Host,
}

/// What a callee is.
enum Target {
    Ctor(CtorRef),
    Op(Op),
    Callee(Callee),
}

/// The library functions whose bodies are in the runtime (§13).
const CORE: &[&str] = &[
    "not",
    "to_float",
    "truncate",
    "floor",
    "ceiling",
    "sqrt",
    "float_to_bits",
    "float_from_bits",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "shift_left",
    "shift_right",
    "concat",
    "byte_size",
    "byte_slice",
    "codepoints",
    "from_codepoints",
    "codepoint_to_int",
    "int_to_codepoint",
    "bit_size",
    "bit_array_to_string",
    "inspect",
    "hash",
    "to_upper",
    "to_lower",
    "index_of",
    "bit_array_concat",
];

const CORE_FUNCS: &[&str] = &[
    "core_not",
    "core_to_float",
    "core_truncate",
    "core_floor",
    "core_ceiling",
    "core_sqrt",
    "core_float_to_bits",
    "core_float_from_bits",
    "core_bit_and",
    "core_bit_or",
    "core_bit_xor",
    "core_bit_not",
    "core_shift_left",
    "core_shift_right",
    "core_concat",
    "core_byte_size",
    "core_byte_slice",
    "core_codepoints",
    "core_from_codepoints",
    "core_codepoint_to_int",
    "core_int_to_codepoint",
    "core_bit_size",
    "core_bit_array_to_string",
    "core_inspect",
    "core_hash",
    "core_to_upper",
    "core_to_lower",
    "core_index_of",
    "core_bit_array_concat",
];

/// The state of one function while it is lowered.
struct Fx {
    module: ModuleId,
    /// Code of the library: it has no sites (§11.7).
    library: bool,
    locals: Vec<LocalType>,
    vars: HashMap<VarId, Local>,
    labels: u32,
    /// The names of the binding whose direct value is being lowered (§6.10).
    pending: Option<HashSet<VarId>>,
    /// Captures to fill once those names are bound.
    patches: Vec<(Local, u32, VarId)>,
    /// The environment of the place being lowered (§9.2).
    env: Env,
}

impl Fx {
    fn new(module: ModuleId, library: bool) -> Fx {
        Fx {
            module,
            library,
            locals: Vec::new(),
            vars: HashMap::new(),
            labels: 0,
            pending: None,
            patches: Vec::new(),
            env: Env::Param,
        }
    }

    fn local(&mut self, t: LocalType) -> Local {
        self.locals.push(t);
        self.locals.len() as Local - 1
    }

    fn label(&mut self) -> Label {
        self.labels += 1;
        self.labels - 1
    }

    fn var(&mut self, v: VarId) -> Local {
        if let Some(&l) = self.vars.get(&v) {
            return l;
        }
        let l = self.local(LocalType::Value);
        self.vars.insert(v, l);
        l
    }
}

fn let_value(fx: &mut Fx, out: &mut Vec<Stmt>, value: Value, site: Option<Site>) -> Atom {
    let dst = fx.local(LocalType::Value);
    out.push(Stmt::Let { dst, value, site });
    Atom::Local(dst)
}

fn let_num(fx: &mut Fx, out: &mut Vec<Stmt>, value: NumValue, site: Option<Site>) -> Num {
    let dst = fx.local(LocalType::I64);
    out.push(Stmt::LetNum { dst, value, site });
    Num::Local(dst)
}

fn finish(out: &mut Vec<Stmt>, tail: bool, a: Atom) -> Option<Atom> {
    if tail {
        out.push(Stmt::Return(a));
        None
    } else {
        Some(a)
    }
}

/// `if not cond, leave to fail`.
fn guard(out: &mut Vec<Stmt>, cond: Cond, fail: Label) {
    out.push(Stmt::If {
        cond: Cond::Not(Box::new(cond)),
        then: vec![Stmt::Break(fail)],
        otherwise: vec![],
    });
}

/// `{ e }` with one expression is `e` (§6.10).
fn ungroup(e: &Expr) -> &Expr {
    match &e.kind {
        ExprKind::Block(b) if b.stmts.len() == 1 => match &b.stmts[0] {
            hir::Stmt::Expr(inner) => ungroup(inner),
            _ => e,
        },
        _ => e,
    }
}

/// The names that a pattern binds.
fn pattern_vars(p: &Pattern, out: &mut HashSet<VarId>) {
    let mut w = Walk::default();
    w.pattern(p);
    out.extend(w.bound);
}

/// A source-level bound on values simultaneously pending while evaluating
/// an expression. Unlike Wasm locals this is independent of code generation
/// and optimization. Earlier call arguments remain pending while later ones
/// (including a recursive call) are evaluated.
fn pending_values<'a>(values: impl IntoIterator<Item = &'a Expr>) -> usize {
    let mut held = 0;
    let mut peak = 1;
    for value in values {
        peak = peak.max(held + expression_values(value));
        held += 1;
    }
    peak.max(held)
}

fn pattern_values(p: &Pattern) -> usize {
    match &p.kind {
        PatternKind::Named { pattern, .. } => 1 + pattern_values(pattern),
        PatternKind::Constructor { args, .. }
        | PatternKind::Tuple(args)
        | PatternKind::Error(args) => 1 + args.iter().map(pattern_values).sum::<usize>(),
        PatternKind::List { items, .. } => 1 + items.iter().map(pattern_values).sum::<usize>(),
        PatternKind::BitArray(segments) => {
            1 + segments
                .iter()
                .map(|s| {
                    pattern_values(&s.value)
                        + s.format
                            .as_ref()
                            .and_then(|f| f.arg.as_deref())
                            .map_or(0, expression_values)
                })
                .sum::<usize>()
        }
        PatternKind::Var(_)
        | PatternKind::Discard
        | PatternKind::Typed { .. }
        | PatternKind::Int { .. }
        | PatternKind::Float { .. }
        | PatternKind::String(_)
        | PatternKind::Prefix { .. } => 1,
    }
}

fn block_values(block: &hir::Block) -> usize {
    block
        .stmts
        .iter()
        .map(|stmt| match stmt {
            hir::Stmt::Bind(binding) => {
                expression_values(&binding.value).max(pattern_values(&binding.pattern))
            }
            hir::Stmt::Env { value, .. } | hir::Stmt::Expr(value) => expression_values(value),
        })
        .max()
        .unwrap_or(1)
}

fn expression_values(e: &Expr) -> usize {
    match &e.kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name(_)
        | ExprKind::Error
        | ExprKind::Fn(_) => 1,
        ExprKind::Tuple(items) => pending_values(items),
        ExprKind::List { items, rest } => pending_values(items.iter().chain(rest.as_deref())),
        ExprKind::BitArray(segments) => pending_values(segments.iter().flat_map(|s| {
            std::iter::once(&s.value).chain(s.format.as_ref().and_then(|f| f.arg.as_deref()))
        })),
        ExprKind::Block(block) => block_values(block),
        ExprKind::Case { subject, clauses } => expression_values(subject).max(
            1 + clauses
                .iter()
                .map(|clause| {
                    clause
                        .patterns
                        .iter()
                        .map(pattern_values)
                        .max()
                        .unwrap_or(1)
                        .max(clause.guard.as_ref().map_or(1, expression_values))
                        .max(expression_values(&clause.body))
                })
                .max()
                .unwrap_or(1),
        ),
        ExprKind::Call { callee, args } => {
            pending_values(std::iter::once(callee.as_ref()).chain(args.iter().map(|a| &a.value)))
        }
        ExprKind::Field { target, .. }
        | ExprKind::TupleIndex { target, .. }
        | ExprKind::Unary { expr: target, .. } => expression_values(target),
        ExprKind::RecordUpdate { base, fields, .. } => pending_values(
            std::iter::once(base.as_ref()).chain(fields.iter().map(|(_, value)| value)),
        ),
        ExprKind::Binary { lhs, rhs, .. } => pending_values([lhs.as_ref(), rhs.as_ref()]),
    }
}

/// Collects the names used and bound in a part of the program.
#[derive(Default)]
struct Walk {
    used: BTreeSet<VarId>,
    bound: HashSet<VarId>,
}

impl Walk {
    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(ValueRef::Local(v)) => {
                self.used.insert(*v);
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name(_)
            | ExprKind::Error => {}
            ExprKind::Tuple(items) => items.iter().for_each(|x| self.expr(x)),
            ExprKind::List { items, rest } => items
                .iter()
                .chain(rest.as_deref())
                .for_each(|x| self.expr(x)),
            ExprKind::BitArray(segs) => {
                for s in segs {
                    self.expr(&s.value);
                    if let Some(a) = s.format.as_ref().and_then(|f| f.arg.as_deref()) {
                        self.expr(a);
                    }
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fn(f) => self.function(f),
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                for c in clauses {
                    c.patterns.iter().for_each(|p| self.pattern(p));
                    c.guard.iter().for_each(|g| self.expr(g));
                    self.expr(&c.body);
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target)
            }
            ExprKind::RecordUpdate { base, fields, .. } => {
                self.expr(base);
                fields.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
        }
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            match s {
                hir::Stmt::Bind(b) => {
                    self.pattern(&b.pattern);
                    self.expr(&b.value);
                }
                hir::Stmt::Env { value, .. } => self.expr(value),
                hir::Stmt::Expr(e) => self.expr(e),
            }
        }
    }

    fn function(&mut self, f: &Function) {
        for p in &f.params {
            match p {
                Param::Pattern { pattern, .. } => self.pattern(pattern),
                Param::Env { var, .. } => {
                    self.bound.insert(*var);
                }
            }
        }
        self.block(&f.body);
    }

    fn binder(&mut self, b: &Binder) {
        self.bound.insert(b.var);
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Quote { captures, .. } => {
                captures.iter().for_each(|(_, p)| self.pattern(p))
            }
            PatternKind::Var(b) => self.binder(b),
            PatternKind::Typed { binder, .. } => binder.iter().for_each(|b| self.binder(b)),
            PatternKind::Named { binder, pattern } => {
                self.binder(binder);
                self.pattern(pattern);
            }
            PatternKind::Prefix { rest, .. } => rest.iter().for_each(|b| self.binder(b)),
            PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                args.iter().for_each(|a| self.pattern(a))
            }
            PatternKind::List { items, rest } => {
                items.iter().for_each(|a| self.pattern(a));
                if let Some(Some(b)) = rest {
                    self.binder(b);
                }
            }
            PatternKind::BitArray(segs) => {
                for s in segs {
                    self.pattern(&s.value);
                    if let Some(a) = s.format.as_ref().and_then(|f| f.arg.as_deref()) {
                        self.expr(a);
                    }
                }
            }
            PatternKind::Error(parts) => parts.iter().for_each(|a| self.pattern(a)),
            PatternKind::Discard
            | PatternKind::Int { .. }
            | PatternKind::Float { .. }
            | PatternKind::String(_) => {}
        }
    }
}

/// The names that a function uses from outside it, in a fixed order.
fn captures_of(f: &Function) -> Vec<VarId> {
    let mut w = Walk::default();
    w.function(f);
    w.used
        .into_iter()
        .filter(|v| !w.bound.contains(v))
        .collect()
}

/// A bit array format (§8.2).
enum Fmt<'e> {
    Int {
        signed: bool,
        width: Width<'e>,
        little: bool,
    },
    Float {
        width: u32,
        little: bool,
    },
    Bits(Option<&'e Expr>),
    Bytes(Option<&'e Expr>),
    Utf {
        encoding: u32,
        little: bool,
        codepoint: bool,
    },
}

enum Width<'e> {
    Const(u32),
    Dyn(&'e Expr),
}

/// The format of a segment. Formats were checked by the type checker.
fn format(f: Option<&Format>) -> Option<Fmt<'_>> {
    let Some(f) = f else {
        return Some(Fmt::Int {
            signed: false,
            width: Width::Const(8),
            little: false,
        });
    };
    let name = f.name.name.as_str();
    let arg = f.arg.as_deref();
    Some(match name {
        "bits" => Fmt::Bits(arg),
        "bytes" => Fmt::Bytes(arg),
        "u" | "i" => Fmt::Int {
            signed: name == "i",
            width: Width::Dyn(arg?),
            little: false,
        },
        _ => {
            let (base, codepoint) = match name.strip_suffix("_codepoint") {
                Some(b) => (b, true),
                None => (name, false),
            };
            let (body, little) = if let Some(b) = base.strip_suffix("le") {
                (b, true)
            } else if let Some(b) = base.strip_suffix("be") {
                (b, false)
            } else {
                (base, false)
            };
            if let Some(n) = body.strip_prefix("utf") {
                Fmt::Utf {
                    encoding: n.parse().ok()?,
                    little,
                    codepoint,
                }
            } else if let Some(n) = body.strip_prefix('f') {
                Fmt::Float {
                    width: n.parse().ok()?,
                    little,
                }
            } else if let Some(n) = body.strip_prefix('u') {
                Fmt::Int {
                    signed: false,
                    width: Width::Const(n.parse().ok()?),
                    little,
                }
            } else {
                Fmt::Int {
                    signed: true,
                    width: Width::Const(body.strip_prefix('i')?.parse().ok()?),
                    little,
                }
            }
        }
    })
}

/// Lowers a checked program. The program must have no errors.
pub fn lower(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
) -> Result<ir::Program, Vec<Diagnostic>> {
    lower_with_bindings(graph, program, typed, options).map(|(ir, _)| ir)
}

/// Capture every required resolved declaration before import coalescing. This
/// plan is returned separately; it changes neither WAT nor ilang.abi (native 3.1).
pub fn lower_with_bindings(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
) -> Result<(ir::Program, Vec<serde_json::Value>), Vec<Diagnostic>> {
    if !typed.coalescing_errors().is_empty() {
        return Err(typed.coalescing_errors().to_vec());
    }
    let mut l = Lowerer::new(graph, program, typed, options);
    l.run();
    if l.diags.is_empty() {
        crate::control::split(&mut l.out);
        Ok((l.out, l.bindings))
    } else {
        Err(l.diags)
    }
}

type Entry = (String, bool);
type HostExport = (String, ExportAbi, bool);
type Entries = (Option<Entry>, Vec<Entry>, Vec<HostExport>);

struct Lowerer<'a> {
    graph: &'a PackageGraph,
    program: &'a hir::Program,
    typed: &'a Typed,
    options: Options,
    packages: HashSet<PackageId>,
    ctors: HashMap<CtorRef, CtorInfo>,
    types: HashMap<TypeId, TypeRepr>,
    /// The top-level bindings whose value is a function, by their name.
    known: HashMap<DefId, BindingRef>,
    def_funcs: HashMap<DefId, FuncId>,
    def_globals: HashMap<DefId, GlobalId>,
    ctor_funcs: HashMap<CtorRef, FuncId>,
    op_funcs: HashMap<String, FuncId>,
    work: Vec<(FuncId, DefId)>,
    constants: BTreeSet<FuncId>,
    site_map: HashMap<SitePos, Site>,
    module_index: HashMap<ModuleId, u32>,
    lines: HashMap<ModuleId, LineIndex<'a>>,
    /// The service id of each service type constructor (§9.2).
    services: HashMap<TypeRef, u32>,
    /// The function that calls each import with its declared result decoder (B.4).
    import_funcs: HashMap<(ImportId, AbiType), FuncId>,
    /// Where each import is declared first, by its index in `out.imports`.
    import_sites: Vec<(ModuleId, Span)>,
    bindings: Vec<serde_json::Value>,
    /// The function of each external function outside the library (§10.1),
    /// or `None` when it has a problem, which is told one time.
    external_funcs: HashMap<DefId, Option<FuncId>>,
    out: ir::Program,
    diags: Vec<Diagnostic>,
}

impl<'a> Lowerer<'a> {
    fn new(
        graph: &'a PackageGraph,
        program: &'a hir::Program,
        typed: &'a Typed,
        options: Options,
    ) -> Self {
        let mut l = Lowerer {
            graph,
            program,
            typed,
            options,
            packages: HashSet::new(),
            ctors: HashMap::new(),
            types: HashMap::new(),
            known: HashMap::new(),
            def_funcs: HashMap::new(),
            def_globals: HashMap::new(),
            ctor_funcs: HashMap::new(),
            op_funcs: HashMap::new(),
            work: Vec::new(),
            constants: BTreeSet::new(),
            site_map: HashMap::new(),
            module_index: HashMap::new(),
            lines: HashMap::new(),
            services: HashMap::new(),
            import_funcs: HashMap::new(),
            import_sites: Vec::new(),
            bindings: Vec::new(),
            external_funcs: HashMap::new(),
            out: ir::Program::default(),
            diags: Vec::new(),
        };
        l.find_packages();
        l.number_types();
        l.number_services();
        l
    }

    /// One dense id for each service type constructor of the program:
    /// `Assert` first, then the types of the environment parameters and
    /// statements (§9.2).
    fn number_services(&mut self) {
        let prelude = self.program.prelude;
        let assert = self
            .program
            .types
            .iter()
            .position(|t| Some(t.module) == prelude && t.name == "Assert")
            .map(|i| TypeRef::Custom(TypeId(i as u32)));
        let mut all: Vec<TypeRef> = self.typed.services.values().copied().collect();
        let key = |t: &TypeRef| match t {
            TypeRef::Custom(id) => (0, id.0),
            TypeRef::Builtin(b) => (1, *b as u32),
        };
        all.sort_by_key(key);
        all.dedup();
        if let Some(a) = assert {
            self.services.insert(a, ASSERT_SERVICE);
        }
        let mut next = ASSERT_SERVICE + 1;
        for t in all {
            if let std::collections::hash_map::Entry::Vacant(e) = self.services.entry(t) {
                e.insert(next);
                next += 1;
            }
        }
        self.out.services = next;
    }

    /// The service id of the environment parameter or statement at `span`.
    fn service(&self, module: ModuleId, span: Span) -> u32 {
        self.typed
            .services
            .get(&(module, span))
            .and_then(|t| self.services.get(t))
            .copied()
            .expect("a checked environment parameter or statement has a service")
    }

    /// The packages that are compiled: the library, the compiled package,
    /// and what it uses; its dev-dependencies only with the tests (§2.6).
    fn find_packages(&mut self) {
        let g = self.graph;
        self.packages.insert(g.library);
        let mut stack: Vec<PackageId> = Vec::new();
        if let Some(root) = g.root {
            stack.push(root);
            if self.options.tests {
                stack.extend(g.packages[root].dev_dependencies.iter().filter_map(|d| d.1));
            }
        }
        while let Some(p) = stack.pop() {
            if self.packages.insert(p) {
                stack.extend(g.packages[p].dependencies.iter().filter_map(|d| d.1));
            }
        }
    }

    fn included(&self, m: ModuleId) -> bool {
        let module = self.program.module(m);
        self.packages.contains(&module.package)
            && (module.kind != ModuleKind::Test || self.options.tests)
    }

    fn is_library(&self, m: ModuleId) -> bool {
        self.program.module(m).package == self.graph.library
    }

    fn number_types(&mut self) {
        let prelude = self.program.prelude;
        let mut next_ctor = FIRST_ID;
        for (i, t) in self.program.types.iter().enumerate() {
            let id = TypeId(i as u32);
            let TypeBody::Custom(ctors) = &t.body else {
                continue;
            };
            let special = if Some(t.module) == prelude {
                match t.name.as_str() {
                    "Bool" => Some(TypeRepr::Bool),
                    "Nil" => Some(TypeRepr::Nil),
                    "Result" => Some(TypeRepr::Record(0)),
                    "AssertionFailure" => Some(TypeRepr::Record(1)),
                    "Assert" => Some(TypeRepr::Record(2)),
                    _ => None,
                }
            } else {
                None
            };
            let repr = match special {
                Some(r) => r,
                None if ctors.is_empty() => TypeRepr::Host,
                None => TypeRepr::Record(FIRST_ID + i as u32),
            };
            self.types.insert(id, repr);
            for (j, c) in ctors.iter().enumerate() {
                let r = CtorRef {
                    ty: id,
                    index: j as u32,
                };
                let repr = match (special, c.name.as_str()) {
                    (Some(TypeRepr::Bool), "True") => CtorRepr::Bool(true),
                    (Some(TypeRepr::Bool), _) => CtorRepr::Bool(false),
                    (Some(TypeRepr::Nil), _) => CtorRepr::Nil,
                    (Some(TypeRepr::Record(ty)), name) => CtorRepr::Record {
                        ctor: match name {
                            "Ok" => 0,
                            "Error" => 1,
                            "AssertionFailure" => 2,
                            _ => 3,
                        },
                        ty,
                    },
                    _ => {
                        let TypeRepr::Record(ty) = repr else {
                            unreachable!("a type with constructors is a record")
                        };
                        self.out.constructors.push(c.name.clone());
                        next_ctor += 1;
                        CtorRepr::Record {
                            ctor: next_ctor - 1,
                            ty,
                        }
                    }
                };
                let labels = c
                    .fields
                    .iter()
                    .map(|f| f.label.as_ref().map(|l| l.name.clone()))
                    .collect();
                self.ctors.insert(r, CtorInfo { repr, labels });
            }
        }
    }

    fn error(&mut self, module: ModuleId, span: Span, message: impl Into<String>) {
        let source = self.program.module(module).source;
        self.diags.push(Diagnostic::error(
            code::NOT_SUPPORTED,
            Some(source),
            span,
            message,
        ));
    }

    fn run(&mut self) {
        // Top-level functions are known functions with one value.
        for (mi, m) in self.program.modules.iter().enumerate() {
            for (bi, b) in m.bindings.iter().enumerate() {
                let one_name = matches!(
                    b.binding.pattern.kind,
                    PatternKind::Var(_)
                        | PatternKind::Typed {
                            binder: Some(_),
                            ..
                        }
                );
                if one_name
                    && b.defs.len() == 1
                    && matches!(ungroup(&b.binding.value).kind, ExprKind::Fn(_))
                {
                    self.known.insert(
                        b.defs[0],
                        BindingRef {
                            module: ModuleId(mi as u32),
                            index: bi as u32,
                        },
                    );
                }
            }
        }
        // §10.1, B.6: every external function of the compiled packages is
        // an import.
        for (mi, m) in self.program.modules.iter().enumerate() {
            let mid = ModuleId(mi as u32);
            if !self.included(mid) || self.is_library(mid) {
                continue;
            }
            for (di, d) in m.defs.iter().enumerate() {
                if matches!(d.kind, DefKind::External { .. }) {
                    self.external_func(DefId {
                        module: mid,
                        index: di as u32,
                    });
                }
            }
        }
        self.start();
        let (entry, tests, exports) = self.entries();
        while let Some((id, d)) = self.work.pop() {
            let b = self.known[&d];
            let tb = self.program.binding(b);
            let ExprKind::Fn(f) = &ungroup(&tb.binding.value).kind else {
                unreachable!()
            };
            let name = format!(
                "{}.{}",
                self.program.module(d.module).path,
                self.program.def(d).name
            );
            let func = self.function(b.module, f, &[], name);
            self.out.funcs[id.0 as usize] = FuncKind::Code(func);
        }
        self.out.abi_json = abi_json(
            self.options.target,
            entry,
            &tests,
            &exports,
            &self.out.imports,
        );
        self.out.constants = self.constants.iter().copied().collect();
    }

    /// `ilang_start`: the top-level values, in the order of §2.5.
    fn start(&mut self) {
        let mut fx = Fx::new(ModuleId(0), false);
        let mut out = Vec::new();
        let mut pending = 1;
        for &b in &self.program.order.init {
            if !self.included(b.module) {
                continue;
            }
            let tb = self.program.binding(b);
            if tb.defs.len() == 1 && self.known.contains_key(&tb.defs[0]) {
                continue;
            }
            pending = pending
                .max(expression_values(&tb.binding.value))
                .max(pattern_values(&tb.binding.pattern));
            fx.module = b.module;
            fx.library = self.is_library(b.module);
            let value = self.expr(&mut fx, &mut out, &tb.binding.value);
            self.bind_checked(
                &mut fx,
                &mut out,
                &tb.binding.pattern,
                value,
                tb.binding.span.start,
            );
            for &d in &tb.defs {
                if let DefKind::Binding { var, .. } = self.program.def(d).kind {
                    let g = self.global(d);
                    let l = fx.var(var);
                    out.push(Stmt::SetGlobal(g, Atom::Local(l)));
                }
            }
        }
        out.push(Stmt::Return(Atom::Nil));
        self.out.start = Some(Func {
            name: "ilang_start".into(),
            arity: 0,
            budget_frame: u32::try_from(1 + fx.vars.len() + pending)
                .expect("source frame exceeds u32"),
            locals: fx.locals,
            body: out,
            frame: true,
        });
    }

    fn global(&mut self, d: DefId) -> GlobalId {
        if let Some(&g) = self.def_globals.get(&d) {
            return g;
        }
        let g = GlobalId(self.out.globals);
        self.out.globals += 1;
        self.def_globals.insert(d, g);
        g
    }

    /// The program entry, the tests, and the host exports (§10.2, B.5),
    /// and the host services that they need (§9.7).
    fn entries(&mut self) -> Entries {
        let program = self.program;
        let mut entry = None;
        let mut tests = Vec::new();
        let mut needed: BTreeSet<TypeId> = BTreeSet::new();
        for (mi, m) in program.modules.iter().enumerate() {
            let mid = ModuleId(mi as u32);
            if Some(m.package) != program.root || !self.included(mid) {
                continue;
            }
            for b in &m.bindings {
                if !(b.main || b.test) || b.defs.len() != 1 {
                    continue;
                }
                let d = b.defs[0];
                let name = format!("{}.{}", m.path, program.def(d).name);
                let callee = self.def_callee(d);
                let fx = Fx::new(mid, false);
                let site = self.site(&fx, b.binding.span.start);
                let is_async = self.entry_async(d, &mut needed);
                let async_body = is_async.then(|| self.async_body(&callee, 0));
                self.out.exports.push(Export {
                    name: name.clone(),
                    callee,
                    abi: None,
                    site,
                    async_body,
                });
                if b.main {
                    entry = Some((name, is_async));
                } else {
                    tests.push((name, is_async));
                }
            }
        }
        let mut exports = Vec::new();
        for &d in &program.host_exports {
            let module = program.module(d.module);
            let name = format!("{}.{}", module.path, program.def(d).name);
            let Some(sig) = self.typed.host_exports.get(&d) else {
                self.error(
                    d.module,
                    program.def(d).span,
                    format!("the host export `{name}` has a type that the host cannot take"),
                );
                continue;
            };
            let abi = ExportAbi {
                params: sig.params.iter().map(|t| abi_type(*t)).collect(),
                result: abi_type(sig.result),
                errors: sig.errors.iter().map(|t| abi_type(*t)).collect(),
            };
            let callee = self.def_callee(d);
            let is_async = self.entry_async(d, &mut needed);
            let async_body = is_async.then(|| self.async_body(&callee, abi.params.len() as u32));
            exports.push((name.clone(), abi.clone(), is_async));
            self.out.exports.push(Export {
                name,
                callee,
                abi: Some(abi),
                site: None,
                async_body,
            });
        }
        for t in needed {
            self.host_service(t);
        }
        if self.options.target == ir::Target::Wasi {
            match &entry {
                Some((name, _)) => self.out.entry = Some(name.clone()),
                None => self.diags.push(Diagnostic::error(
                    code::NOT_SUPPORTED,
                    None,
                    Span::default(),
                    "the WASI target needs a program entry: `main` in `src/main.i`, or select `[entries]` with `--entry` (B.7)",
                )),
            }
        }
        self.out.target = self.options.target;
        (entry, tests, exports)
    }

    /// Whether an entry point is async: it may call a function of an async
    /// host service (§10.2). Every such call gets the service from the
    /// environment, and only the host gives it at an entry point, so this
    /// is when an async host service is in the environment of its type. The
    /// host services of the environment go to `needed`.
    fn entry_async(&self, d: DefId, needed: &mut BTreeSet<TypeId>) -> bool {
        let Some(services) = self.typed.entry_services.get(&d) else {
            return false;
        };
        needed.extend(services.iter().copied());
        services.iter().any(|t| {
            self.program
                .ty(*t)
                .host
                .as_ref()
                .is_some_and(|h| self.host_is_async(h))
        })
    }

    /// Whether a host service is async in the module: on the WASI target
    /// every host function is a blocking call (B.7).
    fn host_is_async(&self, host: &hir::Host) -> bool {
        host.is_async && self.options.target == ir::Target::Ilang
    }

    /// The body of an async entry point inside its root boundary (B.5): a
    /// function of the boundary, whose captures are the arguments.
    fn async_body(&mut self, callee: &Callee, n: u32) -> FuncId {
        let mut body = Vec::new();
        for i in 0..n {
            body.push(Stmt::Let {
                dst: 1 + i,
                value: Value::Capture(i),
                site: None,
            });
        }
        body.push(Stmt::TailCall {
            callee: callee.clone(),
            args: (1..=n).map(Atom::Local).collect(),
            env: Env::Param,
            site: None,
        });
        let f = self.new_func();
        self.out.funcs[f.0 as usize] = FuncKind::Code(Func {
            name: "async entry".into(),
            arity: 1,
            budget_frame: 2,
            locals: vec![LocalType::Value; 1 + n as usize],
            body,
            frame: true,
        });
        f
    }

    /// A host service in the environment of an entry point (§9.7): one
    /// import for each field, and its value in the root environment.
    fn host_service(&mut self, t: TypeId) {
        let program = self.program;
        let def = program.ty(t);
        let (Some(host), TypeBody::Custom(ctors)) = (&def.host, &def.body) else {
            return;
        };
        let [c] = ctors.as_slice() else {
            return;
        };
        let Some(&service) = self.services.get(&TypeRef::Custom(t)) else {
            // No code takes it from the environment.
            return;
        };
        let CtorRepr::Record { ctor, ty } = self.ctors[&CtorRef { ty: t, index: 0 }].repr else {
            return;
        };
        let kind = if self.host_is_async(host) {
            ImportKind::AsyncService
        } else if self.options.target == ir::Target::Wasi && is_standard_service(program, t) {
            ImportKind::WasiStdout
        } else {
            ImportKind::Service
        };
        let mut fields = Vec::new();
        for (i, f) in c.fields.iter().enumerate() {
            let (Some(label), Some(hf)) = (&f.label, program.host_fns.get(&(t, i as u32))) else {
                return;
            };
            let abi = host_abi(hf);
            let site = (def.module, label.span);
            match self.import(
                &host.namespace,
                &label.name,
                abi,
                kind,
                site,
                &format!("{}.{}", def.name, label.name),
            ) {
                Ok(import) => {
                    let f = self.import_func(import, hf.result);
                    self.constants.insert(f);
                    fields.push(f);
                }
                Err((module, span, message)) => {
                    self.error(module, span, message);
                    return;
                }
            }
        }
        self.out.host_services.push(HostService {
            service,
            ctor,
            ty,
            fields,
        });
    }

    /// The function of an external function outside the library (§10.1),
    /// which calls its import.
    fn external_func(&mut self, d: DefId) -> Option<FuncId> {
        if let Some(&f) = self.external_funcs.get(&d) {
            return f;
        }
        let f = self.make_external_func(d);
        self.external_funcs.insert(d, f);
        f
    }

    fn make_external_func(&mut self, d: DefId) -> Option<FuncId> {
        let def = self.program.def(d);
        let DefKind::External {
            module, function, ..
        } = &def.kind
        else {
            return None;
        };
        let Some(hf) = self.program.external_fns.get(&d) else {
            self.error(
                d.module,
                def.span,
                "the type of this external function is not one that the host can take (§10.1)",
            );
            return None;
        };
        let site = (d.module, def.span);
        match self.import(
            module,
            function,
            host_abi(hf),
            ImportKind::External,
            site,
            &def.name,
        ) {
            Ok(import) => {
                // §11.4: each declaration is one value, so two declarations
                // of one import have two functions, which call the same
                // import.
                Some(self.make_import_func(import, abi_type(hf.result)))
            }
            Err((module, span, message)) => {
                self.error(module, span, message);
                None
            }
        }
    }

    /// The import with this module and name (B.4), declared at `site`.
    /// One import may be declared more than once, with one type. A problem
    /// is told at `site`, or, when two declarations do not agree and
    /// `site` is in the library, at the other declaration, which the
    /// program has in its source.
    fn import(
        &mut self,
        module: &str,
        name: &str,
        abi: ExportAbi,
        kind: ImportKind,
        site: (ModuleId, Span),
        declaration: &str,
    ) -> Result<ImportId, (ModuleId, Span, String)> {
        if module == "ilang:host/v1" {
            return Err((
                site.0,
                site.1,
                format!("the import module `{module}` is the module of the runtime imports (B.4)"),
            ));
        }
        if self.options.target == ir::Target::Wasi && module == WASI && WASI_RUNTIME.contains(&name)
        {
            return Err((
                site.0,
                site.1,
                format!(
                    "on the WASI target, the runtime imports `{module}` `{name}` itself, so a program may not import it (B.7)"
                ),
            ));
        }
        if kind != ImportKind::WasiStdout {
            let source_module = self.program.module(site.0);
            let package = &self.graph.packages[source_module.package];
            use ilang_package::lock::{FileKind, PackageFile, hash};
            let module_file = PackageFile {
                path: source_module.path.clone(),
                kind: FileKind::Regular,
                bytes: self
                    .graph
                    .sources
                    .get(source_module.source)
                    .text
                    .as_bytes()
                    .to_vec(),
            };
            let mut files: Vec<_> = package
                .modules
                .iter()
                .map(|m| PackageFile {
                    path: m.path.clone(),
                    kind: FileKind::Regular,
                    bytes: self.graph.sources.get(m.source).text.as_bytes().to_vec(),
                })
                .collect();
            if let Some(source) = package.manifest_source {
                files.push(PackageFile {
                    path: "i.toml".into(),
                    kind: FileKind::Regular,
                    bytes: self.graph.sources.get(source).text.as_bytes().to_vec(),
                });
            }
            let binding = serde_json::json!({
                "package": package.location.as_ref().map(|p| p.hash.clone()).unwrap_or_else(|| hash(&files)),
                "module": source_module.path,
                "module_digest": hash(&[module_file]),
                "declaration": declaration,
                "namespace": module,
                "name": name,
                "kind": if kind == ImportKind::External { "external" } else { "service" },
                "signature": {
                    "params": abi.params.iter().map(|t| t.name()).collect::<Vec<_>>(),
                    "result": abi.result.name(),
                    "errors": abi.errors.iter().map(|t| t.name()).collect::<Vec<_>>(),
                    "async": kind == ImportKind::AsyncService,
                },
            });
            if !self.bindings.contains(&binding) {
                self.bindings.push(binding);
            }
        }
        if let Some(i) = self
            .out
            .imports
            .iter()
            .position(|x| x.module == module && x.name == name)
        {
            let x = &self.out.imports[i];
            // B.4 coalesces wire signatures. The calling declaration keeps
            // its own nominal result decoder in HostCall or HostReply.
            if names(&x.abi.params) == names(&abi.params)
                && x.abi.result.name() == abi.result.name()
                && names(&x.abi.errors) == names(&abi.errors)
                && x.kind == kind
            {
                return Ok(i as ImportId);
            }
            let first = self.import_sites[i];
            let (m, span) = if self.is_library(site.0) && !self.is_library(first.0) {
                first
            } else {
                site
            };
            return Err((
                m,
                span,
                format!(
                    "the import `{module}` `{name}` is declared two times, with different types or kinds (B.4)"
                ),
            ));
        }
        self.out.imports.push(Import {
            module: module.to_string(),
            name: name.to_string(),
            abi,
            kind,
        });
        self.import_sites.push(site);
        Ok(self.out.imports.len() as ImportId - 1)
    }

    /// The function that calls an import: its parameters go to the host
    /// encoded as in B.2, and its value or error comes back. An async
    /// import waits for the host at the root boundary (B.4, B.5).
    fn import_func(&mut self, import: ImportId, result: HostType) -> FuncId {
        let result = abi_type(result);
        if let Some(&f) = self.import_funcs.get(&(import, result)) {
            return f;
        }
        let f = self.make_import_func(import, result);
        self.import_funcs.insert((import, result), f);
        f
    }

    /// A new function that calls an import, as `import_func`.
    fn make_import_func(&mut self, import: ImportId, result: AbiType) -> FuncId {
        let x = self.out.imports[import as usize].clone();
        let n = x.abi.params.len() as u32;
        let args: Vec<Atom> = (0..n).map(Atom::Local).collect();
        let mut locals = vec![LocalType::Value; n as usize];
        let mut body = Vec::new();
        match x.kind {
            ImportKind::External | ImportKind::Service | ImportKind::WasiStdout => {
                locals.push(LocalType::Value);
                body.push(Stmt::Let {
                    dst: n,
                    value: Value::HostCall {
                        import,
                        args,
                        result,
                    },
                    site: None,
                });
                body.push(Stmt::Return(Atom::Local(n)));
            }
            ImportKind::AsyncService => {
                let wait = self.op_func(
                    Op::Runtime {
                        name: "host_wait",
                        arity: 1,
                    },
                    1,
                );
                locals.extend([LocalType::Value; 4]);
                body.push(Stmt::Let {
                    dst: n,
                    value: Value::HostStart { import, args },
                    site: None,
                });
                body.push(Stmt::Let {
                    dst: n + 1,
                    value: Value::Op(
                        Op::Runtime {
                            name: "host_root_boundary",
                            arity: 0,
                        },
                        vec![],
                    ),
                    site: None,
                });
                body.push(Stmt::Let {
                    dst: n + 2,
                    value: Value::Op(Op::Suspend, vec![Atom::Local(n + 1), Atom::Func(wait)]),
                    site: None,
                });
                body.push(Stmt::Let {
                    dst: n + 3,
                    value: Value::HostReply(ImportReply { import, result }),
                    site: None,
                });
                body.push(Stmt::Return(Atom::Local(n + 3)));
            }
        }
        let f = self.new_func();
        self.out.funcs[f.0 as usize] = FuncKind::Code(Func {
            name: format!("import {} {}", x.module, x.name),
            arity: n,
            budget_frame: n + 1,
            locals,
            body,
            frame: true,
        });
        f
    }

    fn def_callee(&mut self, d: DefId) -> Callee {
        if self.known.contains_key(&d) {
            Callee::Direct(self.def_func(d))
        } else {
            Callee::Value(Atom::Global(self.global(d)))
        }
    }

    // ---- Functions ----

    fn new_func(&mut self) -> FuncId {
        self.out.funcs.push(FuncKind::Runtime {
            op: Op::Panic,
            arity: 0,
        });
        FuncId(self.out.funcs.len() as u32 - 1)
    }

    fn def_func(&mut self, d: DefId) -> FuncId {
        if let Some(&f) = self.def_funcs.get(&d) {
            return f;
        }
        let f = self.new_func();
        self.def_funcs.insert(d, f);
        self.work.push((f, d));
        f
    }

    fn ctor_func(&mut self, c: CtorRef) -> FuncId {
        if let Some(&f) = self.ctor_funcs.get(&c) {
            return f;
        }
        let info = &self.ctors[&c];
        let CtorRepr::Record { ctor, ty } = info.repr else {
            unreachable!("only records have fields")
        };
        let arity = info.labels.len() as u32;
        let f = self.new_func();
        self.out.funcs[f.0 as usize] = FuncKind::Ctor { ctor, ty, arity };
        self.ctor_funcs.insert(c, f);
        self.constants.insert(f);
        f
    }

    fn op_func(&mut self, op: Op, arity: u32) -> FuncId {
        let key = format!("{op:?}");
        if let Some(&f) = self.op_funcs.get(&key) {
            return f;
        }
        let f = self.new_func();
        self.out.funcs[f.0 as usize] = FuncKind::Runtime { op, arity };
        self.op_funcs.insert(key, f);
        self.constants.insert(f);
        f
    }

    /// A function expression with its captures, as a function of the
    /// intermediate form.
    fn function(
        &mut self,
        module: ModuleId,
        f: &Function,
        captures: &[VarId],
        name: String,
    ) -> Func {
        let mut fx = Fx::new(module, self.is_library(module));
        let mut out = Vec::new();
        let arity = f
            .params
            .iter()
            .filter(|p| matches!(p, Param::Pattern { .. }))
            .count() as u32;
        for _ in 0..arity {
            fx.local(LocalType::Value);
        }
        for (i, &v) in captures.iter().enumerate() {
            let l = fx.var(v);
            out.push(Stmt::Let {
                dst: l,
                value: Value::Capture(i as u32),
                site: None,
            });
        }
        // Environment parameters are not parameters of the type (§6.6):
        // the parameters of the code are the patterns, in order.
        let mut next = 0;
        for p in &f.params {
            match p {
                Param::Pattern { pattern, .. } => {
                    let v = Atom::Local(next);
                    next += 1;
                    self.bind_checked(&mut fx, &mut out, pattern, v, pattern.span.start);
                }
                Param::Env { var, name, .. } => {
                    let service = self.service(module, name.span);
                    let l = fx.var(*var);
                    out.push(Stmt::Let {
                        dst: l,
                        value: Value::Service(Env::Param, service),
                        site: None,
                    });
                }
            }
        }
        self.block(&mut fx, &mut out, &f.body, true);
        let pending = f
            .params
            .iter()
            .filter_map(|p| match p {
                Param::Pattern { pattern, .. } => Some(pattern_values(pattern)),
                Param::Env { .. } => None,
            })
            .fold(block_values(&f.body), usize::max);
        Func {
            name,
            arity,
            budget_frame: u32::try_from(1 + arity as usize + fx.vars.len() + pending)
                .expect("source frame exceeds u32"),
            locals: fx.locals,
            body: out,
            frame: true,
        }
    }

    // ---- Sites ----

    /// The site of the expression that starts at `offset` (§11.7). Library
    /// code is not program source, so it has none.
    fn site(&mut self, fx: &Fx, offset: u32) -> Option<Site> {
        if fx.library {
            return None;
        }
        let program = self.program;
        let graph = self.graph;
        let module = program.module(fx.module);
        let index = match self.module_index.get(&fx.module) {
            Some(&i) => i,
            None => {
                let i = self.out.site_modules.len() as u32;
                // A package with a location puts `location@major ` first
                // (§11.7).
                self.out.site_modules.push(format!(
                    "{}{}",
                    graph.packages[module.package].position_prefix(),
                    module.path
                ));
                self.module_index.insert(fx.module, i);
                i
            }
        };
        let lines = self
            .lines
            .entry(fx.module)
            .or_insert_with(|| LineIndex::new(&graph.sources.get(module.source).text));
        let (line, column) = lines.line_col(offset);
        let pos = SitePos {
            module: index,
            line,
            column,
        };
        Some(*self.site_map.entry(pos).or_insert_with(|| {
            self.out.sites.push(pos);
            self.out.sites.len() as Site - 1
        }))
    }

    // ---- Names ----

    fn runtime_external(&self, d: DefId) -> bool {
        self.is_library(d.module)
            && matches!(&self.program.def(d).kind, DefKind::External { module, .. } if module.starts_with("ilang:"))
    }

    /// The operation of an external function of the library.
    fn external_op(&self, d: DefId) -> Result<(Op, u32), String> {
        let DefKind::External {
            module,
            function,
            ty,
            ..
        } = &self.program.def(d).kind
        else {
            unreachable!()
        };
        let arity = match &ty.kind {
            TypeKind::Fn { params, .. } => params.len() as u32,
            _ => 0,
        };
        match module.as_str() {
            "ilang:prelude" if function == "panic" => Ok((Op::Panic, arity)),
            "ilang:core" => match CORE.iter().position(|n| n == function) {
                Some(i) => Ok((
                    Op::Runtime {
                        name: CORE_FUNCS[i],
                        arity,
                    },
                    arity,
                )),
                None => Err(format!("`core.{function}` is not known")),
            },
            "ilang:fail" if function == "raise" => Ok((Op::Raise, arity)),
            "ilang:fail" if function == "catch" => Ok((Op::Catch, arity)),
            "ilang:resource" if function == "ensure" => Ok((Op::Ensure, arity)),
            "ilang:control" => match function.as_str() {
                "boundary" => Ok((Op::Boundary, arity)),
                "suspend" => Ok((Op::Suspend, arity)),
                "resume" => Ok((Op::Resume, arity)),
                "cancel" => Ok((Op::Cancel, arity)),
                _ => Err(format!("`control.{function}` is not known")),
            },
            _ => Err(format!(
                "the library has no operation `{module}` `{function}`"
            )),
        }
    }

    fn name(&mut self, fx: &mut Fx, e: &Expr, r: ValueRef) -> Atom {
        match r {
            ValueRef::Local(v) => match fx.vars.get(&v) {
                Some(&l) => Atom::Local(l),
                None => {
                    // A name of a direct binding, used in a function body
                    // before it is bound: only closures may do this, and
                    // they are patched.
                    Atom::Local(fx.var(v))
                }
            },
            ValueRef::Def(d) => match self.program.def(d).kind {
                DefKind::External { .. } if !self.runtime_external(d) => {
                    match self.external_func(d) {
                        Some(f) => {
                            self.constants.insert(f);
                            Atom::Func(f)
                        }
                        None => Atom::Nil,
                    }
                }
                DefKind::External { .. } => match self.external_op(d) {
                    Ok((op, arity)) => Atom::Func(self.op_func(op, arity)),
                    Err(message) => {
                        self.error(fx.module, e.span, message);
                        Atom::Nil
                    }
                },
                DefKind::Binding { .. } => {
                    if self.known.contains_key(&d) {
                        let f = self.def_func(d);
                        self.constants.insert(f);
                        Atom::Func(f)
                    } else {
                        Atom::Global(self.global(d))
                    }
                }
            },
            ValueRef::Ctor(c) => {
                let info = &self.ctors[&c];
                match info.repr {
                    CtorRepr::Bool(b) => Atom::Bool(b),
                    CtorRepr::Nil => Atom::Nil,
                    CtorRepr::Record { ctor, ty } if info.labels.is_empty() => {
                        Atom::Ctor { ctor, ty }
                    }
                    CtorRepr::Record { .. } => Atom::Func(self.ctor_func(c)),
                }
            }
        }
    }

    // ---- Expressions ----

    fn expr(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, e: &Expr) -> Atom {
        self.lower(fx, out, e, false)
            .expect("a value that is not in tail position")
    }

    /// Lowers `e`. In tail position, the statements end the function and
    /// there is no value.
    fn lower(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, e: &Expr, tail: bool) -> Option<Atom> {
        let a = match &e.kind {
            ExprKind::Int(n) => Atom::Int(BigInt::from(n.clone())),
            ExprKind::Float(x) => Atom::Float(*x),
            ExprKind::String(s) => Atom::String(s.clone()),
            ExprKind::Name(r) => self.name(fx, e, *r),
            ExprKind::Tuple(items) => {
                let parts = items.iter().map(|x| self.expr(fx, out, x)).collect();
                let_value(fx, out, Value::Tuple(parts), None)
            }
            ExprKind::List { items, rest } => {
                let parts: Vec<Atom> = items.iter().map(|x| self.expr(fx, out, x)).collect();
                let mut list = match rest {
                    Some(r) => self.expr(fx, out, r),
                    None => Atom::EmptyList,
                };
                for p in parts.into_iter().rev() {
                    list = let_value(fx, out, Value::Cons(p, list), None);
                }
                list
            }
            ExprKind::BitArray(segs) => self.bit_array(fx, out, e, segs),
            ExprKind::Block(b) => return self.block(fx, out, b, tail),
            ExprKind::Fn(f) => self.closure(fx, out, f),
            ExprKind::Case { subject, clauses } => {
                let s = self.expr(fx, out, subject);
                return self.case(fx, out, s, clauses, tail);
            }
            ExprKind::Call { callee, args } => {
                let args: Vec<&Expr> = args.iter().map(|a| &a.value).collect();
                return self.call(fx, out, e.span, callee, None, &args, tail);
            }
            ExprKind::Field { target, .. } => {
                let t = self.expr(fx, out, target);
                match self.typed.access.get(&(fx.module, e.span)) {
                    Some(Access::Field(list)) => self.field(fx, out, t, list),
                    Some(Access::Part(k)) => let_value(fx, out, Value::Field(t, *k), None),
                    None => unreachable!("a checked field has an access"),
                }
            }
            ExprKind::TupleIndex { target, index, .. } => {
                let t = self.expr(fx, out, target);
                let k = match self.typed.access.get(&(fx.module, e.span)) {
                    Some(Access::Part(k)) => *k,
                    _ => u32::try_from(index).unwrap_or(0),
                };
                let_value(fx, out, Value::Field(t, k), None)
            }
            ExprKind::RecordUpdate {
                ctor, base, fields, ..
            } => {
                let b = self.expr(fx, out, base);
                let given: Vec<(String, Atom)> = fields
                    .iter()
                    .map(|(name, x)| (name.name.clone(), self.expr(fx, out, x)))
                    .collect();
                let info = self.ctors[ctor].clone();
                let CtorRepr::Record { ctor, ty } = info.repr else {
                    unreachable!("only records have fields")
                };
                let mut parts = Vec::new();
                for (i, label) in info.labels.iter().enumerate() {
                    let found = given
                        .iter()
                        .find(|(n, _)| Some(n) == label.as_ref())
                        .map(|(_, a)| a.clone());
                    parts.push(match found {
                        Some(a) => a,
                        None => let_value(fx, out, Value::Field(b.clone(), i as u32), None),
                    });
                }
                let_value(
                    fx,
                    out,
                    Value::Record {
                        ctor,
                        ty,
                        fields: parts,
                    },
                    None,
                )
            }
            ExprKind::Unary { op, expr } => match (op, &expr.kind) {
                (UnOp::Neg, ExprKind::Int(n)) => Atom::Int(-BigInt::from(n.clone())),
                (UnOp::NegFloat, ExprKind::Float(x)) => Atom::Float(-*x),
                (UnOp::Neg, _) => {
                    let a = self.expr(fx, out, expr);
                    let_value(fx, out, Value::Op(Op::IntNeg, vec![a]), None)
                }
                (UnOp::NegFloat, _) => {
                    let a = self.expr(fx, out, expr);
                    let_value(fx, out, Value::Op(Op::FloatNeg, vec![a]), None)
                }
            },
            ExprKind::Binary { op, lhs, rhs } => {
                return self.binary(fx, out, e, *op, lhs, rhs, tail);
            }
            ExprKind::Error => {
                out.push(Stmt::Unreachable);
                Atom::Nil
            }
        };
        finish(out, tail, a)
    }

    /// A block (§6.10). An environment statement changes the environment
    /// for the rest of its block only (§9.2).
    fn block(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, b: &Block, tail: bool) -> Option<Atom> {
        let outer = fx.env;
        let a = self.statements(fx, out, b, tail);
        fx.env = outer;
        a
    }

    fn statements(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        b: &Block,
        tail: bool,
    ) -> Option<Atom> {
        let last = b.stmts.len().saturating_sub(1);
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                hir::Stmt::Expr(e) if i == last => return self.lower(fx, out, e, tail),
                hir::Stmt::Expr(e) => {
                    self.expr(fx, out, e);
                }
                hir::Stmt::Bind(binding) => self.binding(fx, out, binding),
                hir::Stmt::Env { value, span, .. } => {
                    // The value uses the environment from outside the
                    // statement (§4.8 rule 9).
                    let v = self.expr(fx, out, value);
                    let service = self.service(fx.module, *span);
                    let dst = fx.local(LocalType::Env);
                    out.push(Stmt::LetEnv {
                        dst,
                        env: fx.env,
                        service,
                        value: v,
                    });
                    fx.env = Env::Local(dst);
                }
            }
        }
        // A block always ends with an expression (§6.10).
        finish(out, tail, Atom::Nil)
    }

    /// A binding in a block (§6.10). The names of a direct value may be used
    /// in the function bodies of the value: those closures are made first and
    /// their captures are filled when the names are bound.
    fn binding(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, b: &Binding) {
        if !b.direct {
            let v = self.expr(fx, out, &b.value);
            self.bind_checked(fx, out, &b.pattern, v, b.span.start);
            return;
        }
        let mut vars = HashSet::new();
        pattern_vars(&b.pattern, &mut vars);
        let saved = fx.pending.replace(vars);
        let saved_patches = std::mem::take(&mut fx.patches);
        let v = self.expr(fx, out, &b.value);
        fx.pending = saved;
        let patches = std::mem::replace(&mut fx.patches, saved_patches);
        self.bind_checked(fx, out, &b.pattern, v, b.span.start);
        for (closure, index, var) in patches {
            let value = Atom::Local(fx.var(var));
            out.push(Stmt::SetCapture {
                closure,
                index,
                value,
            });
        }
    }

    fn closure(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, f: &Function) -> Atom {
        let captures = captures_of(f);
        let id = self.new_func();
        let name = format!("{}:fn{}", self.program.module(fx.module).path, id.0);
        let func = self.function(fx.module, f, &captures, name);
        self.out.funcs[id.0 as usize] = FuncKind::Code(func);
        let mut atoms = Vec::new();
        let mut later = Vec::new();
        for (i, v) in captures.iter().enumerate() {
            if fx.pending.as_ref().is_some_and(|p| p.contains(v)) {
                atoms.push(Atom::Nil);
                later.push((i as u32, *v));
            } else {
                atoms.push(Atom::Local(fx.var(*v)));
            }
        }
        let dst = fx.local(LocalType::Value);
        out.push(Stmt::Let {
            dst,
            value: Value::Closure {
                func: id,
                captures: atoms,
            },
            site: None,
        });
        for (i, v) in later {
            fx.patches.push((dst, i, v));
        }
        Atom::Local(dst)
    }

    /// `v.f` (§6.7): the constructor of the value picks the field.
    fn field(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        t: Atom,
        list: &[(CtorRef, u32)],
    ) -> Atom {
        let first = list[0].1;
        if list.iter().all(|(_, i)| *i == first) {
            return let_value(fx, out, Value::Field(t, first), None);
        }
        let dst = fx.local(LocalType::Value);
        let end = fx.label();
        let mut body = Vec::new();
        for (k, (c, i)) in list.iter().enumerate() {
            let read = Stmt::Let {
                dst,
                value: Value::Field(t.clone(), *i),
                site: None,
            };
            if k + 1 == list.len() {
                body.push(read);
            } else {
                let CtorRepr::Record { ctor, .. } = self.ctors[c].repr else {
                    unreachable!("only records have fields")
                };
                body.push(Stmt::If {
                    cond: Cond::Ctor(t.clone(), ctor),
                    then: vec![read, Stmt::Break(end)],
                    otherwise: vec![],
                });
            }
        }
        out.push(Stmt::Block { label: end, body });
        Atom::Local(dst)
    }

    #[allow(clippy::too_many_arguments)]
    fn binary(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        e: &Expr,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        tail: bool,
    ) -> Option<Atom> {
        if op == BinOp::Pipe {
            // `a |> f` is `f(a)`, and `a |> f(b)` is `f(a, b)` (§6.9).
            return match &rhs.kind {
                ExprKind::Call { callee, args } => {
                    let args: Vec<&Expr> = args.iter().map(|a| &a.value).collect();
                    self.call(fx, out, e.span, callee, Some(lhs), &args, tail)
                }
                _ => self.call(fx, out, e.span, rhs, Some(lhs), &[], tail),
            };
        }
        if matches!(op, BinOp::And | BinOp::Or) && tail {
            // The right side is in tail position (§11.6).
            let a = self.expr(fx, out, lhs);
            let mut right = Vec::new();
            self.lower(fx, &mut right, rhs, true);
            let short = vec![Stmt::Return(Atom::Bool(op == BinOp::Or))];
            let (then, otherwise) = if op == BinOp::And {
                (right, short)
            } else {
                (short, right)
            };
            out.push(Stmt::If {
                cond: Cond::True(a),
                then,
                otherwise,
            });
            return None;
        }
        if matches!(op, BinOp::And | BinOp::Or) {
            // The right side is only worked out when the left is not enough.
            let a = self.expr(fx, out, lhs);
            let dst = fx.local(LocalType::Value);
            let mut right = Vec::new();
            let b = self.expr(fx, &mut right, rhs);
            right.push(Stmt::Let {
                dst,
                value: Value::Atom(b),
                site: None,
            });
            let short = vec![Stmt::Let {
                dst,
                value: Value::Atom(Atom::Bool(op == BinOp::Or)),
                site: None,
            }];
            let (then, otherwise) = if op == BinOp::And {
                (right, short)
            } else {
                (short, right)
            };
            out.push(Stmt::If {
                cond: Cond::True(a),
                then,
                otherwise,
            });
            return finish(out, tail, Atom::Local(dst));
        }
        let a = self.expr(fx, out, lhs);
        let b = self.expr(fx, out, rhs);
        let o = match op {
            BinOp::Eq => Op::Equal,
            BinOp::NotEq => Op::NotEqual,
            BinOp::Lt => Op::IntLt,
            BinOp::LtEq => Op::IntLe,
            BinOp::Gt => Op::IntGt,
            BinOp::GtEq => Op::IntGe,
            BinOp::LtFloat => Op::FloatLt,
            BinOp::LtEqFloat => Op::FloatLe,
            BinOp::GtFloat => Op::FloatGt,
            BinOp::GtEqFloat => Op::FloatGe,
            BinOp::Concat => Op::StringAppend,
            BinOp::Add => Op::IntAdd,
            BinOp::Sub => Op::IntSub,
            BinOp::Mul => Op::IntMul,
            BinOp::Div => Op::IntDiv,
            BinOp::Rem => Op::IntRem,
            BinOp::AddFloat => Op::FloatAdd,
            BinOp::SubFloat => Op::FloatSub,
            BinOp::MulFloat => Op::FloatMul,
            BinOp::DivFloat => Op::FloatDiv,
            BinOp::And | BinOp::Or | BinOp::Pipe => unreachable!(),
        };
        let site = if o.may_stop() {
            self.site(fx, e.span.start)
        } else {
            None
        };
        let v = let_value(fx, out, Value::Op(o, vec![a, b]), site);
        finish(out, tail, v)
    }

    /// What a callee is, worked out when it is a value.
    fn target(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, callee: &Expr) -> Target {
        match &callee.kind {
            ExprKind::Name(ValueRef::Def(d)) => match self.program.def(*d).kind {
                // §10.1: a call of an external function is done when it is
                // reached, like all calls.
                DefKind::External { .. } if !self.runtime_external(*d) => {
                    match self.external_func(*d) {
                        Some(f) => Target::Callee(Callee::Direct(f)),
                        None => Target::Op(Op::Panic),
                    }
                }
                DefKind::External { .. } => match self.external_op(*d) {
                    Ok((op, _)) => Target::Op(op),
                    Err(message) => {
                        self.error(fx.module, callee.span, message);
                        Target::Op(Op::Panic)
                    }
                },
                DefKind::Binding { .. } if self.known.contains_key(d) => {
                    Target::Callee(Callee::Direct(self.def_func(*d)))
                }
                DefKind::Binding { .. } => {
                    Target::Callee(Callee::Value(Atom::Global(self.global(*d))))
                }
            },
            ExprKind::Name(ValueRef::Ctor(c)) => Target::Ctor(*c),
            _ => Target::Callee(Callee::Value(self.expr(fx, out, callee))),
        }
    }

    /// A call (§6.6), or `a |> f(…)` with `a` as `first`. The function is
    /// worked out first, then the arguments as they are written (§6.2); for
    /// `|>`, `a` comes before both.
    #[allow(clippy::too_many_arguments)]
    fn call(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        span: Span,
        callee: &Expr,
        first: Option<&Expr>,
        args: &[&Expr],
        tail: bool,
    ) -> Option<Atom> {
        let mut written = Vec::new();
        if let Some(a) = first {
            written.push(self.expr(fx, out, a));
        }
        let target = self.target(fx, out, callee);
        for a in args {
            written.push(self.expr(fx, out, a));
        }
        let args: Vec<Atom> = match self.typed.args.get(&(fx.module, span)) {
            Some(order) if order.len() == written.len() => {
                order.iter().map(|&i| written[i as usize].clone()).collect()
            }
            _ => written,
        };
        match target {
            Target::Ctor(c) => {
                let info = &self.ctors[&c];
                let a = match info.repr {
                    CtorRepr::Bool(b) => Atom::Bool(b),
                    CtorRepr::Nil => Atom::Nil,
                    CtorRepr::Record { ctor, ty } if args.is_empty() => Atom::Ctor { ctor, ty },
                    CtorRepr::Record { ctor, ty } => let_value(
                        fx,
                        out,
                        Value::Record {
                            ctor,
                            ty,
                            fields: args,
                        },
                        None,
                    ),
                };
                finish(out, tail, a)
            }
            Target::Op(op) => {
                // Operations of the runtime are not Wasm tail calls: those
                // that call i code take errors there, or run final
                // functions. But a call of a library function in tail
                // position counts as a tail call (§11.7): its position is
                // a tail site.
                let site = self
                    .site(fx, span.start)
                    .map(|s| if tail { s | TAIL_SITE } else { s });
                let value = if op.needs_env() {
                    Value::OpEnv(op, args, fx.env)
                } else {
                    Value::Op(op, args)
                };
                let v = let_value(fx, out, value, site);
                finish(out, tail, v)
            }
            Target::Callee(callee) => {
                let env = fx.env;
                if tail {
                    // §11.7: the position of a call in tail position is
                    // given when the stop comes before the callee works out
                    // an expression of the program source.
                    let site = self.site(fx, span.start).map(|s| s | TAIL_SITE);
                    out.push(Stmt::TailCall {
                        callee,
                        args,
                        env,
                        site,
                    });
                    None
                } else {
                    let site = self.site(fx, span.start);
                    Some(let_value(fx, out, Value::Call { callee, args, env }, site))
                }
            }
        }
    }

    // ---- Bit arrays (§8.3) ----

    fn bit_array(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        e: &Expr,
        segs: &[Segment<Expr>],
    ) -> Atom {
        let mut acc: Option<Atom> = None;
        for s in segs {
            let v = self.expr(fx, out, &s.value);
            let fmt = format(s.format.as_ref());
            let piece = match fmt {
                Some(Fmt::Int {
                    width: Width::Const(n),
                    little,
                    ..
                }) => let_value(
                    fx,
                    out,
                    Value::BitsFromInt {
                        value: v,
                        width: Num::Const(n as i64),
                        little,
                    },
                    None,
                ),
                Some(Fmt::Int {
                    width: Width::Dyn(x),
                    little,
                    ..
                }) => {
                    let n = self.expr(fx, out, x);
                    let site = self.site(fx, e.span.start);
                    let w = let_num(fx, out, NumValue::LengthOf(n, 1), site);
                    let_value(
                        fx,
                        out,
                        Value::BitsFromInt {
                            value: v,
                            width: w,
                            little,
                        },
                        None,
                    )
                }
                Some(Fmt::Float { width, little }) => {
                    let site = self.site(fx, e.span.start);
                    let_value(
                        fx,
                        out,
                        Value::BitsFromFloat {
                            value: v,
                            width,
                            little,
                        },
                        site,
                    )
                }
                Some(Fmt::Bits(None)) => v,
                Some(Fmt::Bits(Some(x))) | Some(Fmt::Bytes(Some(x))) => {
                    let unit = if matches!(fmt, Some(Fmt::Bits(_))) {
                        1
                    } else {
                        8
                    };
                    let n = self.expr(fx, out, x);
                    let site = self.site(fx, e.span.start);
                    let w = let_num(fx, out, NumValue::TakeLength(n, unit), site);
                    let_value(fx, out, Value::BitsTake(v, w), site)
                }
                Some(Fmt::Bytes(None)) => {
                    let site = self.site(fx, e.span.start);
                    let_value(fx, out, Value::BitsWhole(v), site)
                }
                Some(Fmt::Utf {
                    encoding,
                    little,
                    codepoint,
                }) => let_value(
                    fx,
                    out,
                    Value::BitsFromUtf {
                        value: v,
                        encoding,
                        little,
                        codepoint,
                    },
                    None,
                ),
                None => {
                    out.push(Stmt::Unreachable);
                    v
                }
            };
            acc = Some(match acc {
                None => piece,
                Some(a) => let_value(fx, out, Value::BitsConcat(a, piece), None),
            });
        }
        match acc {
            // A single `bits` segment is a new value only in name: bit
            // arrays cannot be changed.
            Some(a) => a,
            None => let_value(fx, out, Value::BitsEmpty, None),
        }
    }

    // ---- case (§6.12) ----

    fn case(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        s: Atom,
        clauses: &[Clause],
        tail: bool,
    ) -> Option<Atom> {
        let end = fx.label();
        let dst = if tail {
            None
        } else {
            Some(fx.local(LocalType::Value))
        };
        let mut body = Vec::new();
        for c in clauses {
            let next = fx.label();
            let mut cb = Vec::new();
            if let [p] = c.patterns.as_slice() {
                self.pattern(fx, &mut cb, p, s.clone(), next);
                if let Some(g) = &c.guard {
                    let g = self.expr(fx, &mut cb, g);
                    guard(&mut cb, Cond::True(g), next);
                }
            } else {
                // The choices are tried in order, each with the guard, as
                // if each were a clause of its own. The body is shared.
                let matched = fx.label();
                let mut mb = Vec::new();
                for p in &c.patterns {
                    let no = fx.label();
                    let mut pb = Vec::new();
                    self.pattern(fx, &mut pb, p, s.clone(), no);
                    if let Some(g) = &c.guard {
                        let g = self.expr(fx, &mut pb, g);
                        guard(&mut pb, Cond::True(g), no);
                    }
                    pb.push(Stmt::Break(matched));
                    mb.push(Stmt::Block {
                        label: no,
                        body: pb,
                    });
                }
                mb.push(Stmt::Break(next));
                cb.push(Stmt::Block {
                    label: matched,
                    body: mb,
                });
            }
            if let Some(a) = self.lower(fx, &mut cb, &c.body, tail) {
                cb.push(Stmt::Let {
                    dst: dst.expect("a value"),
                    value: Value::Atom(a),
                    site: None,
                });
                cb.push(Stmt::Break(end));
            }
            body.push(Stmt::Block {
                label: next,
                body: cb,
            });
        }
        if let [c] = clauses
            && c.guard.is_none()
        {
            let span = c
                .patterns
                .first()
                .unwrap()
                .span
                .to(c.patterns.last().unwrap().span);
            if self
                .typed
                .may_fail
                .get(&(fx.module, span))
                .copied()
                .unwrap_or(false)
            {
                self.assert_failure(fx, &mut body, span, s, span.start);
            }
        }
        // The clauses cover every value (§7.4).
        body.push(Stmt::Unreachable);
        out.push(Stmt::Block { label: end, body });
        dst.map(Atom::Local)
    }

    // ---- Patterns (§7) ----

    fn bind_var(fx: &mut Fx, out: &mut Vec<Stmt>, b: &Binder, v: Atom) {
        let l = fx.var(b.var);
        out.push(Stmt::Let {
            dst: l,
            value: Value::Atom(v),
            site: None,
        });
    }

    /// Matches `v` with the pattern of a `case` clause, and leaves to `fail`
    /// when it does not match (§7.2).
    fn pattern(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, p: &Pattern, v: Atom, fail: Label) {
        match &p.kind {
            PatternKind::Quote {
                template,
                functions,
                captures,
                ..
            } => {
                let template = self.pattern_call(
                    fx,
                    out,
                    functions[0],
                    p.span,
                    vec![Atom::Int((*template).into())],
                );
                let result = self.pattern_call(fx, out, functions[1], p.span, vec![template, v]);
                let success =
                    self.pattern_call(fx, out, functions[2], p.span, vec![result.clone()]);
                guard(out, Cond::True(success), fail);
                for (index, (many, pattern)) in captures.iter().enumerate() {
                    let value = self.pattern_call(
                        fx,
                        out,
                        functions[if *many { 4 } else { 3 }],
                        p.span,
                        vec![result.clone(), Atom::Int(index.into())],
                    );
                    self.pattern(fx, out, pattern, value, fail);
                }
            }
            PatternKind::Var(b) => Self::bind_var(fx, out, b, v),
            PatternKind::Discard => {}
            PatternKind::Typed { binder, .. } => {
                if let Some(heads) = self.typed.type_tests.get(&(fx.module, p.span)) {
                    let cond = heads
                        .iter()
                        .map(|m| self.test_cond(v.clone(), m))
                        .reduce(|a, b| Cond::Or(Box::new(a), Box::new(b)))
                        .unwrap_or(Cond::Tag(v.clone(), u32::MAX));
                    guard(out, cond, fail);
                }
                if let Some(b) = binder {
                    Self::bind_var(fx, out, b, v);
                }
            }
            PatternKind::Named { binder, pattern } => {
                Self::bind_var(fx, out, binder, v.clone());
                self.pattern(fx, out, pattern, v, fail);
            }
            PatternKind::Int { negative, value } => {
                let mut n = BigInt::from(value.clone());
                if *negative {
                    n = -n;
                }
                guard(out, Cond::IsInt(v, n), fail);
            }
            PatternKind::Float { negative, value } => {
                let x = if *negative { -*value } else { *value };
                guard(out, Cond::IsFloat(v, x), fail);
            }
            PatternKind::String(s) => guard(out, Cond::IsString(v, s.clone()), fail),
            PatternKind::Prefix { prefix, rest } => {
                guard(out, Cond::Tag(v.clone(), 3), fail);
                guard(
                    out,
                    Cond::StartsWith(v.clone(), Atom::String(prefix.clone())),
                    fail,
                );
                if let Some(b) = rest {
                    let r = let_value(fx, out, Value::StringSuffix(v, prefix.len() as u32), None);
                    Self::bind_var(fx, out, b, r);
                }
            }
            PatternKind::Constructor { ctor, args, .. } => {
                let info = self.ctors[ctor].clone();
                let cond = match info.repr {
                    CtorRepr::Bool(b) => Cond::Bool(v.clone(), b),
                    CtorRepr::Nil => Cond::Tag(v.clone(), 0),
                    CtorRepr::Record { ctor, .. } => Cond::Ctor(v.clone(), ctor),
                };
                guard(out, cond, fail);
                for (arg, i) in self.fields_of(fx, p, args) {
                    let f = let_value(fx, out, Value::Field(v.clone(), i), None);
                    self.pattern(fx, out, arg, f, fail);
                }
            }
            PatternKind::Tuple(parts) => {
                guard(out, Cond::Tuple(v.clone(), parts.len() as u32), fail);
                for (i, part) in parts.iter().enumerate() {
                    let f = let_value(fx, out, Value::Field(v.clone(), i as u32), None);
                    self.pattern(fx, out, part, f, fail);
                }
            }
            PatternKind::List { items, rest } => {
                guard(out, Cond::Tag(v.clone(), 6), fail);
                let mut cur = v;
                for item in items {
                    guard(out, Cond::Not(Box::new(Cond::Empty(cur.clone()))), fail);
                    let h = let_value(fx, out, Value::ListHead(cur.clone()), None);
                    self.pattern(fx, out, item, h, fail);
                    cur = let_value(fx, out, Value::ListTail(cur), None);
                }
                match rest {
                    None => guard(out, Cond::Empty(cur), fail),
                    Some(Some(b)) => Self::bind_var(fx, out, b, cur),
                    Some(None) => {}
                }
            }
            PatternKind::BitArray(segs) => self.bits_pattern(fx, out, v, segs, fail),
            PatternKind::Error(_) => out.push(Stmt::Unreachable),
        }
    }

    fn pattern_call(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        function: DefId,
        span: Span,
        args: Vec<Atom>,
    ) -> Atom {
        let callee = Expr {
            kind: ExprKind::Name(ValueRef::Def(function)),
            span,
        };
        let target = self.target(fx, out, &callee);
        let value = match target {
            Target::Callee(callee) => Value::Call {
                callee,
                args,
                env: fx.env,
            },
            Target::Op(op) if op.needs_env() => Value::OpEnv(op, args, fx.env),
            Target::Op(op) => Value::Op(op, args),
            Target::Ctor(_) => unreachable!("quote service is a function"),
        };
        let_value(fx, out, value, self.site(fx, span.start))
    }

    /// The test of a type test `x: T` for one member of `T` (§7.1).
    fn test_cond(&self, v: Atom, member: &TestMember) -> Cond {
        let values = match &member.values {
            TestValues::All => return self.head_cond(v, member.head),
            TestValues::Only(values) | TestValues::Except(values) => values,
        };
        let equal = values
            .iter()
            .map(|value| match value {
                LitValue::Int(n) => Cond::IsInt(v.clone(), n.clone()),
                LitValue::Str(s) => Cond::IsString(v.clone(), s.clone()),
            })
            .reduce(|a, b| Cond::Or(Box::new(a), Box::new(b)))
            .unwrap_or(Cond::Tag(v.clone(), u32::MAX));
        match &member.values {
            TestValues::Only(_) => equal,
            TestValues::Except(_) => Cond::And(
                Box::new(self.head_cond(v, member.head)),
                Box::new(Cond::Not(Box::new(equal))),
            ),
            TestValues::All => unreachable!(),
        }
    }

    fn head_cond(&self, v: Atom, h: Head) -> Cond {
        match h {
            Head::Tuple(n) => Cond::Tuple(v, n),
            Head::Fn => Cond::Tag(v, 7),
            Head::Named(TypeRef::Builtin(b)) => Cond::Tag(
                v,
                match b {
                    BuiltinType::Int => 8,
                    BuiltinType::Float => 2,
                    BuiltinType::String => 3,
                    BuiltinType::List => 6,
                    BuiltinType::BitArray => 9,
                    BuiltinType::UtfCodepoint => 13,
                    BuiltinType::Boundary => 11,
                    BuiltinType::Continuation => 12,
                    BuiltinType::Never => u32::MAX,
                },
            ),
            Head::Named(TypeRef::Custom(t)) => match self.types.get(&t) {
                Some(TypeRepr::Bool) => Cond::Tag(v, 1),
                Some(TypeRepr::Nil) => Cond::Tag(v, 0),
                Some(TypeRepr::Record(id)) => Cond::Type(v, *id),
                Some(TypeRepr::Host) => Cond::Tag(v, 10),
                None => match &self.program.ty(t).body {
                    TypeBody::Alias(Type {
                        kind: TypeKind::Named { ty, .. },
                        ..
                    }) => self.head_cond(v, Head::Named(*ty)),
                    _ => Cond::Tag(v, u32::MAX),
                },
            },
        }
    }

    /// §8.4: the segments are matched from the start, and all the bits must
    /// be used.
    fn bits_pattern(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        v: Atom,
        segs: &[Segment<Pattern>],
        fail: Label,
    ) {
        guard(out, Cond::Tag(v.clone(), 9), fail);
        let total = let_num(fx, out, NumValue::BitsLength(v.clone()), None);
        let mut pos = Num::Const(0);
        for s in segs {
            let fmt = format(s.format.as_ref());
            match fmt {
                Some(Fmt::Int {
                    signed,
                    width,
                    little,
                }) => {
                    let w = match width {
                        Width::Const(n) => Num::Const(n as i64),
                        Width::Dyn(x) => self.pattern_width(fx, out, x, 1, fail),
                    };
                    guard(out, Cond::BitsHas(v.clone(), pos.clone(), w.clone()), fail);
                    let x = let_value(
                        fx,
                        out,
                        Value::BitsReadInt {
                            bits: v.clone(),
                            start: pos.clone(),
                            width: w.clone(),
                            signed,
                            little,
                        },
                        None,
                    );
                    self.pattern(fx, out, &s.value, x, fail);
                    pos = let_num(fx, out, NumValue::Add(pos, w), None);
                }
                Some(Fmt::Float { width, little }) => {
                    guard(
                        out,
                        Cond::BitsFloat {
                            bits: v.clone(),
                            start: pos.clone(),
                            width,
                            little,
                        },
                        fail,
                    );
                    let x = let_value(
                        fx,
                        out,
                        Value::BitsReadFloat {
                            bits: v.clone(),
                            start: pos.clone(),
                            width,
                            little,
                        },
                        None,
                    );
                    self.pattern(fx, out, &s.value, x, fail);
                    pos = let_num(fx, out, NumValue::Add(pos, Num::Const(width as i64)), None);
                }
                Some(Fmt::Bits(None)) | Some(Fmt::Bytes(None)) => {
                    let rest = let_num(fx, out, NumValue::Sub(total.clone(), pos.clone()), None);
                    if matches!(fmt, Some(Fmt::Bytes(None))) {
                        guard(out, Cond::Whole(rest.clone()), fail);
                    }
                    let x = let_value(
                        fx,
                        out,
                        Value::BitsSlice {
                            bits: v.clone(),
                            start: pos.clone(),
                            length: rest,
                        },
                        None,
                    );
                    self.pattern(fx, out, &s.value, x, fail);
                    pos = total.clone();
                }
                Some(Fmt::Bits(Some(n))) | Some(Fmt::Bytes(Some(n))) => {
                    let unit = if matches!(fmt, Some(Fmt::Bits(_))) {
                        1
                    } else {
                        8
                    };
                    let w = self.pattern_width(fx, out, n, unit, fail);
                    guard(out, Cond::BitsHas(v.clone(), pos.clone(), w.clone()), fail);
                    let x = let_value(
                        fx,
                        out,
                        Value::BitsSlice {
                            bits: v.clone(),
                            start: pos.clone(),
                            length: w.clone(),
                        },
                        None,
                    );
                    self.pattern(fx, out, &s.value, x, fail);
                    pos = let_num(fx, out, NumValue::Add(pos, w), None);
                }
                Some(Fmt::Utf {
                    encoding,
                    little,
                    codepoint: false,
                }) => {
                    let PatternKind::String(text) = &s.value.kind else {
                        self.error(
                            fx.module,
                            s.value.span,
                            "a string format in a pattern needs a string literal (§8.4)",
                        );
                        continue;
                    };
                    let lit = let_value(
                        fx,
                        out,
                        Value::BitsFromUtf {
                            value: Atom::String(text.clone()),
                            encoding,
                            little,
                            codepoint: false,
                        },
                        None,
                    );
                    let w = let_num(fx, out, NumValue::BitsLength(lit.clone()), None);
                    guard(out, Cond::BitsHas(v.clone(), pos.clone(), w.clone()), fail);
                    let x = let_value(
                        fx,
                        out,
                        Value::BitsSlice {
                            bits: v.clone(),
                            start: pos.clone(),
                            length: w.clone(),
                        },
                        None,
                    );
                    guard(out, Cond::BitsEqual(x, lit), fail);
                    pos = let_num(fx, out, NumValue::Add(pos, w), None);
                }
                Some(Fmt::Utf {
                    encoding,
                    little,
                    codepoint: true,
                }) => {
                    let cp = fx.local(LocalType::Value);
                    let width = fx.local(LocalType::I64);
                    let ok = fx.local(LocalType::I32);
                    out.push(Stmt::ReadUtf {
                        codepoint: cp,
                        width,
                        ok,
                        bits: v.clone(),
                        start: pos.clone(),
                        encoding,
                        little,
                    });
                    guard(out, Cond::I32(ok), fail);
                    self.pattern(fx, out, &s.value, Atom::Local(cp), fail);
                    pos = let_num(fx, out, NumValue::Add(pos, Num::Local(width)), None);
                }
                None => out.push(Stmt::Unreachable),
            }
        }
        guard(out, Cond::NumEq(pos, total), fail);
    }

    /// The width of a segment of a pattern with a length; a length that is
    /// negative does not match.
    fn pattern_width(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        n: &Expr,
        unit: u32,
        fail: Label,
    ) -> Num {
        let n = self.expr(fx, out, n);
        let w = let_num(fx, out, NumValue::WidthOf(n, unit), None);
        guard(
            out,
            Cond::Not(Box::new(Cond::NumLt(w.clone(), Num::Const(0)))),
            fail,
        );
        w
    }

    /// The parts of a constructor pattern and the fields they are matched
    /// to (§7.1), as the checker found them.
    fn fields_of<'p>(&self, fx: &Fx, p: &Pattern, args: &'p [Pattern]) -> Vec<(&'p Pattern, u32)> {
        let indices = self
            .typed
            .pattern_fields
            .get(&(fx.module, p.span))
            .expect("a checked constructor pattern has its fields");
        args.iter()
            .zip(indices)
            .filter(|(_, i)| **i != u32::MAX)
            .map(|(a, i)| (a, *i))
            .collect()
    }

    /// Whether `bind` takes the pattern: it has no test at all, given the
    /// type of its value.
    fn plain(&self, p: &Pattern) -> bool {
        match &p.kind {
            PatternKind::Var(_) | PatternKind::Discard | PatternKind::Typed { .. } => true,
            PatternKind::Named { pattern, .. } => self.plain(pattern),
            PatternKind::Tuple(parts) => parts.iter().all(|x| self.plain(x)),
            PatternKind::Constructor { ctor, args, .. } => {
                matches!(&self.program.ty(ctor.ty).body, TypeBody::Custom(cs) if cs.len() == 1)
                    && args.iter().all(|x| self.plain(x))
            }
            PatternKind::List {
                items,
                rest: Some(_),
            } => items.is_empty(),
            _ => false,
        }
    }

    /// Binds the pattern of a binding or a parameter (§6.10, §6.11). When
    /// it may fail, a value that does not match is reported to `Assert`:
    /// `a.fail(AssertionFailure(m))`, with the message of §6.10 and the
    /// position `at` on the shadow stack.
    fn bind_checked(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, p: &Pattern, v: Atom, at: u32) {
        let fails = self
            .typed
            .may_fail
            .get(&(fx.module, p.span))
            .copied()
            .unwrap_or(false);
        if !fails && self.plain(p) {
            self.bind(fx, out, p, v);
            return;
        }
        let ok = fx.label();
        let no = fx.label();
        let mut matching = Vec::new();
        self.pattern(fx, &mut matching, p, v.clone(), no);
        matching.push(Stmt::Break(ok));
        let mut body = vec![Stmt::Block {
            label: no,
            body: matching,
        }];
        if fails {
            self.assert_failure(fx, &mut body, p.span, v, at);
        }
        // `fail` never gives a value (§9.6); a pattern that can not fail
        // always matches.
        body.push(Stmt::Unreachable);
        out.push(Stmt::Block { label: ok, body });
    }

    /// Shared failure path of fitted patterns, including one-clause choices.
    fn assert_failure(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, span: Span, v: Atom, at: u32) {
        let module = self.program.module(fx.module);
        let text = &self.graph.sources.get(module.source).text;
        let source = text
            .get(span.start as usize..span.end as usize)
            .unwrap_or("");
        // §1.1: `\r\n` and `\r` are the same as `\n`.
        let source = source.replace("\r\n", "\n").replace('\r', "\n");
        let prefix = Atom::String(format!("pattern {source} did not match "));
        let shown = let_value(
            fx,
            out,
            Value::Op(
                Op::Runtime {
                    name: "core_inspect",
                    arity: 1,
                },
                vec![v],
            ),
            None,
        );
        let m = let_value(
            fx,
            out,
            Value::Op(Op::StringAppend, vec![prefix, shown]),
            None,
        );
        let failure = let_value(
            fx,
            out,
            Value::Record {
                ctor: 2,
                ty: 1,
                fields: vec![m],
            },
            None,
        );
        let a = let_value(fx, out, Value::Service(fx.env, ASSERT_SERVICE), None);
        let f = let_value(fx, out, Value::Field(a, 0), None);
        let site = self.site(fx, at);
        let_value(
            fx,
            out,
            Value::Call {
                callee: Callee::Value(f),
                args: vec![failure],
                env: fx.env,
            },
            site,
        );
    }

    /// Binds a pattern that `plain` takes.
    fn bind(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, p: &Pattern, v: Atom) {
        match &p.kind {
            PatternKind::Var(b) => Self::bind_var(fx, out, b, v),
            PatternKind::Discard => {}
            PatternKind::Typed { binder, .. } => {
                if let Some(b) = binder {
                    Self::bind_var(fx, out, b, v);
                }
            }
            PatternKind::Named { binder, pattern } => {
                Self::bind_var(fx, out, binder, v.clone());
                self.bind(fx, out, pattern, v);
            }
            PatternKind::Tuple(parts) => {
                for (i, part) in parts.iter().enumerate() {
                    let f = let_value(fx, out, Value::Field(v.clone(), i as u32), None);
                    self.bind(fx, out, part, f);
                }
            }
            PatternKind::Constructor { args, .. } => {
                for (arg, i) in self.fields_of(fx, p, args) {
                    let f = let_value(fx, out, Value::Field(v.clone(), i), None);
                    self.bind(fx, out, arg, f);
                }
            }
            PatternKind::List {
                rest: Some(Some(b)),
                ..
            } => Self::bind_var(fx, out, b, v),
            PatternKind::List { .. } => {}
            _ => out.push(Stmt::Unreachable),
        }
    }
}

/// The ABI type of a type that the host can take (§10.3, B.2). `Never`
/// has no values, so it is passed as nothing, like `Nil`.
fn abi_type(t: HostType) -> AbiType {
    match t {
        HostType::Nil => AbiType::Nil,
        HostType::Bool => AbiType::Bool,
        HostType::Int => AbiType::Int,
        HostType::Float => AbiType::Float,
        HostType::String => AbiType::String,
        HostType::BitArray => AbiType::BitArray,
        HostType::Opaque(id) => AbiType::Externref(id.0),
    }
}

/// The standard service (§9.7): the type `Io` of the library module
/// `i/io`. No other package may have a module path that starts with `i/`
/// (§2.1), so this is that type and no other. A host service declared
/// elsewhere with the namespace `i:io/v1` is a normal host service.
fn is_standard_service(program: &hir::Program, t: TypeId) -> bool {
    let def = program.ty(t);
    def.name == "Io" && program.module(def.module).path == "i/io"
}

/// The parts of B.2 of a function at the host boundary.
fn host_abi(f: &ilang_check::hir::HostFn) -> ExportAbi {
    ExportAbi {
        params: f.params.iter().map(|t| abi_type(*t)).collect(),
        result: abi_type(f.result),
        errors: f.errors.iter().map(|t| abi_type(*t)).collect(),
    }
}

fn names(ts: &[AbiType]) -> Vec<&'static str> {
    ts.iter().map(|t| t.name()).collect()
}

/// The `ilang.abi` metadata (B.6).
fn abi_json(
    target: ir::Target,
    entry: Option<(String, bool)>,
    tests: &[(String, bool)],
    exports: &[(String, ExportAbi, bool)],
    imports: &[Import],
) -> String {
    let entry = match entry {
        Some((name, is_async)) => serde_json::json!({ "name": name, "async": is_async }),
        None => serde_json::Value::Null,
    };
    let tests: Vec<serde_json::Value> = tests
        .iter()
        .map(|(t, is_async)| serde_json::json!({ "name": t, "async": is_async }))
        .collect();
    let mut map = serde_json::Map::new();
    for (name, abi, is_async) in exports {
        map.insert(
            name.clone(),
            serde_json::json!({
                "params": names(&abi.params),
                "result": abi.result.name(),
                "errors": names(&abi.errors),
                "async": is_async,
            }),
        );
    }
    let mut services = serde_json::Map::new();
    let mut externals = serde_json::Map::new();
    for x in imports {
        let (table, sig) = match x.kind {
            // The module gives it itself (B.7).
            ImportKind::WasiStdout => continue,
            ImportKind::External => (
                &mut externals,
                serde_json::json!({
                    "params": names(&x.abi.params),
                    "result": x.abi.result.name(),
                }),
            ),
            ImportKind::Service | ImportKind::AsyncService => (
                &mut services,
                serde_json::json!({
                    "params": names(&x.abi.params),
                    "result": x.abi.result.name(),
                    "errors": names(&x.abi.errors),
                    "async": x.kind == ImportKind::AsyncService,
                }),
            ),
        };
        let module = table
            .entry(x.module.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let serde_json::Value::Object(m) = module {
            m.insert(x.name.clone(), sig);
        }
    }
    let mut abi = serde_json::json!({
        "version": 3,
        "entry": entry,
        "tests": tests,
        "exports": map,
        "services": services,
        "externals": externals,
    });
    // Only a module of the WASI target says its target (B.6, B.7).
    if target == ir::Target::Wasi {
        abi["target"] = target.name().into();
    }
    abi.to_string()
}
