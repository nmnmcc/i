//! From the resolved program (HIR) and its types to the intermediate form
//! of `ir.rs`.
//!
//! - Every function expression becomes a function of the intermediate form
//!   with explicit captures. A top-level function is worked out once, so all its
//!   uses are the same value: a constant closure, and its known calls are direct.
//! - Constructors and library functions used as values are functions
//!   with one constant closure each, so they have one identity.
//! - Evaluation is from left to right, and only the tail positions (the last
//!   expression of a function body, of a block, and the branches of a chain or a
//!   match in tail position) become tail calls.
//! - `case` tests the clauses one after the other, each pattern with
//!   plain tests.
//! - Every call, operator, and `<<` of the program source that may stop has a
//!   site on the shadow stack. A call in tail position, also of a
//!   library function, has a tail site: it is only the first position.
//! - Top-level values are worked out in `ilang_start`, in `order.init`.
//! - Every call passes the environment of its place: environment
//!   parameters read a slot of it, and environment statements make a new one
//!   for the rest of their block.
//! - `fail.raise`, `fail.catch`, `resource.ensure`, and the four functions
//!   of `i/control` are operations of the code generator (`emit.rs`);
//!   `fail.from_result`, `fail.map_error`, `resource.acquire`,
//!   `assertion.catch`, and `assert` are i code.
//! - A binding or a parameter whose pattern may fail (the checker decides)
//!   calls `fail` of the `Assert` service when it does not match.
//! - The host boundary: each external function
//!   outside the library, and each field of a host service that an entry
//!   point needs, is an import with a function that calls it
//!   ([`Value::HostCall`]). The host services are records of these functions
//!   in the entry's chosen environment. A function of an async host service starts
//!   its import and suspends to the root boundary of its async entry point
//!   ([`Value::HostStart`]); when the host answers, it reads the answer
//!   ([`Value::HostReply`]). Entry wrappers construct the checked provider
//!   graph; its host modes and roles determine asynchronous admission.
//!
//! The calls that may suspend are found and split afterwards (`control.rs`).

use crate::ir::Import;
use crate::ir::Stmt;
use crate::ir::{self, *};
use ilang_check::hir::{self, *};
use ilang_check::platform::{self, Stub};
use ilang_check::types::{
    Access, Head, HostType, LitValue, ShownType, TestMember, TestValues, Typed,
};
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
    /// The host the module is made for.
    pub target: ir::Target,
}

/// The WASI functions that the runtime imports on the WASI target.
const WASI: &str = "wasi_snapshot_preview1";
const WASI_RUNTIME: [&str; 2] = ["fd_write", "proc_exit"];

/// How a constructor is represented (`runtime/mod.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CtorRepr {
    Bool(bool),
    Record { ctor: u32, ty: u32 },
}

#[derive(Clone, Debug)]
struct CtorInfo {
    repr: CtorRepr,
    labels: Vec<Option<String>>,
}

/// How a custom type is tested at run time.
#[derive(Clone, Copy, Debug)]
enum TypeRepr {
    /// The prelude's `True` or `False`: the two values of `Bool = True | False`.
    Bool(bool),
    Record(u32),
    /// The prelude's `Ok` or `Error`: one constructor of the run-time type
    /// of `Result = Ok | Error`, which the host also makes.
    Ctor(u32, u32),
    /// A type with no constructors: its values are host references.
    Host,
}

/// What a callee is.
enum Target {
    Ctor(CtorRef),
    Op(Op),
    Callee(Callee),
}

/// The library functions whose bodies are in the runtime.
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
    "portable",
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
    "core_portable",
];

/// The state of one function while it is lowered.
struct Fx {
    module: ModuleId,
    /// Code of the library: it has no sites.
    library: bool,
    locals: Vec<LocalType>,
    numeric: HashMap<Local, ilang_check::types::NumericFact>,
    vars: HashMap<VarId, Local>,
    labels: u32,
    /// The names of the binding whose direct value is being lowered.
    pending: Option<HashSet<VarId>>,
    /// Captures to fill once those names are bound.
    patches: Vec<(Local, u32, VarId)>,
    /// The environment of the place being lowered.
    env: Env,
    /// In the body of a light boundary: the names of its handle, which is
    /// local 0; `None` once one environment statement is made lazy.
    light: Option<HashSet<VarId>>,
    /// Whether the statements being lowered are the start of a light
    /// boundary's body, where its environment is still its own.
    light_start: bool,
    /// The names of environment parameters that hold the slot of their
    /// service as it is (`Value::ServiceRaw`): every use is the callee of
    /// `x.f(…)`, which is lowered as `Callee::Lazy`.
    lazy_params: HashSet<VarId>,
}

impl Fx {
    fn new(module: ModuleId, library: bool) -> Fx {
        Fx {
            module,
            library,
            locals: Vec::new(),
            numeric: HashMap::new(),
            vars: HashMap::new(),
            labels: 0,
            pending: None,
            patches: Vec::new(),
            env: Env::Param,
            light: None,
            light_start: false,
            lazy_params: HashSet::new(),
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

/// `{ e }` with one expression is `e`.
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
        PatternKind::Quote { captures, .. } => {
            1 + captures
                .iter()
                .map(|(_, p)| pattern_values(p))
                .sum::<usize>()
        }
        PatternKind::Named { pattern, .. } | PatternKind::Keyed { pattern, .. } => {
            1 + pattern_values(pattern)
        }
        PatternKind::Constructor { args, .. }
        | PatternKind::Tuple(args)
        | PatternKind::Data { parts: args, .. }
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
        ExprKind::Multi(groups) => pending_values(groups.iter().map(|g| &g.value)),
        ExprKind::Record { base, fields } => pending_values(
            base.iter()
                .map(Box::as_ref)
                .chain(fields.iter().map(|(_, value)| value)),
        ),
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
    /// How many times each name is used.
    uses: HashMap<VarId, u32>,
    /// The calls of a top-level name whose first written argument (or the
    /// left side of `|>`) is a local name: the callee, the name, and the
    /// span of the call, which keys its argument order.
    first_args: Vec<(DefId, VarId, Span)>,
    /// How deep in function literals the walk is.
    depth: u32,
    /// The calls `x.f(…)` of a local name `x` outside function literals:
    /// the name and the span of the callee `x.f`.
    field_calls: Vec<(VarId, Span)>,
    /// The environment statements whose value is a constructor applied to
    /// function literals and local names (as `lazy_provide` takes them).
    lazy_envs: Vec<Span>,
}

impl Walk {
    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(ValueRef::Local(v)) => {
                self.used.insert(*v);
                *self.uses.entry(*v).or_default() += 1;
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name(_)
            | ExprKind::Error => {}
            ExprKind::Tuple(items) => items.iter().for_each(|x| self.expr(x)),
            ExprKind::Multi(groups) => groups.iter().for_each(|g| self.expr(&g.value)),
            ExprKind::Record { base, fields } => {
                if let Some(base) = base {
                    self.expr(base);
                }
                for (_, value) in fields {
                    self.expr(value);
                }
            }
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
            ExprKind::Fn(f) => {
                self.depth += 1;
                self.function(f);
                self.depth -= 1;
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                for c in clauses {
                    c.patterns.iter().for_each(|p| self.pattern(p));
                    c.guard.iter().for_each(|g| self.expr(g));
                    self.expr(&c.body);
                }
            }
            ExprKind::Call { callee, args } => {
                if let (ExprKind::Name(ValueRef::Def(d)), Some(first)) =
                    (&callee.kind, args.first())
                    && let ExprKind::Name(ValueRef::Local(v)) = first.value.kind
                {
                    self.first_args.push((*d, v, e.span));
                }
                if self.depth == 0
                    && args.iter().all(|a| a.label.is_none())
                    && let ExprKind::Field { target, .. } = &callee.kind
                    && let ExprKind::Name(ValueRef::Local(v)) = target.kind
                {
                    self.field_calls.push((v, callee.span));
                }
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Binary {
                op: BinOp::Pipe,
                lhs,
                rhs,
            } => {
                // `a |> f(…)` is `f(a, …)`: `a` is the first
                // written argument.
                let (callee, args) = match &rhs.kind {
                    ExprKind::Call { callee, args } => (callee.as_ref(), &args[..]),
                    _ => (rhs.as_ref(), &[][..]),
                };
                if let (ExprKind::Name(ValueRef::Def(d)), ExprKind::Name(ValueRef::Local(v))) =
                    (&callee.kind, &lhs.kind)
                {
                    self.first_args.push((*d, *v, e.span));
                }
                self.expr(lhs);
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
                hir::Stmt::Env { value, span, .. } => {
                    if let ExprKind::Call { callee, args } = &ungroup(value).kind
                        && let ExprKind::Name(ValueRef::Ctor(_)) = &ungroup(callee).kind
                        && args.iter().all(|a| {
                            matches!(
                                ungroup(&a.value).kind,
                                ExprKind::Fn(_) | ExprKind::Name(ValueRef::Local(_))
                            )
                        })
                    {
                        self.lazy_envs.push(*span);
                    }
                    self.expr(value)
                }
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
            PatternKind::Keyed { pattern, .. } => self.pattern(pattern),
            PatternKind::Var(b) => self.binder(b),
            PatternKind::Typed { binder, .. } => binder.iter().for_each(|b| self.binder(b)),
            PatternKind::Named { binder, pattern } => {
                self.binder(binder);
                self.pattern(pattern);
            }
            PatternKind::Prefix { rest, .. } => rest.iter().for_each(|b| self.binder(b)),
            PatternKind::Constructor { args, .. }
            | PatternKind::Tuple(args)
            | PatternKind::Data { parts: args, .. } => args.iter().for_each(|a| self.pattern(a)),
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

/// The names of a light boundary literal's handle, when its parameter is a
/// plain name (`b` or `b: T`).
fn light_names(f: &Function) -> Option<HashSet<VarId>> {
    let mut patterns = f.params.iter().filter_map(|p| match p {
        Param::Pattern { pattern, .. } => Some(pattern),
        Param::Env { .. } => None,
    });
    let binder = match &patterns.next()?.kind {
        PatternKind::Var(b) => b,
        PatternKind::Typed {
            binder: Some(b), ..
        } => b,
        _ => return None,
    };
    Some(HashSet::from([binder.var]))
}

/// A bit array format.
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
    lower_tests_with_bindings(graph, program, typed, options, &[])
}

pub fn lower_tests_with_bindings(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
    tests: &[(DefId, String)],
) -> Result<(ir::Program, Vec<serde_json::Value>), Vec<Diagnostic>> {
    lower_tests_numeric(
        graph,
        program,
        typed,
        options,
        tests,
        crate::Numeric::Reference,
    )
}

pub(crate) fn lower_tests_numeric(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
    tests: &[(DefId, String)],
    numeric: crate::Numeric,
) -> Result<(ir::Program, Vec<serde_json::Value>), Vec<Diagnostic>> {
    if !typed.coalescing_errors().is_empty() {
        return Err(typed.coalescing_errors().to_vec());
    }
    let mut stubs = std::collections::BTreeMap::new();
    let mut errors = Vec::new();
    let mut all_stubs: Vec<_> = platform::stubs(program, typed, options.target.platform())
        .into_iter()
        .collect();
    mark_later_entries(program, typed, &mut all_stubs);
    for (entry, stub) in all_stubs {
        if program.module(entry.module).kind == ModuleKind::Test
            && (!options.tests || options.target == ir::Target::Wasi)
        {
            continue;
        }
        match stub {
            Ok(stub) => {
                // A synchronous export may not be async.
                let synchronous = graph.root.is_some_and(|root| {
                    graph.packages[root].wasm_synchronous.iter().any(|e| {
                        e.module == program.module(entry.module).path
                            && e.function == program.def(entry).name
                    })
                });
                if synchronous && stub.is_async {
                    errors.push(Diagnostic::error(
                        platform::NOT_PROVIDED,
                        Some(program.module(entry.module).source),
                        program.def(entry).span,
                        format!(
                            "synchronous export `{}` needs an asynchronous platform service",
                            program.def(entry).name
                        ),
                    ));
                    continue;
                }
                stubs.insert(entry, stub);
            }
            Err(more) => errors.extend(more),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut l = Lowerer::new(graph, program, typed, options, None);
    l.specialize_equality = numeric == crate::Numeric::Scalars;
    l.install_stubs(stubs);
    l.extra_tests = tests.to_vec();
    l.run();
    if l.diags.is_empty() {
        l.out.numeric = numeric;
        let may = if numeric == crate::Numeric::Scalars {
            crate::specialize::run(&mut l.out);
            crate::numeric::run(&mut l.out)
        } else {
            crate::control::may_suspend(&l.out)
        };
        crate::control::split_with_may(&mut l.out, &may);
        Ok((l.out, l.bindings))
    } else {
        Err(l.diags)
    }
}

/// Whether `later` is reachable from the entry through HIR references,
/// including function bodies made in an activated body. The implicit
/// services are ambient, so an entry's need does not show them: an entry
/// that reaches a function needing `Runner` or `Scheduler` runs in
/// `task.run`, as an entry that names them does.
fn mark_later_entries(
    program: &hir::Program,
    typed: &Typed,
    stubs: &mut [(DefId, Result<platform::Stub, Vec<Diagnostic>>)],
) {
    let mut later = BTreeSet::new();
    for (mi, m) in program.modules.iter().enumerate() {
        for (di, d) in m.defs.iter().enumerate() {
            let id = DefId {
                module: ModuleId(mi as u32),
                index: di as u32,
            };
            match &d.kind {
                DefKind::ServiceAccessor { ty }
                    if program
                        .ty(*ty)
                        .import
                        .as_ref()
                        .is_some_and(|i| i.mode == hir::ImportMode::Later) =>
                {
                    later.insert(id);
                }
                DefKind::External { .. }
                    if program
                        .import_modes
                        .get(&id)
                        .is_some_and(|m| *m == hir::ImportMode::Later) =>
                {
                    later.insert(id);
                }
                _ => {}
            }
        }
    }
    let [_, _, _, runner, scheduler] = platform::request_types(program);
    let tasks_types: Vec<TypeRef> = [runner, scheduler]
        .into_iter()
        .flatten()
        .map(TypeRef::Custom)
        .collect();
    if later.is_empty() && tasks_types.is_empty() {
        return;
    }
    let mut edges: std::collections::HashMap<DefId, BTreeSet<DefId>> =
        std::collections::HashMap::new();
    let mut tasks = BTreeSet::new();
    for (mi, m) in program.modules.iter().enumerate() {
        let module = ModuleId(mi as u32);
        for b in &m.bindings {
            let mut set = Refs::default();
            refs(&b.binding.value, &mut set);
            let needs_tasks = set.envs.iter().any(|span| {
                typed
                    .services
                    .get(&(module, *span))
                    .is_some_and(|t| tasks_types.contains(t))
            });
            for d in b.defs.iter().copied() {
                edges.entry(d).or_default().extend(set.defs.iter().copied());
                if needs_tasks {
                    tasks.insert(d);
                }
            }
        }
    }
    for (entry, stub) in stubs.iter_mut() {
        let Ok(stub) = stub else { continue };
        let mut seen = BTreeSet::from([*entry]);
        let mut queue = vec![*entry];
        let mut found = false;
        let mut found_tasks = false;
        while let Some(d) = queue.pop() {
            found |= later.contains(&d);
            found_tasks |= tasks.contains(&d);
            if found && found_tasks {
                break;
            }
            if let Some(next) = edges.get(&d) {
                for &n in next {
                    if seen.insert(n) {
                        queue.push(n);
                    }
                }
            }
        }
        if found_tasks && !stub.tasks {
            stub.tasks = true;
            stub.is_async |= stub.target != platform::Target::Wasi;
        }
        if found || stub.is_async {
            // An async entry opens the request root (`platform::stub`).
            stub.is_async = true;
            stub.root = true;
            stub.owner = true;
            stub.await_ = true;
        }
    }
}

/// What a top-level value refers to: the definitions it names, and the
/// spans of the names of the environment parameters of its functions.
#[derive(Default)]
struct Refs {
    defs: BTreeSet<DefId>,
    envs: Vec<Span>,
}

fn refs_block(b: &hir::Block, out: &mut Refs) {
    for s in &b.stmts {
        match s {
            hir::Stmt::Bind(b) => refs(&b.value, out),
            hir::Stmt::Env { value, .. } | hir::Stmt::Expr(value) => refs(value, out),
        }
    }
}

fn refs(e: &hir::Expr, out: &mut Refs) {
    match &e.kind {
        hir::ExprKind::Name(hir::ValueRef::Def(d)) => {
            out.defs.insert(*d);
        }
        hir::ExprKind::Tuple(ts) | hir::ExprKind::List { items: ts, .. } => {
            ts.iter().for_each(|t| refs(t, out))
        }
        hir::ExprKind::Record { base, fields } => {
            base.iter().for_each(|b| refs(b, out));
            fields.iter().for_each(|(_, v)| refs(v, out));
        }
        hir::ExprKind::BitArray(segs) => segs.iter().for_each(|s| refs(&s.value, out)),
        hir::ExprKind::Block(b) => refs_block(b, out),
        hir::ExprKind::Fn(f) => {
            for p in &f.params {
                if let hir::Param::Env { name, .. } = p {
                    out.envs.push(name.span);
                }
            }
            refs_block(&f.body, out)
        }
        hir::ExprKind::Multi(gs) => gs.iter().for_each(|g| refs(&g.value, out)),
        hir::ExprKind::Case { subject, clauses } => {
            refs(subject, out);
            for c in clauses {
                c.guard.iter().for_each(|g| refs(g, out));
                refs(&c.body, out);
            }
        }
        hir::ExprKind::Call { callee, args } => {
            refs(callee, out);
            args.iter().for_each(|a| refs(&a.value, out));
        }
        hir::ExprKind::Field { target, .. } | hir::ExprKind::TupleIndex { target, .. } => {
            refs(target, out)
        }
        hir::ExprKind::RecordUpdate { base, fields, .. } => {
            refs(base, out);
            fields.iter().for_each(|(_, v)| refs(v, out));
        }
        hir::ExprKind::Unary { expr, .. } => refs(expr, out),
        hir::ExprKind::Binary { lhs, rhs, .. } => {
            refs(lhs, out);
            refs(rhs, out);
        }
        _ => {}
    }
}

/// Lower one consumer-selected collection target as the sole tool root.
/// The target remains separate from program entries, tests and host exports.
pub fn lower_selected_with_bindings(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
    target: DefId,
    export: &str,
) -> Result<(ir::Program, Vec<serde_json::Value>, bool), Vec<Diagnostic>> {
    lower_selected_numeric(
        graph,
        program,
        typed,
        options,
        target,
        export,
        crate::Numeric::Reference,
    )
}

pub(crate) fn lower_selected_numeric(
    graph: &PackageGraph,
    program: &hir::Program,
    typed: &Typed,
    options: Options,
    target: DefId,
    export: &str,
    numeric: crate::Numeric,
) -> Result<(ir::Program, Vec<serde_json::Value>, bool), Vec<Diagnostic>> {
    if !typed.coalescing_errors().is_empty() {
        let mut diags = typed.coalescing_errors().to_vec();
        ilang_check::map_expansion_diagnostics(program, &graph.sources, &mut diags);
        return Err(diags);
    }
    let mut l = Lowerer::new(
        graph,
        program,
        typed,
        options,
        Some((target, export.to_owned())),
    );
    let stub = platform::stub(program, typed, target, options.target.platform())?;
    let mut pair = vec![(target, Ok(stub))];
    mark_later_entries(program, typed, &mut pair);
    let (_, stub) = pair.pop().expect("one stub");
    let stub = stub.expect("the stub was Ok");
    l.specialize_equality = numeric == crate::Numeric::Scalars;
    l.install_stubs(std::collections::BTreeMap::from([(target, stub)]));
    l.run();
    let is_async = l.selected_async;
    if l.diags.is_empty() {
        l.out.numeric = numeric;
        let may = if numeric == crate::Numeric::Scalars {
            crate::specialize::run(&mut l.out);
            crate::numeric::run(&mut l.out)
        } else {
            crate::control::may_suspend(&l.out)
        };
        crate::control::split_with_may(&mut l.out, &may);
        Ok((l.out, l.bindings, is_async))
    } else {
        ilang_check::map_expansion_diagnostics(program, &graph.sources, &mut l.diags);
        Err(l.diags)
    }
}

struct Lowerer<'a> {
    graph: &'a PackageGraph,
    /// The entry stub of each entry.
    stubs: std::collections::BTreeMap<DefId, Stub>,
    program: &'a hir::Program,
    typed: &'a Typed,
    options: Options,
    selected: Option<(DefId, String)>,
    selected_async: bool,
    extra_tests: Vec<(DefId, String)>,
    packages: HashSet<PackageId>,
    ctors: HashMap<CtorRef, CtorInfo>,
    types: HashMap<TypeId, TypeRepr>,
    /// The label sets of data, as `ir::Program::shapes`.
    shapes: std::cell::RefCell<Vec<Vec<String>>>,
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
    /// The service id of each service type constructor.
    services: HashMap<TypeRef, u32>,
    /// The function that calls each import with its declared result decoder.
    import_funcs: HashMap<(ImportId, AbiType), FuncId>,
    /// Where each import is declared first, by its index in `out.imports`.
    import_sites: Vec<(ModuleId, Span)>,
    bindings: Vec<serde_json::Value>,
    /// The function of each external function outside the library,
    /// or `None` when it has a problem, which is told one time.
    external_funcs: HashMap<DefId, Option<FuncId>>,
    /// The import function of each field of a host service, or `None` when
    /// it has a problem, which is told one time. One function per field, so
    /// every record of the service has the same field values.
    service_fields: HashMap<(TypeId, u32), Option<FuncId>>,
    /// The function of the accessor `make_<type>` of each `@import` type.
    accessor_funcs: HashMap<TypeId, FuncId>,
    /// The function that starts an async import and gives its request cell
    /// without awaiting it (`hostcall.call`).
    start_funcs: HashMap<(ImportId, AbiType), FuncId>,
    /// The content hash of each package that declares an import, computed
    /// once: hashing every source of the package per import was measured at
    /// a twentieth of a test build.
    package_hashes: HashMap<PackageId, String>,
    module_hashes: HashMap<ilang_package::SourceId, String>,
    /// Whether structural equality may be specialized for closed types
    /// (O2/O3). O0 keeps the generic comparison as the reference lowering.
    specialize_equality: bool,
    /// The services that an environment statement may provide lazily
    /// (`lazy_provide`), found before lowering: a call `x.f(…)` of an
    /// environment parameter of one of them is a `Callee::Lazy`.
    lazy_candidates: HashSet<u32>,
    out: ir::Program,
    diags: Vec<Diagnostic>,
}

impl<'a> Lowerer<'a> {
    fn new(
        graph: &'a PackageGraph,
        program: &'a hir::Program,
        typed: &'a Typed,
        options: Options,
        selected: Option<(DefId, String)>,
    ) -> Self {
        let mut l = Lowerer {
            graph,
            stubs: Default::default(),
            program,
            typed,
            options,
            selected,
            selected_async: false,
            extra_tests: Vec::new(),
            packages: HashSet::new(),
            ctors: HashMap::new(),
            types: HashMap::new(),
            shapes: Default::default(),
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
            service_fields: HashMap::new(),
            accessor_funcs: HashMap::new(),
            start_funcs: HashMap::new(),
            package_hashes: HashMap::new(),
            module_hashes: HashMap::new(),
            lazy_candidates: HashSet::new(),
            specialize_equality: false,
            out: ir::Program::default(),
            diags: Vec::new(),
        };
        l.find_packages();
        l.number_types();
        l.number_services();
        l
    }

    fn install_stubs(&mut self, mut stubs: std::collections::BTreeMap<DefId, Stub>) {
        self.link_later_imports(&mut stubs);
        self.stubs = stubs;
        // The stub gives services that the source may not name.
        let mut given: Vec<TypeId> = platform::request_types(self.program)
            .into_iter()
            .flatten()
            .collect();
        for stub in self.stubs.values() {
            given.extend(stub.hosts.iter().copied());
        }
        for t in given {
            let next = self.services.len() as u32;
            self.services.entry(TypeRef::Custom(t)).or_insert(next);
        }
        self.out.services = self.services.len() as u32;
    }

    /// An entry that reaches a `"later"` import without naming its service
    /// in its need (through an accessor `make_<type>` of a `"later"` service,
    /// or a raw `@import(…, "later")` declaration) waits for the host: it is
    /// async and opens the request root. Reachability is by reference from
    /// the entry and the top-level initializers.
    fn link_later_imports(&self, stubs: &mut std::collections::BTreeMap<DefId, Stub>) {
        if self.options.target == ir::Target::Wasi {
            return;
        }
        let program = self.program;
        let mut later = HashSet::new();
        for (mi, m) in program.modules.iter().enumerate() {
            for (di, d) in m.defs.iter().enumerate() {
                let id = DefId {
                    module: ModuleId(mi as u32),
                    index: di as u32,
                };
                let waits = match d.kind {
                    DefKind::ServiceAccessor { ty } => program
                        .ty(ty)
                        .host
                        .as_ref()
                        .is_some_and(|h| h.mode != HostMode::Now),
                    DefKind::External { .. } => {
                        program.import_modes.get(&id) == Some(&ImportMode::Later)
                    }
                    _ => false,
                };
                if waits {
                    later.insert(id);
                }
            }
        }
        if later.is_empty() {
            return;
        }
        for (entry, stub) in stubs.iter_mut() {
            if stub.is_async && stub.requests() {
                continue;
            }
            if self
                .tool_reachable(*entry)
                .iter()
                .any(|d| later.contains(d))
            {
                stub.is_async = true;
                stub.root = true;
                stub.owner = true;
                stub.await_ = true;
            }
        }
    }

    /// One dense id for each service type constructor of the program:
    /// `Assert` first, then the types of the environment parameters and
    /// statements, with ordinary types before host types.
    fn number_services(&mut self) {
        let prelude = self.program.prelude;
        let assert = self
            .program
            .types
            .iter()
            .position(|t| Some(t.module) == prelude && t.name == "Assert")
            .map(|i| TypeRef::Custom(TypeId(i as u32)));
        let fail = self
            .program
            .types
            .iter()
            .position(|t| Some(t.module) == prelude && t.name == "Fail")
            .map(|i| TypeRef::Custom(TypeId(i as u32)));
        let mut all: Vec<TypeRef> = self.typed.services.values().copied().collect();
        for supply in self.typed.env_supplies.values() {
            match supply {
                ilang_check::types::EnvSupply::Row(keys)
                | ilang_check::types::EnvSupply::Data(keys) => all.extend(keys),
            }
        }
        all.extend(self.typed.env_captures.values().flatten().copied());
        let key = |t: &TypeRef| match t {
            TypeRef::Custom(id) => (
                u8::from(self.program.types[id.0 as usize].host.is_some()),
                id.0,
            ),
            TypeRef::Builtin(b) => (2, *b as u32),
        };
        all.sort_by_key(key);
        all.dedup();
        if let Some(a) = assert {
            self.services.insert(a, ASSERT_SERVICE);
        }
        if let Some(f) = fail {
            self.services.insert(f, FAIL_SERVICE);
        }
        let mut next = FAIL_SERVICE + 1;
        for t in all {
            if let std::collections::hash_map::Entry::Vacant(e) = self.services.entry(t) {
                e.insert(next);
                next += 1;
            }
        }
        self.out.services = next;
        self.out.request_owner = self
            .program
            .types
            .iter()
            .enumerate()
            .find(|(_, t)| t.name == "Owner" && self.program.module(t.module).path == "i/request")
            .and_then(|(i, _)| {
                self.services
                    .get(&TypeRef::Custom(TypeId(i as u32)))
                    .copied()
            });
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
    /// and what it uses; its dev-dependencies only with the tests.
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
            && (module.kind != ModuleKind::Test
                || (self.options.tests && self.options.target != ir::Target::Wasi))
    }

    /// A definition of the library package, by module path and name.
    fn library_def(&self, path: &str, name: &str) -> DefId {
        let (mi, m) = self
            .program
            .modules
            .iter()
            .enumerate()
            .find(|(_, m)| m.path == path)
            .unwrap_or_else(|| panic!("library module {path}"));
        let index = m
            .defs
            .iter()
            .position(|d| d.name == name)
            .unwrap_or_else(|| panic!("library definition {path}.{name}"));
        DefId {
            module: ModuleId(mi as u32),
            index: index as u32,
        }
    }

    fn is_library(&self, m: ModuleId) -> bool {
        self.graph.is_language(self.program.module(m).package)
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
                    "True" => Some(TypeRepr::Bool(true)),
                    "False" => Some(TypeRepr::Bool(false)),
                    "Ok" => Some(TypeRepr::Ctor(0, 0)),
                    "Error" => Some(TypeRepr::Ctor(0, 1)),
                    "AssertionFailure" => Some(TypeRepr::Record(1)),
                    "Assert" => Some(TypeRepr::Record(2)),
                    "Fail" => Some(TypeRepr::Record(3)),
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
                    (Some(TypeRepr::Bool(b)), _) => CtorRepr::Bool(b),
                    (Some(TypeRepr::Ctor(ty, ctor)), _) => CtorRepr::Record { ctor, ty },
                    (Some(TypeRepr::Record(ty)), name) => CtorRepr::Record {
                        ctor: match name {
                            "AssertionFailure" => 2,
                            "Assert" => 3,
                            _ => 4,
                        },
                        ty,
                    },
                    _ => {
                        let TypeRepr::Record(ty) = repr else {
                            unreachable!("a type with constructors is a record")
                        };
                        self.out.constructors.push(c.name.clone());
                        let mut parts: Vec<_> = c
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(i, f)| (i as u32, f.label.as_ref().map(|l| l.name.clone())))
                            .collect();
                        if parts.iter().all(|(_, label)| label.is_none()) {
                            parts.clear();
                        } else {
                            parts.sort_by(|a, b| a.1.cmp(&b.1));
                        }
                        self.out.constructor_parts.push(parts);
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
        self.run_all();
        self.out.shapes = self.shapes.take();
    }

    /// Records that the function of the group closure `value` has a shape
    /// with labels, so that its closure shows its labels (field 2 of
    /// `$clo{n}`). The value is a constant closure or the
    /// closure just made.
    fn label_shape(&mut self, out: &[Stmt], value: &Atom, id: ShapeId) {
        let func = match value {
            Atom::Func(f) => Some(*f),
            Atom::Local(l) => match out.last() {
                Some(Stmt::Let {
                    dst,
                    value: Value::Closure { func, .. },
                    ..
                }) if dst == l => Some(*func),
                _ => None,
            },
            _ => None,
        };
        if let Some(f) = func {
            self.out.func_shapes.insert(f, id);
        }
    }

    /// The index of the label set of a data shape (`ir::Program::shapes`).
    fn shape_set(&self, shape: ilang_check::types::ShapeId) -> u32 {
        let labels = shape.labels().to_vec();
        let mut shapes = self.shapes.borrow_mut();
        let i = match shapes.iter().position(|l| *l == labels) {
            Some(i) => i,
            None => {
                shapes.push(labels);
                shapes.len() - 1
            }
        };
        i as u32 + 1
    }

    fn run_all(&mut self) {
        // The services that an environment statement may provide lazily.
        for (mi, m) in self.program.modules.iter().enumerate() {
            let mut w = Walk::default();
            for b in &m.bindings {
                w.expr(&b.binding.value);
            }
            for span in w.lazy_envs {
                if let Some(id) = self
                    .typed
                    .services
                    .get(&(ModuleId(mi as u32), span))
                    .and_then(|t| self.services.get(t))
                {
                    self.lazy_candidates.insert(*id);
                }
            }
        }
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
        // Tool builds retain only externals reachable from their selected root
        // or from required initializers; ordinary builds retain the imports of every
        // external function.
        let reachable = self.selected.as_ref().map(|(d, _)| self.tool_reachable(*d));
        for (mi, m) in self.program.modules.iter().enumerate() {
            let mid = ModuleId(mi as u32);
            if !self.included(mid) || self.is_library(mid) {
                continue;
            }
            for (di, d) in m.defs.iter().enumerate() {
                let id = DefId {
                    module: mid,
                    index: di as u32,
                };
                if matches!(d.kind, DefKind::External { .. })
                    && !self.program.typed_macros.contains(&id)
                    && !self.program.js_values.contains(&id)
                    && reachable.as_ref().is_none_or(|set| set.contains(&id))
                {
                    self.external_func(id);
                }
            }
        }
        self.start();
        // Resolve host leaves before generating wrappers. Import conflicts are
        // source diagnostics; no wrapper is emitted from an invalid binding.
        let hosts: BTreeSet<_> = self
            .stubs
            .values()
            .flat_map(|stub| stub.hosts.iter().copied())
            .collect();
        for host in hosts {
            self.host_service(host);
        }
        if !self.diags.is_empty() {
            return;
        }
        self.entries();
        for &(wat, module, name) in ir::RUNTIME_CALLS {
            let def = self.library_def(module, name);
            let f = self.def_func(def);
            self.constants.insert(f);
            self.out.runtime_calls.push((wat, f));
        }
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
            let func = self.function(b.module, f, &[], name, id);
            self.out.funcs[id.0 as usize] = FuncKind::Code(func);
        }
        self.out.constants = self.constants.iter().copied().collect();
        // Function lowering can discover imports after entry metadata was built.
        if self.selected.is_some() {
            self.out.abi_json = abi_json(self.options.target, None, &[], &[], &self.out.imports);
        } else {
            let mut abi: serde_json::Value =
                serde_json::from_str(&self.out.abi_json).expect("generated ABI must be valid JSON");
            let imports: serde_json::Value = serde_json::from_str(&abi_json(
                self.options.target,
                None,
                &[],
                &[],
                &self.out.imports,
            ))
            .expect("generated import ABI must be valid JSON");
            abi["services"] = imports["services"].clone();
            abi["externals"] = imports["externals"].clone();
            self.out.abi_json = abi.to_string();
        }
    }

    /// Dependency closure of a selected root plus the initializers that every
    /// instance executes. An unused function does not create a host import.
    fn tool_reachable(&self, target: DefId) -> HashSet<DefId> {
        let mut seen = HashSet::new();
        let mut work = vec![target];
        for &binding in &self.program.order.init {
            if !self.included(binding.module) {
                continue;
            }
            let b = &self.program.binding(binding).binding;
            work.extend(
                ilang_check::order::references(&b.pattern, &b.value)
                    .into_iter()
                    .filter_map(|(d, inside)| (!b.direct || !inside).then_some(d)),
            );
        }
        while let Some(id) = work.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let DefKind::Binding { binding, .. } = self.program.def(id).kind {
                let b = &self.program.module(id.module).bindings[binding as usize].binding;
                work.extend(
                    ilang_check::order::references(&b.pattern, &b.value)
                        .into_iter()
                        .map(|(d, _)| d),
                );
            }
        }
        seen
    }

    /// `ilang_start`: the top-level values, each module after the modules it
    /// imports, and in a module the first binding of the source whose dependencies
    /// are worked out comes next.
    fn start(&mut self) {
        let mut fx = Fx::new(ModuleId(0), false);
        let mut out = Vec::new();
        let mut pending = 1;
        // Top-level values may use the implicit services. Give them one
        // startup root for the whole start, as an entry does.
        if !self.program.implicit_services.is_empty() {
            let_value(
                &mut fx,
                &mut out,
                Value::Op(
                    Op::Runtime {
                        name: "request_entry_begin",
                        arity: 0,
                    },
                    vec![],
                ),
                None,
            );
            let [root, owner, await_, runner, _] = platform::request_types(self.program);
            let key = let_value(
                &mut fx,
                &mut out,
                Value::Op(
                    Op::Runtime {
                        name: "request_entry_root",
                        arity: 0,
                    },
                    vec![],
                ),
                None,
            );
            let env = fx.env;
            let factory = self.library_callee("i/request", "_provider_root");
            let value = self.stub_factory(&mut fx, &mut out, factory, vec![Atom::EmptyList, key], env);
            fx.env = self.stub_bind(&mut fx, &mut out, env, root.expect("Root"), value);
            let env = fx.env;
            let factory = self.library_callee("i/request", "_provider_owner");
            let value = self.stub_factory(&mut fx, &mut out, factory, vec![Atom::EmptyList], env);
            fx.env = self.stub_bind(&mut fx, &mut out, env, owner.expect("Owner"), value);
            let env = fx.env;
            let factory = self.library_callee("i/request", "_provider_await");
            let value = self.stub_factory(&mut fx, &mut out, factory, vec![Atom::EmptyList], env);
            fx.env = self.stub_bind(&mut fx, &mut out, env, await_.expect("Await"), value);
            let _ = runner;
        }
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
        out.push(Stmt::Return(Atom::Unit));
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

    /// The program entry, the tests, and the host exports,
    /// and the host services that they need.
    fn entries(&mut self) {
        let program = self.program;
        let mut entry = None;
        let mut tests = Vec::new();
        if let Some((target, name)) = self.selected.clone() {
            let def = program.def(target);
            let callee = self.entry_callee(target, 0);
            let fx = Fx::new(target.module, false);
            let site = self.site(&fx, def.span.start);
            let is_async = self.stubs[&target].is_async;
            let async_body = is_async.then(|| self.async_body(&callee, 0));
            self.selected_async = is_async;
            if self.options.target == ir::Target::Wasi {
                self.out.entry = Some(name.clone());
            }
            self.out.exports.push(Export {
                name,
                callee,
                abi: None,
                site,
                async_body,
            });
            self.out.target = self.options.target;
            return;
        }
        for (mi, m) in program.modules.iter().enumerate() {
            let mid = ModuleId(mi as u32);
            if Some(m.package) != program.root || !self.included(mid) {
                continue;
            }
            for b in &m.bindings {
                if !b.main || b.defs.len() != 1 {
                    continue;
                }
                let d = b.defs[0];
                let name = format!("{}.{}", m.path, program.def(d).name);
                let callee = self.entry_callee(d, 0);
                let fx = Fx::new(mid, false);
                let site = self.site(&fx, b.binding.span.start);
                let is_async = self.stubs[&d].is_async;
                let async_body = is_async.then(|| self.async_body(&callee, 0));
                self.out.exports.push(Export {
                    name: name.clone(),
                    callee,
                    abi: None,
                    site,
                    async_body,
                });
                entry = Some((name, is_async));
            }
        }
        for (d, name) in self.extra_tests.clone() {
            let callee = self.entry_callee(d, 0);
            let fx = Fx::new(d.module, false);
            let site = self.site(&fx, program.def(d).span.start);
            let is_async = self.stubs[&d].is_async;
            let async_body = is_async.then(|| self.async_body(&callee, 0));
            self.out.exports.push(Export {
                name: name.clone(),
                callee,
                abi: None,
                site,
                async_body,
            });
            tests.push((name, is_async));
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
            let callee = self.entry_callee(d, abi.params.len() as u32);
            let is_async = self.stubs[&d].is_async;
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
        if self.options.target == ir::Target::Wasi {
            match &entry {
                Some((name, _)) => self.out.entry = Some(name.clone()),
                None => self.diags.push(Diagnostic::error(
                    code::NOT_SUPPORTED,
                    None,
                    Span::default(),
                    "the WASI target needs a program entry selected from src/main.i or [entries] (B.7)",
                )),
            }
        }
        self.out.target = self.options.target;
        self.out.abi_json = abi_json(
            self.options.target,
            entry,
            &tests,
            &exports,
            &self.out.imports,
        );
    }

    /// The entry stub: the platform's host services in
    /// the environment, then `Root`, `Owner` and `Await` of the request root
    /// when the entry needs one of them or tasks, and `task.run` around the
    /// entry when it needs `Runner` or `Scheduler`.
    fn entry_callee(&mut self, d: DefId, n: u32) -> Callee {
        let stub = self.stubs[&d].clone();
        let mut fx = Fx::new(d.module, false);
        let args: Vec<_> = (0..n)
            .map(|_| Atom::Local(fx.local(LocalType::Value)))
            .collect();
        let mut body = Vec::new();
        let_value(
            &mut fx,
            &mut body,
            Value::Op(
                Op::Runtime {
                    name: "request_entry_begin",
                    arity: 0,
                },
                vec![],
            ),
            None,
        );
        // The completion catch supplies Fail before this stub runs. Keep it
        // while installing the platform services so an ordinary error reaches
        // provider_finish and ends the entry's request context.
        let mut env = Env::Param;
        for &t in &stub.hosts {
            let slot = self.services[&TypeRef::Custom(t)];
            let host = self
                .out
                .host_services
                .iter()
                .find(|h| h.service == slot)
                .expect("checked host service");
            let value = Value::Record {
                ctor: host.ctor,
                ty: host.ty,
                fields: host.fields.iter().copied().map(Atom::Func).collect(),
            };
            let value = let_value(&mut fx, &mut body, value, None);
            env = self.stub_bind(&mut fx, &mut body, env, t, value);
        }
        if stub.requests() {
            let [root, owner, await_, runner, _] = platform::request_types(self.program);
            let key = let_value(
                &mut fx,
                &mut body,
                Value::Op(
                    Op::Runtime {
                        name: "request_entry_root",
                        arity: 0,
                    },
                    vec![],
                ),
                None,
            );
            let factory = self.library_callee("i/request", "_provider_root");
            let value =
                self.stub_factory(&mut fx, &mut body, factory, vec![Atom::EmptyList, key], env);
            env = self.stub_bind(&mut fx, &mut body, env, root.expect("Root"), value);
            let factory = self.library_callee("i/request", "_provider_owner");
            let value = self.stub_factory(&mut fx, &mut body, factory, vec![Atom::EmptyList], env);
            env = self.stub_bind(&mut fx, &mut body, env, owner.expect("Owner"), value);
            let factory = self.library_callee("i/request", "_provider_await");
            let value = self.stub_factory(&mut fx, &mut body, factory, vec![Atom::EmptyList], env);
            env = self.stub_bind(&mut fx, &mut body, env, await_.expect("Await"), value);
            if stub.tasks {
                let factory = self.library_callee("i/task", "_provider_runner");
                let value =
                    self.stub_factory(&mut fx, &mut body, factory, vec![Atom::EmptyList], env);
                env = self.stub_bind(&mut fx, &mut body, env, runner.expect("Runner"), value);
            }
        }
        let callee = self.def_callee(d);
        let result = if stub.tasks {
            // `task.run(() { entry(args) })`: the entry runs as the root task.
            let main = self.provider_closure(&mut fx, &mut body, args, |_, _, out, args| {
                out.push(Stmt::TailCall {
                    callee,
                    args,
                    env: Env::Param,
                    site: None,
                });
            });
            let run = self.library_callee("i/task", "run");
            self.provider_call(&mut fx, &mut body, run, vec![main], env)
        } else {
            self.provider_call(&mut fx, &mut body, callee, args, env)
        };
        body.push(Stmt::Return(result));
        let inner = self.new_func();
        self.out.funcs[inner.0 as usize] = FuncKind::Code(Func {
            name: "entry stub".into(),
            arity: n,
            budget_frame: 1 + fx.locals.len() as u32,
            locals: fx.locals,
            body,
            frame: true,
        });
        // The request root ends after the whole entry, also when it ends
        // with an error. Traps retire through host teardown.
        let mut fx = Fx::new(d.module, false);
        let args: Vec<_> = (0..n)
            .map(|_| Atom::Local(fx.local(LocalType::Value)))
            .collect();
        let mut body = Vec::new();
        let call = self.provider_closure(&mut fx, &mut body, args, |_, _, out, args| {
            out.push(Stmt::TailCall {
                callee: Callee::Direct(inner),
                args,
                env: Env::Param,
                site: None,
            });
        });
        let result = self.provider_finish(
            &mut fx,
            &mut body,
            call,
            Env::Root,
            "request_entry_end",
            vec![],
        );
        body.push(Stmt::Return(result));
        let wrapper = self.new_func();
        self.out.funcs[wrapper.0 as usize] = FuncKind::Code(Func {
            name: "entry context".into(),
            arity: n,
            budget_frame: 1 + fx.locals.len() as u32,
            locals: fx.locals,
            body,
            frame: true,
        });
        Callee::Direct(wrapper)
    }

    /// `env` with the service `t` bound to `value`.
    fn stub_bind(&self, fx: &mut Fx, out: &mut Vec<Stmt>, env: Env, t: TypeId, value: Atom) -> Env {
        let dst = fx.local(LocalType::Env);
        out.push(Stmt::LetEnv {
            dst,
            env,
            service: self.services[&TypeRef::Custom(t)],
            value,
            in_place: false,
        });
        Env::Local(dst)
    }

    /// The value of a library factory of the stub, called in `env`.
    fn stub_factory(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        callee: Callee,
        args: Vec<Atom>,
        env: Env,
    ) -> Atom {
        let_value(fx, out, Value::Call { callee, args, env }, None)
    }

    fn provider_closure(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        captures: Vec<Atom>,
        build: impl FnOnce(&mut Self, &mut Fx, &mut Vec<Stmt>, Vec<Atom>),
    ) -> Atom {
        self.provider_closure_arity(fx, out, captures, 0, build)
    }

    fn provider_closure_arity(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        captures: Vec<Atom>,
        arity: u32,
        build: impl FnOnce(&mut Self, &mut Fx, &mut Vec<Stmt>, Vec<Atom>),
    ) -> Atom {
        let mut inner = Fx::new(fx.module, false);
        for _ in 0..arity {
            inner.local(LocalType::Value);
        }
        let mut body = Vec::new();
        let args = (0..captures.len())
            .map(|i| let_value(&mut inner, &mut body, Value::Capture(i as u32), None))
            .collect();
        build(self, &mut inner, &mut body, args);
        let func = self.new_func();
        self.out.funcs[func.0 as usize] = FuncKind::Code(Func {
            name: "provider continuation".into(),
            arity,
            budget_frame: 1 + inner.locals.len() as u32,
            locals: inner.locals,
            body,
            frame: true,
        });
        let_value(
            fx,
            out,
            Value::Closure {
                func,
                captures,
                patched: false,
            },
            None,
        )
    }

    /// Pin only this invocation's chosen services across adapter/host resumes.
    fn provider_call(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        callee: Callee,
        args: Vec<Atom>,
        env: Env,
    ) -> Atom {
        let body = self.provider_closure_arity(fx, out, args, 1, |_, _, out, args| {
            out.push(Stmt::TailCall {
                callee,
                args,
                env: Env::Param,
                site: None,
            });
        });
        let kept = let_value(
            fx,
            out,
            Value::OpEnv(Op::PinEnvironment, vec![body], env),
            None,
        );
        let_value(
            fx,
            out,
            Value::Call {
                callee: Callee::Value(kept),
                args: vec![Atom::Unit],
                env,
            },
            None,
        )
    }

    /// Administrative completion must not catch `$cancel`: an unwinding
    /// escaping the entry is a stop, whose original shadow frames are required.
    /// Normal returns and typed errors complete the scope; suspension does not.
    fn provider_finish(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        body: Atom,
        env: Env,
        operation: &'static str,
        args: Vec<Atom>,
    ) -> Atom {
        let catch = self.library_callee("i/fail", "catch");
        let result = let_value(
            fx,
            out,
            Value::Call {
                callee: catch,
                args: vec![body],
                env,
            },
            None,
        );
        let arity = args.len() as u32;
        let_value(
            fx,
            out,
            Value::Op(
                Op::Runtime {
                    name: operation,
                    arity,
                },
                args,
            ),
            None,
        );
        let callee = self.library_callee("i/fail", "from_result");
        let_value(
            fx,
            out,
            Value::Call {
                callee,
                args: vec![result],
                env: Env::Root,
            },
            None,
        )
    }

    /// Whether a host service is async in the module: on the WASI target
    /// every host function is a blocking call.
    fn host_is_async(&self, host: &hir::Host) -> bool {
        host.mode != HostMode::Now && self.options.target != ir::Target::Wasi
    }

    /// The body of an async entry point inside its root boundary: a
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

    /// A host service in the environment of an entry point: one
    /// import for each field, and a descriptor for the entry's provider leaf.
    fn host_service(&mut self, t: TypeId) {
        if self
            .out
            .host_services
            .iter()
            .any(|h| Some(&h.service) == self.services.get(&TypeRef::Custom(t)))
        {
            return;
        }
        let def = self.program.ty(t);
        let (Some(_), TypeBody::Custom(ctors)) = (&def.host, &def.body) else {
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
        let mut fields = Vec::new();
        for i in 0..c.fields.len() as u32 {
            match self.service_field_func(t, i) {
                Some(f) => fields.push(f),
                None => return,
            }
        }
        self.out.host_services.push(HostService {
            service,
            ctor,
            ty,
            fields,
        });
    }

    /// How the fields of the host service `t` are imported, from its mode
    /// (`@host` or `@import`): `"now"` is a call, `"later"` and `"request"`
    /// are starts, except on the WASI target, where every call blocks.
    fn service_import_kind(&self, t: TypeId, host: &hir::Host) -> ImportKind {
        let def = self.program.ty(t);
        if self.host_is_async(host) {
            ImportKind::AsyncService
        } else if self.options.target == ir::Target::Wasi && is_standard_service(self.program, t) {
            ImportKind::WasiStdout
        } else if self.options.target == ir::Target::Wasi
            && self.graph.packages[self.program.module(def.module).package].name == "i"
            && matches!(
                host.namespace.as_str(),
                "i:clock/v1"
                    | "i:stdio/v1"
                    | "i:fs/v1"
                    | "i:entropy/v1"
                    | "i:arguments/v1"
                    | "i:environment/v1"
            )
        {
            ImportKind::WasiAdapter
        } else {
            ImportKind::Service
        }
    }

    /// The import function of field `index` of the host service `t`: one
    /// top-level value per field, shared by the entry's provider leaf, the
    /// accessor `make_<type>`, and `hostcall.call`, so they have one
    /// identity and one import.
    fn service_field_func(&mut self, t: TypeId, index: u32) -> Option<FuncId> {
        if let Some(&f) = self.service_fields.get(&(t, index)) {
            return f;
        }
        let f = self.make_service_field_func(t, index);
        self.service_fields.insert((t, index), f);
        f
    }

    fn make_service_field_func(&mut self, t: TypeId, index: u32) -> Option<FuncId> {
        let program = self.program;
        let def = program.ty(t);
        let (Some(host), TypeBody::Custom(ctors)) = (&def.host, &def.body) else {
            return None;
        };
        let [c] = ctors.as_slice() else {
            return None;
        };
        let field = c.fields.get(index as usize)?;
        let (Some(label), Some(hf)) = (&field.label, program.host_fns.get(&(t, index))) else {
            return None;
        };
        let kind = self.service_import_kind(t, host);
        let later = host.mode != HostMode::Now;
        let request = host.mode == HostMode::Request;
        match self.import(
            &host.namespace,
            &label.name,
            host_abi(hf),
            kind,
            (def.module, label.span),
            &format!("{}.{}", def.name, label.name),
        ) {
            Ok(import) => {
                let x = &mut self.out.imports[import as usize];
                x.request = request;
                x.request_result =
                    hf.request_result || (later && !request && !hf.errors.is_empty());
                x.awaited = later;
                x.answer_result = def.import.is_some() && returns_result(program, &field.ty);
                let f = self.import_func(import, hf.result);
                self.constants.insert(f);
                Some(f)
            }
            Err((module, span, message)) => {
                self.error(module, span, message);
                None
            }
        }
    }

    /// The import behind the function of field `index` of `t`, when it is
    /// a start that `hostcall.call` may leave unawaited.
    fn service_field_start(&mut self, t: TypeId, index: u32) -> Option<(FuncId, FuncId)> {
        let field = self.service_field_func(t, index)?;
        let (&(import, result), _) = self.import_funcs.iter().find(|(_, f)| **f == field)?;
        let x = &self.out.imports[import as usize];
        // A field with a `Fail` need answers a `Result` that its function
        // unwraps; a `Call` of it would hold the `Result`, so it is awaited
        // now. A field written with its `Result` answer starts.
        if x.kind != ImportKind::AsyncService || x.request || (x.request_result && !x.answer_result)
        {
            return None;
        }
        Some((field, self.start_func(import, result)))
    }

    /// A function that starts an async import and gives its request cell,
    /// registered with the `Owner` of its environment.
    fn start_func(&mut self, import: ImportId, result: AbiType) -> FuncId {
        if let Some(&f) = self.start_funcs.get(&(import, result)) {
            return f;
        }
        let x = &self.out.imports[import as usize];
        let n = x.abi.params.len() as u32;
        let name = format!("start {} {}", x.module, x.name);
        let body = vec![
            Stmt::Let {
                dst: n,
                value: Value::HostStart {
                    import,
                    args: (0..n).map(Atom::Local).collect(),
                    result,
                },
                site: None,
            },
            Stmt::Return(Atom::Local(n)),
        ];
        let f = self.new_func();
        self.out.funcs[f.0 as usize] = FuncKind::Code(Func {
            name,
            arity: n,
            budget_frame: n + 1,
            locals: vec![LocalType::Value; n as usize + 1],
            body,
            frame: true,
        });
        self.start_funcs.insert((import, result), f);
        f
    }

    /// `make_<type>` of an `@import` service type: the service of the
    /// caller's environment when it has one (a provided value, a fake, or
    /// the platform's), else the record of the type's import functions.
    /// Every record has the same field values.
    fn accessor_func(&mut self, t: TypeId) -> FuncId {
        if let Some(&f) = self.accessor_funcs.get(&t) {
            return f;
        }
        let f = self.new_func();
        self.accessor_funcs.insert(t, f);
        let def = self.program.ty(t);
        let mut fx = Fx::new(def.module, false);
        let mut body = Vec::new();
        if let Some(&slot) = self.services.get(&TypeRef::Custom(t)) {
            let raw = let_value(
                &mut fx,
                &mut body,
                Value::ServiceRaw(Env::Param, slot),
                None,
            );
            let given = fx.local(LocalType::Value);
            body.push(Stmt::If {
                cond: Cond::Not(Box::new(Cond::Tag(raw, 0))),
                then: vec![
                    Stmt::Let {
                        dst: given,
                        value: Value::Service(Env::Param, slot),
                        site: None,
                    },
                    Stmt::Return(Atom::Local(given)),
                ],
                otherwise: vec![],
            });
        }
        let n = match &def.body {
            TypeBody::Custom(ctors) if ctors.len() == 1 => ctors[0].fields.len() as u32,
            _ => 0,
        };
        let mut fields = Vec::new();
        for i in 0..n {
            match self.service_field_func(t, i) {
                Some(f) => fields.push(Atom::Func(f)),
                None => fields.push(Atom::Unit),
            }
        }
        let value = match self.ctors.get(&CtorRef { ty: t, index: 0 }).map(|c| c.repr) {
            Some(CtorRepr::Record { ctor, ty }) if fields.is_empty() => Atom::Ctor { ctor, ty },
            Some(CtorRepr::Record { ctor, ty }) => {
                let_value(&mut fx, &mut body, Value::Record { ctor, ty, fields }, None)
            }
            _ => {
                self.error(
                    def.module,
                    def.span,
                    format!(
                        "`{}` is not a record service, so it has no accessor",
                        def.name
                    ),
                );
                Atom::Unit
            }
        };
        body.push(Stmt::Return(value));
        self.out.funcs[f.0 as usize] = FuncKind::Code(Func {
            name: format!(
                "{}.{}",
                self.program.module(def.module).path,
                accessor_name(&def.name)
            ),
            arity: 0,
            budget_frame: 1 + fx.locals.len() as u32,
            locals: fx.locals,
            body,
            frame: true,
        });
        f
    }

    /// The function of an external function outside the library,
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
        let (DefKind::External { .. } | DefKind::Runtime { .. }, Some((module, function))) =
            (&def.kind, def.kind.foreign())
        else {
            return None;
        };
        let Some(hf) = self.program.external_fns.get(&d) else {
            self.error(
                d.module,
                def.span,
                "an external function needs a fully written function type with positional parameters. Parameters and results may use only `()`, `Bool`, `Int`, `Float`, `String`, `BitArray`, or opaque types. Its only allowed need is `Fail(E)`, where `E` contains only the six built-in types",
            );
            return None;
        };
        if self.program.js_values.contains(&d) {
            self.error(
                d.module,
                def.span,
                "`@import(\"js\", …)` value declarations are not supported yet",
            );
            return None;
        }
        let site = (d.module, def.span);
        // `@import("ns", "label", "later")` starts its call and waits for
        // the answer; `"now"` (and `@external`) calls it.
        let later = self.program.import_modes.get(&d) == Some(&ImportMode::Later)
            && self.options.target != ir::Target::Wasi;
        let kind = if later {
            ImportKind::AsyncService
        } else {
            ImportKind::External
        };
        match self.import(module, function, host_abi(hf), kind, site, &def.name) {
            Ok(import) => {
                if later {
                    let x = &mut self.out.imports[import as usize];
                    x.awaited = true;
                    x.request_result = hf.request_result || !hf.errors.is_empty();
                }
                // Each declaration is one value, so two declarations
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

    /// The import with this module and name, declared at `site`.
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
        if !matches!(kind, ImportKind::WasiStdout | ImportKind::WasiAdapter) {
            let source_module = self.program.module(site.0);
            let package = &self.graph.packages[source_module.package];
            use ilang_package::lock::{FileKind, PackageFile, hash};
            let source_id = source_module.source;
            let module_digest = match self.module_hashes.get(&source_id) {
                Some(digest) => digest.clone(),
                None => {
                    let file = PackageFile {
                        path: source_module.path.clone(),
                        kind: FileKind::Regular,
                        bytes: self.graph.sources.get(source_id).text.as_bytes().to_vec(),
                    };
                    let digest = hash(&[file]);
                    self.module_hashes.insert(source_id, digest.clone());
                    digest
                }
            };
            let package_hash = if self.graph.is_language(source_module.package) {
                self.graph.language_hash()
            } else {
                match &package.location {
                    Some(location) => location.hash.clone(),
                    None => self
                        .package_hashes
                        .entry(source_module.package)
                        .or_insert_with(|| {
                            let mut files: Vec<_> = package
                                .modules
                                .iter()
                                .map(|m| PackageFile {
                                    path: m.path.clone(),
                                    kind: FileKind::Regular,
                                    bytes: self
                                        .graph
                                        .sources
                                        .get(m.source)
                                        .text
                                        .as_bytes()
                                        .to_vec(),
                                })
                                .collect();
                            if let Some(source) = package.manifest_source {
                                files.push(PackageFile {
                                    path: "i.toml".into(),
                                    kind: FileKind::Regular,
                                    bytes: self.graph.sources.get(source).text.as_bytes().to_vec(),
                                });
                            }
                            hash(&files)
                        })
                        .clone(),
                }
            };
            let binding = serde_json::json!({
                "package": package_hash,
                "module": source_module.path,
                "module_digest": module_digest,
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
            // Wire signatures are coalesced. The calling declaration keeps
            // its own nominal result decoder in HostCall or HostStart.
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
            request: false,
            request_result: false,
            awaited: false,
            answer_result: false,
        });
        self.import_sites.push(site);
        Ok(self.out.imports.len() as ImportId - 1)
    }

    /// The function that calls an import: its parameters go to the host
    /// encoded in wire order, and its value or error comes back. An async
    /// import waits for the host at the root boundary.
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
    fn library_callee(&mut self, path: &str, name: &str) -> Callee {
        let (module, m) = self
            .program
            .modules
            .iter()
            .enumerate()
            .find(|(_, m)| m.path == path)
            .expect("runtime module is registered");
        let index = m
            .defs
            .iter()
            .position(|d| d.name == name)
            .expect("runtime binding exists");
        self.def_callee(DefId {
            module: ModuleId(module as u32),
            index: index as u32,
        })
    }

    fn make_import_func(&mut self, import: ImportId, abi_result: AbiType) -> FuncId {
        let x = self.out.imports[import as usize].clone();
        let n = x.abi.params.len() as u32;
        let args: Vec<Atom> = (0..n).map(Atom::Local).collect();
        let mut locals = vec![LocalType::Value; n as usize];
        let mut body = Vec::new();
        let mut fresh = || {
            let id = locals.len() as u32;
            locals.push(LocalType::Value);
            id
        };
        let asynchronous = x.kind == ImportKind::AsyncService;
        if x.awaited && !asynchronous {
            let owner = fresh();
            let checked = fresh();
            body.push(Stmt::Let {
                dst: owner,
                value: Value::Service(
                    Env::Param,
                    self.out.request_owner.expect("request Owner is registered"),
                ),
                site: None,
            });
            body.push(Stmt::Let {
                dst: checked,
                value: Value::Op(
                    Op::Runtime {
                        name: "request_check_owner",
                        arity: 1,
                    },
                    vec![Atom::Local(owner)],
                ),
                site: None,
            });
        }
        let value = fresh();
        body.push(Stmt::Let {
            dst: value,
            value: if asynchronous {
                Value::HostStart {
                    import,
                    args,
                    result: abi_result,
                }
            } else {
                Value::HostCall {
                    import,
                    args,
                    result: abi_result,
                }
            },
            site: None,
        });
        let mut result = value;
        if x.awaited && !asynchronous {
            result = fresh();
            body.push(Stmt::Let {
                dst: result,
                value: Value::Op(
                    Op::Runtime {
                        name: "request_ready",
                        arity: 1,
                    },
                    vec![Atom::Local(value)],
                ),
                site: None,
            });
        }
        if x.awaited && !x.request {
            let answer = fresh();
            let awaiter = self.library_callee("i/request", "await_owned");
            body.push(Stmt::Let {
                dst: answer,
                value: Value::Call {
                    callee: awaiter,
                    args: vec![Atom::Local(result)],
                    env: Env::Param,
                },
                site: None,
            });
            result = answer;
            if x.request_result && !x.answer_result {
                let unwrapped = fresh();
                let from_result = self.library_callee("i/fail", "from_result");
                body.push(Stmt::Let {
                    dst: unwrapped,
                    value: Value::Call {
                        callee: from_result,
                        args: vec![Atom::Local(result)],
                        env: Env::Param,
                    },
                    site: None,
                });
                result = unwrapped;
            }
        }
        body.push(Stmt::Return(Atom::Local(result)));
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
        match self.program.def(d).kind {
            DefKind::External { ref module, .. } | DefKind::Runtime { ref module, .. }
                if self.is_library(d.module) && module.starts_with("ilang:") =>
            {
                let (op, arity) = self.external_op(d).expect("checked library operation");
                Callee::Direct(self.op_func(op, arity))
            }
            DefKind::External { .. } | DefKind::Runtime { .. } if self.is_library(d.module) => {
                Callee::Direct(self.external_func(d).expect("checked library import"))
            }
            DefKind::Runtime { .. } => unreachable!("`@runtime` is only in the library"),
            DefKind::ServiceAccessor { ty } => Callee::Direct(self.accessor_func(ty)),
            DefKind::External { .. } => Callee::Direct(
                self.external_func(d)
                    .expect("entry imports were checked before plan application"),
            ),
            DefKind::Binding { .. } if self.known.contains_key(&d) => {
                Callee::Direct(self.def_func(d))
            }
            DefKind::Binding { .. } => Callee::Value(Atom::Global(self.global(d))),
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
        let (body, arity) = match info.repr {
            CtorRepr::Bool(b) => (CtorBody::Bool(b), 0),
            CtorRepr::Record { ctor, ty } => {
                (CtorBody::Record { ctor, ty }, info.labels.len() as u32)
            }
        };
        let f = self.new_func();
        self.out.funcs[f.0 as usize] = FuncKind::Ctor { body, arity };
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
        id: FuncId,
    ) -> Func {
        self.function_code(module, f, captures, name, id, false, false)
    }

    /// As `function`; when `lifted`, the captures are parameters after the
    /// function's own, and it has no closure; when `light` too, it is a
    /// light boundary's body.
    #[allow(clippy::too_many_arguments)]
    fn function_code(
        &mut self,
        module: ModuleId,
        f: &Function,
        captures: &[VarId],
        name: String,
        id: FuncId,
        lifted: bool,
        light: bool,
    ) -> Func {
        let mut fx = Fx::new(module, self.is_library(module));
        let mut out = Vec::new();
        let own = f
            .params
            .iter()
            .filter(|p| matches!(p, Param::Pattern { .. }))
            .count() as u32;
        let arity = if lifted {
            own + captures.len() as u32
        } else {
            own
        };
        for _ in 0..arity {
            fx.local(LocalType::Value);
        }
        if light {
            fx.light = light_names(f);
        }
        let mut start = light;
        for (i, &v) in captures.iter().enumerate() {
            let l = fx.var(v);
            let value = if lifted {
                Value::Atom(Atom::Local(own + i as u32))
            } else {
                Value::Capture(i as u32)
            };
            out.push(Stmt::Let {
                dst: l,
                value,
                site: None,
            });
        }
        // Environment parameters are not parameters of the type:
        // the parameters of the code are the patterns, in order.
        let mut next = 0;
        for p in &f.params {
            match p {
                Param::Pattern { pattern, .. } => {
                    let v = Atom::Local(next);
                    if let Some(ilang_check::types::NumericFact::FiniteF64) =
                        self.typed.numeric_params.get(&(module, pattern.span))
                    {
                        fx.numeric
                            .insert(next, ilang_check::types::NumericFact::FiniteF64);
                    }
                    next += 1;
                    self.bind_checked(&mut fx, &mut out, pattern, v, pattern.span.start);
                }
                Param::Env {
                    rest, var, name, ..
                } => {
                    let value = if *rest {
                        let mut excluded: Vec<u32> = self
                            .typed
                            .env_captures
                            .get(&(module, name.span))
                            .into_iter()
                            .flatten()
                            .map(|t| self.services[t])
                            .collect();
                        excluded.sort_unstable();
                        Value::Environment(Env::Param, excluded)
                    } else {
                        let service = self.service(module, name.span);
                        let raw = self.lazy_candidates.contains(&service)
                            && self.only_field_calls(module, &f.body, *var);
                        if raw {
                            fx.lazy_params.insert(*var);
                        }
                        if raw {
                            Value::ServiceRaw(Env::Param, service)
                        } else {
                            Value::Service(Env::Param, service)
                        }
                    };
                    let l = fx.var(*var);
                    out.push(Stmt::Let {
                        dst: l,
                        value,
                        site: None,
                    });
                }
            }
        }
        // Binding a plain name keeps nothing of the environment.
        start &= fx.light.is_some();
        fx.light_start = start;
        self.block(&mut fx, &mut out, &f.body, true);
        let pending = f
            .params
            .iter()
            .filter_map(|p| match p {
                Param::Pattern { pattern, .. } => Some(pattern_values(pattern)),
                Param::Env { .. } => None,
            })
            .fold(block_values(&f.body), usize::max);
        self.out.numeric_locals.insert(id, fx.numeric);
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

    /// The site of the expression that starts at `offset`. Library
    /// code is not program source, so it has none.
    fn site(&mut self, fx: &Fx, offset: u32) -> Option<Site> {
        if fx.library {
            return None;
        }
        let origin = self
            .program
            .origin(fx.module, Span::new(offset, offset + 1));
        let (source, offset) = origin
            .as_ref()
            .map_or((self.program.module(fx.module).source, offset), |origin| {
                (origin.source, origin.span.start)
            });
        let module_id = self
            .program
            .modules
            .iter()
            .position(|m| m.source == source)
            .map(|index| ModuleId(index as u32))
            .expect("site source belongs to a module");
        let module = self.program.module(module_id);
        let index = *self.module_index.entry(module_id).or_insert_with(|| {
            let index = self.out.site_modules.len() as u32;
            self.out.site_modules.push(format!(
                "{}{}",
                self.graph.packages[module.package].position_prefix(),
                module.path
            ));
            index
        });
        let lines = self
            .lines
            .entry(module_id)
            .or_insert_with(|| LineIndex::new(&self.graph.sources.get(source).text));
        let (line, column) = lines.line_col(offset);
        let pos = SitePos {
            module: index,
            line,
            column,
        };
        let note = origin
            .and_then(|origin| origin.template)
            .map(|(source, span)| {
                let module = self
                    .program
                    .modules
                    .iter()
                    .find(|m| m.source == source)
                    .expect("template source belongs to a module");
                let (line, column) =
                    LineIndex::new(&self.graph.sources.get(source).text).line_col(span.start);
                format!("\nexpanded from {}:{line}:{column}", module.path)
            });
        Some(*self.site_map.entry(pos).or_insert_with(|| {
            self.out.sites.push(pos);
            self.out.site_notes.push(note);
            self.out.sites.len() as Site - 1
        }))
    }

    // ---- Names ----

    /// The operation of an external function of the library.
    fn external_op(&self, d: DefId) -> Result<(Op, u32), String> {
        let kind = &self.program.def(d).kind;
        let (Some((module, function)), Some((ty, _))) = (kind.foreign(), kind.declared()) else {
            unreachable!()
        };
        let arity = match &ty.kind {
            TypeKind::Fn { params, .. } => params.len() as u32,
            _ => 0,
        };
        match module {
            "ilang:prelude" if function == "panic" => Ok((Op::Panic, arity)),
            // The private primitives of `lib/i/bits.i`.
            "ilang:bits" => Ok((
                Op::Runtime {
                    name: match function {
                        "read_unsigned" => "bits_read_unsigned",
                        "from_unsigned" => "bits_from_unsigned",
                        _ => return Err(format!("`bits.{function}` is not known")),
                    },
                    arity,
                },
                arity,
            )),
            // The private primitives of `lib/i/big.i`.
            "ilang:big" => Ok((
                Op::Runtime {
                    name: match function {
                        "sign" => "big_sign",
                        "digits" => "big_digits",
                        "from_digits" => "big_from_digits",
                        "size" => "big_size",
                        "limb" => "big_limb",
                        "from_top" => "big_from_top",
                        "builder" => "big_builder",
                        "put" => "big_put",
                        "finish" => "big_finish",
                        "multiply_low" => "big_multiply_low",
                        "multiply_high" => "big_multiply_high",
                        "divide_digit" => "big_divide_digit",
                        _ => return Err(format!("`big.{function}` is not known")),
                    },
                    arity,
                },
                arity,
            )),
            "ilang:core" => match CORE.iter().position(|n| *n == function) {
                Some(i) => Ok((
                    Op::Runtime {
                        name: CORE_FUNCS[i],
                        arity,
                    },
                    arity,
                )),
                None => Err(format!("`core.{function}` is not known")),
            },
            "ilang:request" => {
                let op = match function {
                    "wait_any" => Op::WaitRequests,
                    "poll" => Op::PollRequests,
                    _ => Op::Runtime {
                        name: match function {
                            "open_root" => "request_open_root",
                            "close_root" => "request_close_root",
                            "open_owner" => "request_open_owner",
                            "close_owner" => "request_close_owner",
                            "count" => "request_count",
                            "count_root" => "request_count_root",
                            "cancel_all" => "request_cancel_all",
                            "cancel_all_root" => "request_cancel_all_root",
                            "observe_owner" => "request_observe_owner",
                            "observe_root" => "request_observe_root",
                            "observe_change" => "request_observe_change",
                            "has_work" => "request_has_work",
                            "ready" => "request_ready",
                            "ticket" => "request_ticket",
                            "state" => "request_state",
                            "take" => "request_take",
                            "settled" => "request_settled",
                            "cancel" => "request_cancel",
                            "entry_root" => "request_entry_root",
                            "entry_end" => "request_entry_end",
                            _ => return Err(format!("`request.{function}` is not known")),
                        },
                        arity,
                    },
                };
                Ok((op, arity))
            }
            "ilang:key" => match function {
                "new" => Ok((
                    Op::Runtime {
                        name: "key_new",
                        arity,
                    },
                    arity,
                )),
                "wrap" => Ok((
                    Op::Runtime {
                        name: "key_wrap",
                        arity,
                    },
                    arity,
                )),
                "read" => Ok((
                    Op::Runtime {
                        name: "key_read",
                        arity,
                    },
                    arity,
                )),
                _ => Err(format!("`key.{function}` is not known")),
            },
            "ilang:worker" => {
                let name = match function {
                    "enabled" => "worker_enabled",
                    "publish" => "worker_publish",
                    "claim" => "worker_claim",
                    "adopt" => "worker_adopt",
                    "placeholder" => "worker_placeholder",
                    "away" => "worker_away",
                    "send" => "worker_send",
                    "inbox" => "worker_inbox",
                    "message" => "worker_message",
                    "pending" => "worker_pending",
                    "wait" => "worker_wait",
                    "share_put" => "worker_share_put",
                    "share_get" => "worker_share_get",
                    "share_drop" => "worker_share_drop",
                    _ => return Err(format!("`worker.{function}` is not known")),
                };
                Ok((Op::Runtime { name, arity }, arity))
            }
            // `i/hostcall`: `call` and `call_in` are lowered at their call
            // (`hostcall_call`); the others are request table operations.
            "ilang:hostcall" => match function {
                "group_live" => Ok((
                    Op::Runtime {
                        name: "request_group_live",
                        arity,
                    },
                    arity,
                )),
                "root_live" => Ok((
                    Op::Runtime {
                        name: "request_root_live",
                        arity,
                    },
                    arity,
                )),
                "adopt" => Ok((
                    Op::Runtime {
                        name: "request_adopt",
                        arity,
                    },
                    arity,
                )),
                "call" | "call_in" => Err(format!(
                    "`hostcall.{function}` is called directly with its operation; it is not a value"
                )),
                _ => Err(format!("`hostcall.{function}` is not known")),
            },
            "ilang:resource" if function == "ensure" => Ok((Op::Ensure, arity)),
            "ilang:control" => match function {
                "boundary" => Ok((Op::Boundary, arity)),
                "suspend" => Ok((Op::Suspend, arity)),
                "escape" => Ok((Op::Escape, arity)),
                "resume" => Ok((Op::Resume, arity)),
                "cancel" => Ok((Op::Cancel, arity)),
                "finalizing" => Ok((
                    Op::Runtime {
                        name: "control_finalizing",
                        arity,
                    },
                    arity,
                )),
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
                DefKind::External { ref module, .. } | DefKind::Runtime { ref module, .. }
                    if !self.is_library(d.module) || !module.starts_with("ilang:") =>
                {
                    match self.external_func(d) {
                        Some(f) => {
                            self.constants.insert(f);
                            Atom::Func(f)
                        }
                        None => Atom::Unit,
                    }
                }
                DefKind::External { .. } | DefKind::Runtime { .. } => match self.external_op(d) {
                    Ok((op, arity)) => Atom::Func(self.op_func(op, arity)),
                    Err(message) => {
                        self.error(fx.module, e.span, message);
                        Atom::Unit
                    }
                },
                DefKind::ServiceAccessor { ty } => {
                    let f = self.accessor_func(ty);
                    self.constants.insert(f);
                    Atom::Func(f)
                }
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
            // A constructor used as a value is a function, even one
            // without fields; only a call makes the value.
            ValueRef::Ctor(c) => Atom::Func(self.ctor_func(c)),
        }
    }

    // ---- Expressions ----

    fn expr(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, e: &Expr) -> Atom {
        let first = fx.locals.len() as Local;
        let atom = self
            .lower(fx, out, e, false)
            .expect("a value that is not in tail position");
        if let Atom::Local(local) = atom
            && local >= first
            && let Some(fact) = self.typed.numeric.get(&(fx.module, e.span))
        {
            fx.numeric.insert(local, *fact);
        }
        atom
    }

    /// Lowers `e`. In tail position, the statements end the function and
    /// there is no value.
    fn lower(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, e: &Expr, tail: bool) -> Option<Atom> {
        let a = match &e.kind {
            ExprKind::Int(n) => Atom::Int(BigInt::from(n.clone())),
            ExprKind::Float(x) => Atom::Float(*x),
            ExprKind::String(s) => Atom::String(s.clone()),
            ExprKind::Name(r) => {
                // A chain name that an expected type narrowed to one shape
                // group stands for that group's function.
                let r = self
                    .typed
                    .selected_names
                    .get(&(fx.module, e.span))
                    .copied()
                    .unwrap_or(*r);
                self.name(fx, e, r)
            }
            // `()`, empty data, is the null reference.
            ExprKind::Tuple(items) if items.is_empty() => Atom::Unit,
            ExprKind::Tuple(items) => {
                let parts = items.iter().map(|x| self.expr(fx, out, x)).collect();
                let_value(fx, out, Value::Tuple(parts), None)
            }
            ExprKind::Multi(groups) => match self.typed.selected_groups.get(&(fx.module, e.span)) {
                // An expected type chose one shape group: the value is that
                // group's function.
                Some(i) => self.expr(fx, out, &groups[*i as usize].value),
                // A multi-shape value: the closure of
                // each group with the run-time id of its shape.
                None => {
                    let parts = groups
                        .iter()
                        .map(|g| {
                            let set = if g.shape.has_labels() {
                                self.shape_set(g.shape)
                            } else {
                                0
                            };
                            let id = crate::ir::shape_id(g.shape.positions(), set);
                            let value = self.expr(fx, out, &g.value);
                            if set != 0 {
                                self.label_shape(out, &value, id);
                            }
                            (id, value)
                        })
                        .collect();
                    let_value(fx, out, Value::Multi(parts), None)
                }
            },
            ExprKind::Record { base, fields } => {
                // The items are worked out in written order,
                // the spread base first; the data has its parts in canonical
                // order.
                let (shape, base_shape) = self.typed.data_shapes[&(fx.module, e.span)];
                let base = base.as_ref().map(|b| self.expr(fx, out, b));
                let given: Vec<(String, Atom)> = fields
                    .iter()
                    .map(|(label, value)| (label.name.clone(), self.expr(fx, out, value)))
                    .collect();
                let mut parts = Vec::with_capacity(shape.len());
                for key in shape.keys() {
                    let found = key
                        .as_ref()
                        .and_then(|l| given.iter().find(|(g, _)| g == l))
                        .map(|(_, a)| a.clone());
                    parts.push(match found {
                        Some(a) => a,
                        None => {
                            let base_shape = base_shape.expect("a key from the spread");
                            let at = match &key {
                                Some(l) => base_shape.label_index(l).expect("a spread label"),
                                None => parts.len().min(base_shape.positions() as usize),
                            };
                            let b = base.clone().expect("a spread base");
                            let_value(fx, out, Value::Field(b, at as u32), None)
                        }
                    });
                }
                if shape.is_empty() {
                    // `()`, empty data, is the null reference.
                    Atom::Unit
                } else {
                    let value = if shape.has_labels() {
                        Value::Data(self.shape_set(shape), parts)
                    } else {
                        Value::Tuple(parts)
                    };
                    let_value(fx, out, value, None)
                }
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
            ExprKind::Call { callee, args } if args.iter().any(|a| a.label.is_some()) => {
                return self.labelled_call(fx, out, e.span, callee, None, args, tail);
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
                Atom::Unit
            }
        };
        finish(out, tail, a)
    }

    /// A block. An environment statement changes the environment
    /// for the rest of its block only.
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
        let start = std::mem::replace(&mut fx.light_start, false);
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                hir::Stmt::Expr(e) if i == last => return self.lower(fx, out, e, tail),
                hir::Stmt::Expr(e) => {
                    self.expr(fx, out, e);
                }
                hir::Stmt::Bind(binding) => self.binding(fx, out, binding),
                hir::Stmt::Env { value, span, .. } => {
                    if self.lazy_provide(fx, out, value, *span, start && i == 0) {
                        continue;
                    }
                    // The value uses the environment from outside the
                    // statement.
                    let v = self.expr(fx, out, value);
                    if let Some(supply) = self.typed.env_supplies.get(&(fx.module, *span)) {
                        let mut entries: Vec<(u32, Value)> = match supply {
                            ilang_check::types::EnvSupply::Row(keep) => self
                                .services
                                .iter()
                                .filter(|(ty, _)| !keep.contains(ty))
                                .map(|(_, id)| (*id, Value::CapturedService(v.clone(), *id)))
                                .collect(),
                            ilang_check::types::EnvSupply::Data(keys) => keys
                                .iter()
                                .enumerate()
                                .map(|(i, ty)| {
                                    (self.services[ty], Value::Field(v.clone(), i as u32))
                                })
                                .collect(),
                        };
                        entries.sort_by_key(|(id, _)| *id);
                        for (service, value) in entries {
                            let value = let_value(fx, out, value, None);
                            let dst = fx.local(LocalType::Env);
                            out.push(Stmt::LetEnv {
                                dst,
                                env: fx.env,
                                service,
                                value,
                                in_place: false,
                            });
                            fx.env = Env::Local(dst);
                        }
                        continue;
                    }
                    let service = self.service(fx.module, *span);
                    let dst = fx.local(LocalType::Env);
                    out.push(Stmt::LetEnv {
                        dst,
                        env: fx.env,
                        service,
                        value: v,
                        in_place: false,
                    });
                    fx.env = Env::Local(dst);
                }
            }
        }
        // A block always ends with an expression.
        finish(out, tail, Atom::Unit)
    }

    /// A binding in a block. The names of a direct value may be used
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

    /// `control.boundary(f)` where `f` is a function literal whose parameter
    /// is used only as the first argument of `control.escape`, directly or
    /// in function literals inside it. The decision uses only the program's
    /// shape. Its handle
    /// is never suspended on, stored, or given elsewhere, so the boundary
    /// needs no continuation of its own; `f` is lifted to code that takes
    /// its captures as parameters, so no closure is made for it. `None` when
    /// the call is not one.
    #[allow(clippy::too_many_arguments)]
    fn light_boundary(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        span: Span,
        callee: &Expr,
        first: Option<&Expr>,
        args: &[&Expr],
        tail: bool,
    ) -> Option<Option<Atom>> {
        if self.library_op(callee) != Some(Op::Boundary) {
            return None;
        }
        let arg: &Expr = match (first, args) {
            (Some(a), []) => a,
            (None, [a]) => a,
            _ => return None,
        };
        let ExprKind::Fn(f) = &ungroup(arg).kind else {
            return None;
        };
        let mut patterns = f.params.iter().filter_map(|p| match p {
            Param::Pattern { pattern, .. } => Some(pattern),
            Param::Env { .. } => None,
        });
        let (Some(param), None) = (patterns.next(), patterns.next()) else {
            return None;
        };
        // The names the parameter binds, and every use of them.
        let mut names = Walk::default();
        names.pattern(param);
        let mut body = Walk::default();
        body.block(&f.body);
        for &v in &names.bound {
            let all = body.uses.get(&v).copied().unwrap_or(0);
            let escapes = body
                .first_args
                .iter()
                .filter(|&&(d, x, call)| {
                    x == v
                        && self.def_op(d) == Some(Op::Escape)
                        && self
                            .typed
                            .args
                            .get(&(fx.module, call))
                            .is_none_or(|order| order.first() == Some(&0))
                })
                .count() as u32;
            if all != escapes {
                return None;
            }
        }
        let captures = captures_of(f);
        if fx
            .pending
            .as_ref()
            .is_some_and(|p| captures.iter().any(|v| p.contains(v)))
        {
            return None;
        }
        let id = self.new_func();
        let name = format!("{}:light{}", self.program.module(fx.module).path, id.0);
        let func = self.function_code(fx.module, f, &captures, name, id, true, true);
        self.out.funcs[id.0 as usize] = FuncKind::Code(func);
        // The boundary is the environment of its body (`$light_new`).
        let b = let_value(
            fx,
            out,
            Value::OpEnv(
                Op::Runtime {
                    name: "light_new",
                    arity: 0,
                },
                vec![],
                fx.env,
            ),
            None,
        );
        let mut atoms = vec![b];
        atoms.extend(captures.iter().map(|v| Atom::Local(fx.var(*v))));
        let site = self.site(fx, span.start);
        let v = let_value(
            fx,
            out,
            Value::Light {
                func: id,
                args: atoms,
                env: fx.env,
            },
            site,
        );
        Some(finish(out, tail, v))
    }

    /// An environment statement in the body of a light boundary whose value
    /// is a constructor applied to function literals and names that use
    /// nothing but the boundary's handle: making the value has no effect and
    /// depends only on the handle, so the handle stands for it in the
    /// environment, and the value is made at the first read of the service
    /// (`$service_force`), once. Only the first such statement of a body.
    /// False when the statement is not one.
    fn lazy_provide(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        value: &Expr,
        span: Span,
        in_place: bool,
    ) -> bool {
        let Some(names) = fx.light.as_ref() else {
            return false;
        };
        let e = ungroup(value);
        let ExprKind::Call { callee, args } = &e.kind else {
            return false;
        };
        let ExprKind::Name(ValueRef::Ctor(c)) = &ungroup(callee).kind else {
            return false;
        };
        if !matches!(
            self.ctors.get(c).map(|i| &i.repr),
            Some(CtorRepr::Record { .. })
        ) {
            return false;
        }
        let simple = args.iter().all(|a| match &ungroup(&a.value).kind {
            ExprKind::Fn(f) => captures_of(f).iter().all(|v| names.contains(v)),
            ExprKind::Name(ValueRef::Local(v)) => names.contains(v),
            _ => false,
        });
        if !simple {
            return false;
        }
        let names = fx.light.take().expect("checked above");
        // The function that makes the value from the handle.
        let mut inner = Fx::new(fx.module, fx.library);
        inner.local(LocalType::Value);
        for v in &names {
            inner.vars.insert(*v, 0);
        }
        let mut body = Vec::new();
        let made = self.expr(&mut inner, &mut body, value);
        body.push(Stmt::Return(made));
        let builder = self.new_func();
        self.out.funcs[builder.0 as usize] = FuncKind::Code(Func {
            name: format!("{}:lazy{}", self.program.module(fx.module).path, builder.0),
            arity: 1,
            budget_frame: 1 + inner.locals.len() as u32,
            locals: inner.locals,
            body,
            frame: true,
        });
        self.constants.insert(builder);
        // The code of each field that is a function literal, lifted so
        // that the handle is its last parameter: a call of that field of
        // the service (`Callee::Lazy`) runs it without making the value.
        if let [handle] = names.iter().copied().collect::<Vec<_>>()[..] {
            let written: Vec<Option<FuncId>> = args
                .iter()
                .map(|a| match &ungroup(&a.value).kind {
                    ExprKind::Fn(f) => {
                        let id = self.new_func();
                        let name = format!("{}:field{}", self.program.module(fx.module).path, id.0);
                        let func =
                            self.function_code(fx.module, f, &[handle], name, id, true, false);
                        self.out.funcs[id.0 as usize] = FuncKind::Code(func);
                        Some(id)
                    }
                    _ => None,
                })
                .collect();
            let fields: Vec<Option<FuncId>> = match self.typed.args.get(&(fx.module, e.span)) {
                Some(order) if order.len() == written.len() => {
                    order.iter().map(|&i| written[i as usize]).collect()
                }
                _ => written,
            };
            if fields.iter().any(Option::is_some) {
                self.out.lazy_fields.insert(builder, fields);
            }
        }
        let_value(
            fx,
            out,
            Value::Op(
                Op::Runtime {
                    name: "light_lazy",
                    arity: 2,
                },
                vec![Atom::Local(0), Atom::Func(builder)],
            ),
            None,
        );
        let service = self.service(fx.module, span);
        self.out.lazy_services.insert(service);
        let dst = fx.local(LocalType::Env);
        out.push(Stmt::LetEnv {
            dst,
            env: fx.env,
            service,
            value: Atom::Local(0),
            in_place,
        });
        fx.env = Env::Local(dst);
        true
    }

    /// Whether every use of the name `v` in `body` is the callee `v.f` of
    /// a call outside function literals, with one field index for `f`.
    fn only_field_calls(&self, module: ModuleId, body: &Block, v: VarId) -> bool {
        let mut w = Walk::default();
        w.block(body);
        let calls: Vec<Span> = w
            .field_calls
            .iter()
            .filter(|(x, _)| *x == v)
            .map(|(_, span)| *span)
            .collect();
        !calls.is_empty()
            && w.uses.get(&v).copied() == Some(calls.len() as u32)
            && calls
                .iter()
                .all(|span| self.lazy_field(module, *span).is_some())
    }

    /// The field index of the callee `x.f` at `span`, when it is the same
    /// for every constructor.
    fn lazy_field(&self, module: ModuleId, span: Span) -> Option<u32> {
        match self.typed.access.get(&(module, span)) {
            Some(Access::Field(list)) if !list.is_empty() => {
                let first = list[0].1;
                list.iter().all(|(_, i)| *i == first).then_some(first)
            }
            _ => None,
        }
    }

    /// The operation of a callee that names an external function of the
    /// library, as `target` finds it.
    fn library_op(&self, callee: &Expr) -> Option<Op> {
        match &callee.kind {
            ExprKind::Name(ValueRef::Def(d)) => self.def_op(*d),
            _ => None,
        }
    }

    fn def_op(&self, d: DefId) -> Option<Op> {
        match self.program.def(d).kind {
            DefKind::External { ref module, .. }
                if self.is_library(d.module) && module.starts_with("ilang:") =>
            {
                self.external_op(d).ok().map(|(op, _)| op)
            }
            DefKind::Runtime { ref module, .. } if module.starts_with("ilang:") => {
                self.external_op(d).ok().map(|(op, _)| op)
            }
            _ => None,
        }
    }

    fn closure(&mut self, fx: &mut Fx, out: &mut Vec<Stmt>, f: &Function) -> Atom {
        let captures = captures_of(f);
        let id = self.new_func();
        let name = format!("{}:fn{}", self.program.module(fx.module).path, id.0);
        let func = self.function(fx.module, f, &captures, name, id);
        self.out.funcs[id.0 as usize] = FuncKind::Code(func);
        let mut atoms = Vec::new();
        let mut later = Vec::new();
        for (i, v) in captures.iter().enumerate() {
            if fx.pending.as_ref().is_some_and(|p| p.contains(v)) {
                atoms.push(Atom::Unit);
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
                patched: !later.is_empty(),
            },
            site: None,
        });
        for (i, v) in later {
            fx.patches.push((dst, i, v));
        }
        Atom::Local(dst)
    }

    /// `v.f`: the constructor of the value picks the field.
    fn field(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        t: Atom,
        list: &[(CtorRef, u32)],
    ) -> Atom {
        if list.is_empty() {
            out.push(Stmt::Unreachable);
            return t;
        }
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

    /// Registers a comparison plan (children first) and gives its index.
    fn eq_plan(&mut self, plan: EqPlan) -> u32 {
        if let Some(i) = self.out.eq_plans.iter().position(|p| p == &plan) {
            return i as u32;
        }
        self.out.eq_plans.push(plan);
        (self.out.eq_plans.len() - 1) as u32
    }

    /// A plan for a closed structural type, or `None` when the type has an
    /// unsupported part. Caps: nesting depth 6, 32 plans. Over a cap, or for
    /// a function, host reference, opaque value or custom type, the generic
    /// comparison stays.
    fn build_eq_plan(&mut self, ty: &ShownType, depth: u32, nodes: &mut u32) -> Option<u32> {
        if depth > 6 || *nodes >= 32 {
            return None;
        }
        *nodes += 1;
        let plan = match ty {
            ShownType::Named(TypeRef::Builtin(BuiltinType::Int), args) if args.is_empty() => {
                EqPlan::Int
            }
            ShownType::Named(TypeRef::Builtin(BuiltinType::Float), args) if args.is_empty() => {
                EqPlan::Float
            }
            ShownType::Named(TypeRef::Builtin(BuiltinType::String), args) if args.is_empty() => {
                EqPlan::String
            }
            ShownType::Named(TypeRef::Builtin(BuiltinType::List), args) if args.len() == 1 => {
                let part = self.build_eq_plan(&args[0], depth + 1, nodes)?;
                EqPlan::List(part)
            }
            ShownType::Lit(text) => {
                if text.starts_with('"') {
                    EqPlan::String
                } else if text.contains('.') || text.contains('e') || text.contains('E') {
                    EqPlan::Float
                } else {
                    EqPlan::Int
                }
            }
            ShownType::Union(items) if !items.is_empty() => {
                let mut plans = Vec::with_capacity(items.len());
                for item in items {
                    plans.push(self.build_eq_plan(item, depth, nodes)?);
                }
                if plans.iter().all(|p| *p == plans[0]) {
                    self.out.eq_plans[plans[0] as usize].clone()
                } else {
                    return None;
                }
            }
            ShownType::Tuple(parts) if !parts.is_empty() => {
                let mut ids = Vec::with_capacity(parts.len());
                for part in parts {
                    ids.push(self.build_eq_plan(part, depth + 1, nodes)?);
                }
                EqPlan::Tuple(ids)
            }
            _ => return None,
        };
        Some(self.eq_plan(plan))
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
            // `a |> f` is `f(a)`, and `a |> f(b)` is `f(a, b)`.
            return match &rhs.kind {
                ExprKind::Call { callee, args } if args.iter().any(|a| a.label.is_some()) => {
                    self.labelled_call(fx, out, e.span, callee, Some(lhs), args, tail)
                }
                ExprKind::Call { callee, args } => {
                    let args: Vec<&Expr> = args.iter().map(|a| &a.value).collect();
                    self.call(fx, out, e.span, callee, Some(lhs), &args, tail)
                }
                _ => self.call(fx, out, e.span, rhs, Some(lhs), &[], tail),
            };
        }
        if matches!(op, BinOp::And | BinOp::Or) && tail {
            // The right side is in tail position.
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
        let plan = if self.specialize_equality && matches!(op, BinOp::Eq | BinOp::NotEq) {
            let types = self
                .typed
                .comparison_types
                .get(&(fx.module, e.span))
                .cloned();
            match types {
                Some((left, right)) if left == right => {
                    let mut nodes = 0;
                    match self.build_eq_plan(&left, 0, &mut nodes) {
                        // A bare scalar comparison keeps the generic (and
                        // numeric-plan) path; this pass is for structure.
                        Some(i)
                            if !matches!(
                                self.out.eq_plans[i as usize],
                                EqPlan::Int | EqPlan::Float | EqPlan::String
                            ) =>
                        {
                            Some(i)
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            None
        };
        let o = match op {
            BinOp::Eq => plan.map(Op::EqualPlan).unwrap_or(Op::Equal),
            BinOp::NotEq => plan.map(Op::NotEqualPlan).unwrap_or(Op::NotEqual),
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
                // A call of an external function is done when it is
                // reached, like all calls.
                DefKind::External { ref module, .. } | DefKind::Runtime { ref module, .. }
                    if !self.is_library(d.module) || !module.starts_with("ilang:") =>
                {
                    match self.external_func(*d) {
                        Some(f) => Target::Callee(Callee::Direct(f)),
                        None => Target::Op(Op::Panic),
                    }
                }
                DefKind::ServiceAccessor { ty } => {
                    Target::Callee(Callee::Direct(self.accessor_func(ty)))
                }
                DefKind::External { .. } | DefKind::Runtime { .. } => match self.external_op(*d) {
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

    /// A call with labelled arguments of a function value: the arguments are
    /// worked out as written, then passed in the
    /// canonical order of their shape (positions, then labels in UTF-8
    /// order), and a multi-shape value dispatches on that shape. Any other
    /// callee is called as `call` does.
    #[allow(clippy::too_many_arguments)]
    fn labelled_call(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        span: Span,
        callee: &Expr,
        first: Option<&Expr>,
        args: &[hir::Arg],
        tail: bool,
    ) -> Option<Atom> {
        if matches!(callee.kind, ExprKind::Name(ValueRef::Ctor(_))) {
            let args: Vec<&Expr> = args.iter().map(|a| &a.value).collect();
            return self.call(fx, out, span, callee, first, &args, tail);
        }
        let mut positional = Vec::new();
        let mut labelled: Vec<(String, Atom)> = Vec::new();
        let mut written = Vec::new();
        if let Some(a) = first {
            let v = self.expr(fx, out, a);
            written.push(v.clone());
            positional.push(v);
        }
        let target = self.target(fx, out, callee);
        for a in args {
            let v = self.expr(fx, out, &a.value);
            written.push(v.clone());
            match &a.label {
                Some(l) => labelled.push((l.name.clone(), v)),
                None => positional.push(v),
            }
        }
        labelled.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        let labels: Vec<String> = labelled.iter().map(|(l, _)| l.clone()).collect();
        let shape = ilang_check::types::ShapeId::new(positional.len() as u32, labels);
        let (target, all) = match target {
            Target::Callee(Callee::Value(f)) => {
                let id = crate::ir::shape_id(shape.positions(), self.shape_set(shape));
                let mut all = positional;
                all.extend(labelled.into_iter().map(|(_, v)| v));
                (Target::Callee(Callee::Shaped(f, id)), all)
            }
            // A known function takes its arguments in the order the
            // checker gives (`typed.args`), as `call` does.
            other => {
                let all = match self.typed.args.get(&(fx.module, span)) {
                    Some(order) if order.len() == written.len() => {
                        order.iter().map(|&i| written[i as usize].clone()).collect()
                    }
                    _ => written,
                };
                (other, all)
            }
        };
        self.finish_call(fx, out, span, target, all, tail)
    }

    /// A call, or `a |> f(…)` with `a` as `first`. The function is
    /// worked out first, then the arguments as they are written; for
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
        if let Some(a) = self.light_boundary(fx, out, span, callee, first, args, tail) {
            return a;
        }
        if let Some(a) = self.hostcall_call(fx, out, span, callee, first, args, tail) {
            return a;
        }
        let mut written = Vec::new();
        if let Some(a) = first {
            written.push(self.expr(fx, out, a));
        }
        let lazy = match &callee.kind {
            ExprKind::Field { target, .. } => match target.kind {
                ExprKind::Name(ValueRef::Local(v)) if fx.lazy_params.contains(&v) => {
                    let field = self
                        .lazy_field(fx.module, callee.span)
                        .expect("checked by only_field_calls");
                    Some(Callee::Lazy {
                        service: Atom::Local(fx.var(v)),
                        field,
                    })
                }
                _ => None,
            },
            _ => None,
        };
        let target = match lazy {
            Some(callee) => Target::Callee(callee),
            None => self.target(fx, out, callee),
        };
        for a in args {
            written.push(self.expr(fx, out, a));
        }
        let args: Vec<Atom> = match self.typed.args.get(&(fx.module, span)) {
            Some(order) if order.len() == written.len() => {
                order.iter().map(|&i| written[i as usize].clone()).collect()
            }
            _ => written,
        };
        self.finish_call(fx, out, span, target, args, tail)
    }

    /// `hostcall.call(op)` and `hostcall.call_in(g, op)`: `op` runs in
    /// no-wait mode. When it is a function literal whose body is one call of
    /// a field of an imported `"later"` service, the field value is tested
    /// at run time: the service's own import function (its import tag) is
    /// started under the ambient `Owner`, or `g`, and the request cell is the
    /// `Call`; any other value (a fake) is called now and its result is a
    /// ready `Call`. Any other `op` is called now the same way. `call_in`
    /// first panics `hostcall: group closed` when `g` is closed.
    #[allow(clippy::too_many_arguments)]
    fn hostcall_call(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        span: Span,
        callee: &Expr,
        first: Option<&Expr>,
        args: &[&Expr],
        tail: bool,
    ) -> Option<Option<Atom>> {
        let ExprKind::Name(ValueRef::Def(d)) = callee.kind else {
            return None;
        };
        if !self.is_library(d.module) {
            return None;
        }
        let in_group = match self.program.def(d).kind.foreign() {
            Some(("ilang:hostcall", "call")) => false,
            Some(("ilang:hostcall", "call_in")) => true,
            _ => return None,
        };
        let written: Vec<&Expr> = first.into_iter().chain(args.iter().copied()).collect();
        let (group, op) = match (in_group, written.as_slice()) {
            (false, [op]) => (None, *op),
            (true, [g, op]) => (Some(*g), *op),
            _ => return None,
        };
        let site = self.site(fx, span.start);
        let mut env = fx.env;
        if let Some(g) = group {
            let g = self.expr(fx, out, g);
            let open = self.library_callee("i/hostcall", "open_group");
            let_value(
                fx,
                out,
                Value::Call {
                    callee: open,
                    args: vec![g.clone()],
                    env: fx.env,
                },
                site,
            );
            let Some(owner) = self.out.request_owner else {
                self.error(
                    fx.module,
                    span,
                    "`hostcall.call_in` needs the request Owner",
                );
                return Some(finish(out, tail, Atom::Unit));
            };
            let dst = fx.local(LocalType::Env);
            out.push(Stmt::LetEnv {
                dst,
                env: fx.env,
                service: owner,
                value: g,
                in_place: false,
            });
            env = Env::Local(dst);
        }
        if group.is_none() {
            let Some(owner) = self.out.request_owner else {
                self.error(fx.module, span, "`hostcall.call` needs the request Owner");
                return Some(finish(out, tail, Atom::Unit));
            };
            let value = fx.local(LocalType::Value);
            out.push(Stmt::Let {
                dst: value,
                value: Value::Service(fx.env, owner),
                site,
            });
            let open = self.library_callee("i/hostcall", "open_group");
            let_value(
                fx,
                out,
                Value::Call {
                    callee: open,
                    args: vec![Atom::Local(value)],
                    env: fx.env,
                },
                site,
            );
        }
        let result = fx.local(LocalType::Value);
        let ready = |value: Atom| Stmt::Let {
            dst: result,
            value: Value::Op(
                Op::Runtime {
                    name: "request_ready",
                    arity: 1,
                },
                vec![value],
            ),
            site: None,
        };
        match self.field_start(fx.module, op) {
            Some((target, list, field_args, field, start)) => {
                let t = self.expr(fx, out, target);
                let value = self.field(fx, out, t, list);
                let mut values = Vec::new();
                for a in field_args {
                    values.push(self.expr(fx, out, a));
                }
                let answer = fx.local(LocalType::Value);
                out.push(Stmt::If {
                    cond: Cond::Equal(value.clone(), Atom::Func(field)),
                    then: vec![Stmt::Let {
                        dst: result,
                        value: Value::Call {
                            callee: Callee::Direct(start),
                            args: values.clone(),
                            env,
                        },
                        site,
                    }],
                    otherwise: vec![
                        Stmt::Let {
                            dst: answer,
                            value: Value::Call {
                                callee: Callee::Value(value),
                                args: values,
                                env,
                            },
                            site,
                        },
                        ready(Atom::Local(answer)),
                    ],
                });
            }
            None => {
                let f = self.expr(fx, out, op);
                let answer = let_value(
                    fx,
                    out,
                    Value::Call {
                        callee: Callee::Value(f),
                        args: vec![],
                        env,
                    },
                    site,
                );
                out.push(ready(answer));
            }
        }
        Some(finish(out, tail, Atom::Local(result)))
    }

    /// The parts of `() { x.f(args) }` when `f` is a field of an imported
    /// service whose calls are starts: `x`, the field access, the arguments,
    /// the field's import function and its start function.
    #[allow(clippy::type_complexity)]
    fn field_start<'e>(
        &mut self,
        module: ModuleId,
        op: &'e Expr,
    ) -> Option<(
        &'e Expr,
        &'a [(CtorRef, u32)],
        Vec<&'e Expr>,
        FuncId,
        FuncId,
    )> {
        let ExprKind::Fn(f) = &ungroup(op).kind else {
            return None;
        };
        let ([], [hir::Stmt::Expr(body)]) = (f.params.as_slice(), f.body.stmts.as_slice()) else {
            return None;
        };
        let ExprKind::Call { callee, args } = &ungroup(body).kind else {
            return None;
        };
        if args.iter().any(|a| a.label.is_some()) {
            return None;
        }
        let ExprKind::Field { target, .. } = &callee.kind else {
            return None;
        };
        let typed: &'a Typed = self.typed;
        let Some(Access::Field(list)) = typed.access.get(&(module, callee.span)) else {
            return None;
        };
        let (c, index) = *list.first()?;
        if list
            .iter()
            .any(|&(other, i)| other.ty != c.ty || i != index)
        {
            return None;
        }
        self.program.ty(c.ty).import.as_ref()?;
        let (field, start) = self.service_field_start(c.ty, index)?;
        let written: Vec<&Expr> = args.iter().map(|a| &a.value).collect();
        let ordered = match typed.args.get(&(module, body.span)) {
            Some(order) if order.len() == written.len() => {
                order.iter().map(|&i| written[i as usize]).collect()
            }
            _ => written,
        };
        Some((target, list.as_slice(), ordered, field, start))
    }

    /// The call of a worked-out callee with its arguments in order.
    fn finish_call(
        &mut self,
        fx: &mut Fx,
        out: &mut Vec<Stmt>,
        span: Span,
        target: Target,
        args: Vec<Atom>,
        tail: bool,
    ) -> Option<Atom> {
        match target {
            Target::Ctor(c) => {
                let info = &self.ctors[&c];
                let a = match info.repr {
                    CtorRepr::Bool(b) => Atom::Bool(b),
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
                // position counts as a tail call: its position is
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
                    // The position of a call in tail position is
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

    // ---- Bit arrays ----

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

    // ---- case ----

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
        // The clauses cover every value.
        body.push(Stmt::Unreachable);
        out.push(Stmt::Block { label: end, body });
        dst.map(Atom::Local)
    }

    // ---- Patterns ----

    fn bind_var(fx: &mut Fx, out: &mut Vec<Stmt>, b: &Binder, v: Atom) {
        let l = fx.var(b.var);
        out.push(Stmt::Let {
            dst: l,
            value: Value::Atom(v),
            site: None,
        });
    }

    /// Matches `v` with the pattern of a `case` clause, and leaves to `fail`
    /// when it does not match.
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
            PatternKind::Keyed { pattern, .. } => self.pattern(fx, out, pattern, v, fail),
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
            PatternKind::Prefix {
                prefix,
                suffix,
                rest,
            } => {
                guard(out, Cond::Tag(v.clone(), 3), fail);
                guard(
                    out,
                    Cond::StartsWith(
                        v.clone(),
                        Atom::String(prefix.clone()),
                        Atom::String(suffix.clone()),
                    ),
                    fail,
                );
                if let Some(b) = rest {
                    let r = let_value(
                        fx,
                        out,
                        Value::StringSuffix(v, prefix.len() as u32, suffix.len() as u32),
                        None,
                    );
                    Self::bind_var(fx, out, b, r);
                }
            }
            PatternKind::Constructor { ctor, args, .. } => {
                let info = self.ctors[ctor].clone();
                let cond = match info.repr {
                    CtorRepr::Bool(b) => Cond::Bool(v.clone(), b),
                    CtorRepr::Record { ctor, .. } => Cond::Ctor(v.clone(), ctor),
                };
                guard(out, cond, fail);
                for (arg, i) in self.fields_of(fx, p, args) {
                    let f = let_value(fx, out, Value::Field(v.clone(), i), None);
                    self.pattern(fx, out, arg, f, fail);
                }
            }
            PatternKind::Tuple(parts) | PatternKind::Data { parts, .. } => {
                let cond = self.head_cond(v.clone(), Head::Shape(p.data_shape().unwrap()));
                guard(out, cond, fail);
                for (i, part) in parts.iter().enumerate() {
                    let f = let_value(fx, out, Value::Field(v.clone(), i as u32), None);
                    self.pattern(fx, out, part, f, fail);
                }
            }
            PatternKind::List { items, rest } => {
                // Only lists are `$cons` cells, so a first cell proves the
                // tag. `[]` tests for a cell first, which fails at once on
                // the cells of a recursion, and the tag after that.
                let first = items.is_empty();
                if first {
                    if rest.is_none() {
                        guard(out, Cond::Empty(v.clone()), fail);
                    }
                    guard(out, Cond::Tag(v.clone(), 6), fail);
                }
                let mut cur = v;
                for item in items {
                    guard(out, Cond::Not(Box::new(Cond::Empty(cur.clone()))), fail);
                    let h = let_value(fx, out, Value::ListHead(cur.clone()), None);
                    self.pattern(fx, out, item, h, fail);
                    cur = let_value(fx, out, Value::ListTail(cur), None);
                }
                match rest {
                    None if first => {}
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

    /// The test of a type test `x: T` for one member of `T`.
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
            Head::Shape(shape) if shape.has_labels() => {
                Cond::Data(v, self.shape_set(shape), shape.len() as u32)
            }
            // `()`, empty data, is the null reference: tag 0.
            Head::Shape(shape) if shape.is_empty() => Cond::Tag(v, 0),
            Head::Shape(shape) => Cond::Tuple(v, shape.positions()),
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
                    BuiltinType::Key => 15,
                    BuiltinType::Keyed => 16,
                    BuiltinType::Request => 17,
                    BuiltinType::Ticket => 18,
                    BuiltinType::Never => u32::MAX,
                },
            ),
            Head::Named(TypeRef::Custom(t)) => match self.types.get(&t) {
                Some(TypeRepr::Bool(b)) => Cond::And(
                    Box::new(Cond::Tag(v.clone(), 1)),
                    Box::new(Cond::Bool(v, *b)),
                ),
                Some(TypeRepr::Record(id)) => Cond::Type(v, *id),
                Some(TypeRepr::Ctor(id, ctor)) => Cond::And(
                    Box::new(Cond::Type(v.clone(), *id)),
                    Box::new(Cond::Ctor(v, *ctor)),
                ),
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

    /// The segments are matched from the start, and all the bits must
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
                        unreachable!("checked by check_bit_pattern")
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
    /// to, as the checker found them.
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
            PatternKind::Named { pattern, .. } | PatternKind::Keyed { pattern, .. } => {
                self.plain(pattern)
            }
            PatternKind::Tuple(parts) | PatternKind::Data { parts, .. } => {
                parts.iter().all(|x| self.plain(x))
            }
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

    /// Binds the pattern of a binding or a parameter. When
    /// it may fail, a value that does not match is reported to `Assert`:
    /// `a.fail(AssertionFailure(m))`, with the message `m` of the binding and the
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
        // `fail` never gives a value; a pattern that can not fail
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
        // `\r\n` and `\r` are the same as `\n`.
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
            PatternKind::Keyed { pattern, .. } => self.bind(fx, out, pattern, v),
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
            PatternKind::Tuple(parts) | PatternKind::Data { parts, .. } => {
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

/// The ABI type of a type that the host can take. `Never`
/// has no values, so it is passed as nothing, like `()`.
fn abi_type(t: HostType) -> AbiType {
    match t {
        HostType::Unit => AbiType::Unit,
        HostType::Bool => AbiType::Bool,
        HostType::Int => AbiType::Int,
        HostType::Float => AbiType::Float,
        HostType::String => AbiType::String,
        HostType::BitArray => AbiType::BitArray,
        HostType::Opaque(id) => AbiType::Externref(id.0),
    }
}

/// The standard service: the type `Io` of the library module
/// `i/io`. No other package may have a module path that starts with `i/`,
/// so this is that type and no other. A host service declared
/// elsewhere with the namespace `i:io/v1` is a normal host service.
fn is_standard_service(program: &hir::Program, t: TypeId) -> bool {
    let def = program.ty(t);
    def.name == "Io" && program.module(def.module).path == "i/io"
}

/// Whether the function type `t` is written with the result `Result(A, E)`
/// of the prelude, also through aliases: an imported field's answer.
fn returns_result(program: &hir::Program, t: &hir::Type) -> bool {
    let TypeKind::Fn { ret, .. } = &t.kind else {
        return false;
    };
    let mut ret: &hir::Type = ret;
    for _ in 0..64 {
        let TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } = &ret.kind
        else {
            return false;
        };
        let def = program.ty(*id);
        if def.name == "Result" && Some(def.module) == program.prelude {
            return true;
        }
        match &def.body {
            TypeBody::Alias(body) => ret = body,
            TypeBody::Custom(_) => return false,
        }
    }
    false
}

/// The host signature must preserve wire order and result tags.
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

/// The `ilang.abi` metadata.
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
            // The module gives it itself.
            ImportKind::WasiStdout | ImportKind::WasiAdapter => continue,
            ImportKind::External => (
                &mut externals,
                serde_json::json!({
                    "params": names(&x.abi.params),
                    "result": x.abi.result.name(),
                    "errors": names(&x.abi.errors),
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
        "version": 5,
        "entry": entry,
        "tests": tests,
        "exports": map,
        "services": services,
        "externals": externals,
    });
    // Only a module of the WASI target says its target.
    if target == ir::Target::Wasi {
        abi["target"] = target.name().into();
    }
    abi.to_string()
}
