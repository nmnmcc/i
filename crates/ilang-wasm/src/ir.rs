//! A small intermediate form between the resolved program (HIR) and Wasm.
//!
//! It is in A-normal form: every operand is an [`Atom`] (a local, a global, or
//! a literal), so the order of evaluation (§6.2) is fixed by the order of the
//! statements, and every call can be a tail call once its arguments are known.
//! Control flow is structured, as in Wasm: blocks that a `Break` leaves, `If`,
//! and statements that end a function (`Return`, `TailCall`, `Unreachable`).
//!
//! Every value is an `eqref` (see `runtime/mod.rs`). A few locals hold `i64`
//! or `i32` numbers, for positions and widths in bit array patterns.
//!
//! # Calling convention
//!
//! A function of n parameters is the Wasm type
//! `$fn_n = (func (param (ref null $obj) (ref null $env) eqref×n) (result eqref))`.
//! The first parameter is its closure; its captures are the `$values` array
//! in field 5 of the closure. The second is the environment (§9.2). A
//! closure is a `$clo_n`, a subtype of `$obj` with tag 7, the arity in field
//! 2, and a reference to the code after the eight `$obj` fields.
//!
//! # The environment (§9.2)
//!
//! An environment is a `$env`, a GC array of `eqref` with one slot for each
//! service type constructor of the program, by its service id. Slot
//! [`ASSERT_SERVICE`] is `Assert`. Every call passes the environment of the
//! place of the call: the `$env` parameter of the function, or the local made
//! by the last environment statement of an enclosing block (`LetEnv`), which
//! copies the array and sets one slot. Entry points start with the root
//! environment, which has the `Assert` of the runtime (§9.6).
//!
//! # Errors and unwinding (§9.3, §9.5)
//!
//! Errors are the Wasm exception `$error (param eqref)`. Unwinding a
//! cancelled continuation is the exception `$cancel (param eqref)`, whose
//! value is the continuation. `fail.catch` takes only `$error`;
//! `resource.ensure` takes both, runs its final function, and throws the
//! exception again. No handler takes anything else, so a stop (a trap) never
//! runs handlers.
//!
//! # Control (§9.4)
//!
//! Frames are captured lazily, with the exception `$capture`. `suspend`
//! throws it with a capture object; every frame between it and its boundary
//! adds a **frame record** to the object as the exception goes through, and
//! throws it again. The boundary makes the continuation from the records.
//!
//! A function **may suspend** when it makes a call that is not a tail call to
//! a closure, to an operation that calls i code or captures
//! ([`Op::suspends`]), or to a function that may suspend. In such a
//! function, each such call is a [`Stmt::Suspendable`] with a [`Point`]: a
//! handler around the call adds the frame record of the point. The point has
//! a **segment**, a function with the rest of the function after the call,
//! and a **re-entry** function that rebuilds the frame when the frames inside
//! it have returned: it rebuilds the environment, puts the frame on the
//! shadow stack with the slot that the frame record keeps, and goes on with
//! the segment. A segment goes on with a
//! [`Join`], the code after an `If` or a `Block` around the point, so that no
//! code is in more than one segment. The normal path does not use segments:
//! the rest of the function stays where it is. See `control.rs` and
//! `emit.rs`.
//!
//! # The host boundary (§9.7, §10, Appendix B)
//!
//! Each [`Import`] has a function of the program that calls it, with the
//! encoding of B.2 ([`Value::HostCall`]); an import of external functions has
//! one for each declaration, so that two declarations are two values
//! (§11.4). An async entry point runs inside a
//! root boundary; a function of an async host service starts its import
//! ([`Value::HostStart`]), suspends to that boundary with `control.suspend`,
//! and reads the answer when its `ilang_resume:…` export resumes it
//! ([`Value::HostReply`]). So the control machinery of §9.4 saves the frames
//! while the program waits.

/// The service id of `Assert` (§9.6).
pub const ASSERT_SERVICE: u32 = 0;

use num_bigint::BigInt;

/// A local of the function. The first `arity` locals are its parameters.
pub type Local = u32;
/// A label of a `Block`.
pub type Label = u32;
/// An index in the site table (§11.7). With [`TAIL_SITE`], it is a **tail
/// site**: the position of a call in tail position, which is printed only
/// as the first position of a stop (§11.7).
pub type Site = u32;

/// The flag of a tail site.
pub const TAIL_SITE: Site = 1 << 31;

/// The number that the shadow stack holds for a site: the index, or
/// `-2 - index` for a tail site (`runtime/stops.rs`).
pub fn site_code(s: Site) -> i64 {
    if s & TAIL_SITE != 0 {
        -2 - (s & !TAIL_SITE) as i64
    } else {
        s as i64
    }
}
/// An index in [`Program::points`].
pub type PointId = u32;
/// An index in [`Program::joins`].
pub type JoinId = u32;
/// An index in [`Program::imports`].
pub type ImportId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalType {
    Value,
    I64,
    I32,
    /// An environment, `(ref null $env)`.
    Env,
}

/// The environment that a call gets (§9.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Env {
    /// The environment of the running function.
    Param,
    /// A local of type `Env`, made by an environment statement.
    Local(Local),
}

/// An operand: it is worked out without any effect, so it may be used more
/// than once.
#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    Local(Local),
    /// A top-level value that is worked out in `ilang_start`.
    Global(GlobalId),
    /// The one closure of a function that captures nothing: a top-level
    /// function, a constructor, or a library function used as a value.
    Func(FuncId),
    Nil,
    Bool(bool),
    Int(BigInt),
    Float(f64),
    String(String),
    /// A value of a constructor with no fields.
    Ctor {
        ctor: u32,
        ty: u32,
    },
    EmptyList,
}

/// An `i64` operand.
#[derive(Clone, Debug, PartialEq)]
pub enum Num {
    Local(Local),
    Const(i64),
}

/// Operations of the runtime on values. Each gives a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    IntAdd,
    IntSub,
    IntMul,
    IntDiv,
    IntRem,
    IntNeg,
    IntLt,
    IntLe,
    IntGt,
    IntGe,
    FloatAdd,
    FloatSub,
    FloatMul,
    FloatDiv,
    FloatNeg,
    FloatLt,
    FloatLe,
    FloatGt,
    FloatGe,
    Equal,
    NotEqual,
    StringAppend,
    /// `panic(m)`: it never gives a value.
    Panic,
    /// A function of the runtime, such as `$core_inspect`, called with
    /// `arity` values.
    Runtime {
        name: &'static str,
        arity: u32,
    },
    /// `fail.raise(e)`: throws `$error`. It never gives a value.
    Raise,
    /// `fail.catch(body)`: `Ok` of the value, or `Error` of the error.
    Catch,
    /// `resource.ensure(body, with)`.
    Ensure,
    /// `control.boundary(body)` (§9.4).
    Boundary,
    /// `control.suspend(b, f)`: it throws `$capture`.
    Suspend,
    /// `control.resume(k, value)`.
    Resume,
    /// `control.cancel(k)`.
    Cancel,
    /// Keeps all currently bound services in a fresh one-argument function.
    KeepEnv,
    WaitRequests,
    PollRequests,
}

impl Op {
    /// Whether the operation may stop the program (§11.7), so that its
    /// position must be on the shadow stack.
    pub fn may_stop(self) -> bool {
        matches!(
            self,
            Op::FloatAdd
                | Op::FloatSub
                | Op::FloatMul
                | Op::FloatDiv
                | Op::Panic
                | Op::Runtime { .. }
                | Op::Catch
                | Op::Ensure
                | Op::Boundary
                | Op::Suspend
                | Op::Resume
                | Op::Cancel
                | Op::WaitRequests
                | Op::PollRequests
        )
    }

    /// Whether the operation calls i code, which gets the environment.
    pub fn needs_env(self) -> bool {
        matches!(
            self,
            Op::Catch
                | Op::Ensure
                | Op::Boundary
                | Op::Resume
                | Op::Cancel
                | Op::KeepEnv
                | Op::WaitRequests
                | Op::PollRequests
        )
    }

    /// Whether a `$capture` may go through the operation: it calls i code,
    /// or it is `suspend`.
    pub fn suspends(self) -> bool {
        matches!(
            self,
            Op::Catch
                | Op::Ensure
                | Op::Boundary
                | Op::Suspend
                | Op::Resume
                | Op::Cancel
                | Op::WaitRequests
                | Op::PollRequests
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Callee {
    /// A known function, called with `call`.
    Direct(FuncId),
    /// A closure value, called with `call_ref` after a cast to `$clo_n`.
    Value(Atom),
}

/// The right side of a `Let`.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Atom(Atom),
    /// Capture i of the running closure.
    Capture(u32),
    Call {
        callee: Callee,
        args: Vec<Atom>,
        env: Env,
    },
    Op(Op, Vec<Atom>),
    /// An operation that calls i code with this environment.
    OpEnv(Op, Vec<Atom>, Env),
    /// The service with this id in the environment (§9.2).
    Service(Env, u32),
    Closure {
        func: FuncId,
        captures: Vec<Atom>,
        patched: bool,
    },
    Tuple(Vec<Atom>),
    Record {
        ctor: u32,
        ty: u32,
        fields: Vec<Atom>,
    },
    Cons(Atom, Atom),
    /// Field or part i of a custom value or a tuple.
    Field(Atom, u32),
    ListHead(Atom),
    ListTail(Atom),
    /// The string after its first `bytes` bytes.
    StringSuffix(Atom, u32),
    // Making bit arrays (§8.3). Widths are in bits.
    BitsEmpty,
    BitsConcat(Atom, Atom),
    BitsFromInt {
        value: Atom,
        width: Num,
        little: bool,
    },
    BitsFromFloat {
        value: Atom,
        width: u32,
        little: bool,
    },
    BitsFromUtf {
        value: Atom,
        encoding: u32,
        little: bool,
        codepoint: bool,
    },
    BitsTake(Atom, Num),
    BitsWhole(Atom),
    // Matching bit arrays (§8.4).
    BitsReadInt {
        bits: Atom,
        start: Num,
        width: Num,
        signed: bool,
        little: bool,
    },
    BitsReadFloat {
        bits: Atom,
        start: Num,
        width: u32,
        little: bool,
    },
    BitsSlice {
        bits: Atom,
        start: Num,
        length: Num,
    },
    // The host boundary (§9.7, §10.1, B.2–B.4).
    /// Calls the import with the arguments, and gives its value or throws
    /// its error. A value that is not correct stops (B.2).
    HostCall {
        import: ImportId,
        args: Vec<Atom>,
        result: AbiType,
    },
    /// Starts the async import with the arguments, and records that the
    /// host may answer it (B.4). Its value is the typed request cell.
    HostStart {
        import: ImportId,
        args: Vec<Atom>,
        result: AbiType,
    },
    /// The answer of the async import, given to its `ilang_resume:…`
    /// export (B.5): its value, or its error thrown.
    HostReply(ImportReply),
}

/// A declaration's decoder, even when its wire import is shared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImportReply {
    pub import: ImportId,
    pub result: AbiType,
}

/// An `i64` value for a `Let` into an `I64` local.
#[derive(Clone, Debug, PartialEq)]
pub enum NumValue {
    Num(Num),
    Add(Num, Num),
    Sub(Num, Num),
    /// The number of bits of a bit array.
    BitsLength(Atom),
    /// The length of a segment when a bit array is made: the Int times
    /// `unit`; stops when it is negative (§8.3).
    LengthOf(Atom, u32),
    /// The length of a `bits(n)` or `bytes(n)` segment when a bit array is
    /// made: as `LengthOf`, but a length that no bit array can have is
    /// still a number, so that the value is too short (§8.3).
    TakeLength(Atom, u32),
    /// The length of a segment in a pattern: the Int times `unit`, or -1
    /// when it is negative or too big, which never matches (§8.4).
    WidthOf(Atom, u32),
}

/// A test, as an `i32`.
#[derive(Debug)]
pub enum Cond {
    /// A Bool value that is `True`.
    True(Atom),
    Not(Box<Cond>),
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
    /// The value tag of `runtime/mod.rs`.
    Tag(Atom, u32),
    Bool(Atom, bool),
    Ctor(Atom, u32),
    /// A custom value of the type with this id.
    Type(Atom, u32),
    Tuple(Atom, u32),
    /// A list that is `[]`; the value is a list.
    Empty(Atom),
    Equal(Atom, Atom),
    /// The value is this Int, Float, or String: a literal pattern (§7.2).
    IsInt(Atom, BigInt),
    IsFloat(Atom, f64),
    IsString(Atom, String),
    StartsWith(Atom, Atom),
    /// The `i32` local is not 0.
    I32(Local),
    NumLt(Num, Num),
    NumEq(Num, Num),
    /// The number is a multiple of 8.
    Whole(Num),
    /// The bits from `start`, `width` of them, are in the bit array.
    BitsHas(Atom, Num, Num),
    BitsFloat {
        bits: Atom,
        start: Num,
        width: u32,
        little: bool,
    },
    BitsEqual(Atom, Atom),
}

#[derive(Debug)]
pub enum Stmt {
    /// `dst = value`. With a site, the site is put on the shadow stack first.
    Let {
        dst: Local,
        value: Value,
        site: Option<Site>,
    },
    LetNum {
        dst: Local,
        value: NumValue,
        site: Option<Site>,
    },
    /// Reads one codepoint of a `_codepoint` segment: the codepoint, its
    /// width in bits, and whether there was one.
    ReadUtf {
        codepoint: Local,
        width: Local,
        ok: Local,
        bits: Atom,
        start: Num,
        encoding: u32,
        little: bool,
    },
    If {
        cond: Cond,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    /// A block that `Break(label)` leaves.
    Block {
        label: Label,
        body: Vec<Stmt>,
    },
    Break(Label),
    /// Leaves the function with the value.
    Return(Atom),
    /// A call in tail position (§11.6): the frame is gone first. Its site,
    /// a tail site, is the first position of a stop until the callee works
    /// out an expression of its own (§11.7). Library code has none, and
    /// passes on the tail site of its own call.
    TailCall {
        callee: Callee,
        args: Vec<Atom>,
        env: Env,
        site: Option<Site>,
    },
    /// `dst` is `env` with the service `value` in the slot `service`: an
    /// environment statement (§6.10, §9.2).
    LetEnv {
        dst: Local,
        env: Env,
        service: u32,
        value: Atom,
    },
    Unreachable,
    /// Fills a capture of a closure made by a direct value, once the names of
    /// its binding are bound (§6.10).
    SetCapture {
        closure: Local,
        index: u32,
        value: Atom,
    },
    SetGlobal(GlobalId, Atom),
    /// `dst = value`, as `Let`, for a call that may suspend in a function
    /// that may suspend (§9.4): a `$capture` that goes through the call gets
    /// the frame record of `point`. Made by `control.rs`.
    Suspendable {
        dst: Local,
        value: Value,
        site: Option<Site>,
        point: PointId,
    },
    /// Goes on with the segment of a point, after its `Suspendable`: a tail
    /// call with the locals that the segment needs. Only in segments.
    Continue(PointId),
    /// Goes on with a join: a tail call with the locals that it needs. Only
    /// in segments and joins.
    Join(JoinId),
}

/// A function of the program.
#[derive(Clone, Debug)]
pub struct Func {
    /// A name for reading the Wasm text.
    pub name: String,
    pub arity: u32,
    /// Logical frame units (source parameters and local names), before optimization.
    pub budget_frame: u32,
    /// The types of the locals; the first `arity` are the parameters.
    pub locals: Vec<LocalType>,
    pub body: Vec<Stmt>,
    /// Whether the function has a frame on the shadow stack (§11.7).
    pub frame: bool,
}

/// A call that may suspend in a function that may suspend (§9.4), and how
/// its frame is captured and rebuilt. Its segment and re-entry function have
/// the locals of the function `func`. The frame record keeps the slot of the
/// frame on the shadow stack, which the re-entry puts back (§11.7).
#[derive(Clone, Debug)]
pub struct Point {
    pub func: FuncId,
    /// The environment of the call. The environment statements between the
    /// function's environment and this one (`Point::delta`) are kept in the
    /// frame record, so that the frames inside it get them when they are
    /// re-entered (§9.4 rule 3).
    pub env: Env,
    /// The value of the call.
    pub dst: Local,
    /// The locals that the segment needs, in order: its parameters after
    /// the environment, and before the value of the call.
    pub params: Vec<Local>,
    /// The environment locals of the function that the re-entry rebuilds
    /// from the environment where the continuation is resumed, in order:
    /// `dst` is the environment `from` with the service `service`, whose
    /// value is in the frame record (§9.4 rule 3).
    pub envs: Vec<EnvLink>,
    /// The rest of the function after the call.
    pub segment: Vec<Stmt>,
    /// The locals of the function that the segment declares.
    pub locals: Vec<Local>,
}

/// The code after an `If` or a `Block` that has a point in it, in a
/// function that may suspend (§9.4): segments go on with it as a tail call,
/// so that the code after it is in one place and not in every segment. It
/// has the locals of the function `func`.
#[derive(Clone, Debug)]
pub struct Join {
    pub func: FuncId,
    /// The locals that are live after the statement: its parameters after
    /// the environment.
    pub params: Vec<Local>,
    /// The rest of the function after the statement.
    pub segment: Vec<Stmt>,
    /// The locals of the function that the join declares.
    pub locals: Vec<Local>,
}

/// An environment statement that a frame rebuilds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvLink {
    pub dst: Local,
    pub from: Env,
    pub service: u32,
}

impl Point {
    /// The locals whose values are in the frame record, before the values
    /// of the environment statements of `envs`.
    pub fn saved<'a>(&'a self, locals: &'a [LocalType]) -> impl Iterator<Item = Local> + 'a {
        self.params
            .iter()
            .copied()
            .filter(|&l| locals[l as usize] != LocalType::Env)
    }

    /// The environment statements between the function's environment and
    /// the environment of the call, outermost first: the links of `envs`
    /// from `env` back to [`Env::Param`]. `None` when a link is missing.
    pub fn delta(&self) -> Option<Vec<EnvLink>> {
        let mut links = Vec::new();
        let mut env = self.env;
        loop {
            match env {
                Env::Param => break,
                Env::Local(l) => {
                    let link = self.envs.iter().find(|e| e.dst == l)?;
                    if links.len() == self.envs.len() {
                        // The links go round in a circle.
                        return None;
                    }
                    links.push(*link);
                    env = link.from;
                }
            }
        }
        links.reverse();
        Some(links)
    }
}

/// How a function is made.
#[derive(Clone, Debug)]
pub enum FuncKind {
    /// Code of the program.
    Code(Func),
    /// A constructor used as a value.
    Ctor { ctor: u32, ty: u32, arity: u32 },
    /// A library function of the runtime used as a value.
    Runtime { op: Op, arity: u32 },
}

impl FuncKind {
    pub fn arity(&self) -> u32 {
        match self {
            FuncKind::Code(f) => f.arity,
            FuncKind::Ctor { arity, .. } | FuncKind::Runtime { arity, .. } => *arity,
        }
    }
}

/// How an export of the binary interface gives and takes values (B.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AbiType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    BitArray,
    /// The resolved nominal type, carried only inside the guest.
    Externref(u32),
}

impl AbiType {
    pub fn name(self) -> &'static str {
        match self {
            AbiType::Nil => "nil",
            AbiType::Bool => "bool",
            AbiType::Int => "int",
            AbiType::Float => "float",
            AbiType::String => "string",
            AbiType::BitArray => "bit_array",
            AbiType::Externref(_) => "externref",
        }
    }
}

/// How a host export takes and gives values (B.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportAbi {
    pub params: Vec<AbiType>,
    pub result: AbiType,
    /// The members of the errors, in the canonical order.
    pub errors: Vec<AbiType>,
}

/// An export of the module.
#[derive(Clone, Debug)]
pub struct Export {
    pub name: String,
    pub callee: Callee,
    /// `None` for the program entry and tests: no parameters and no results.
    pub abi: Option<ExportAbi>,
    /// For the program entry and tests: the site of the binding, the only
    /// position of the stop for an error that comes to it (§11.7).
    pub site: Option<Site>,
    /// For an async entry point (§10.2, B.5): a function of one parameter,
    /// the root boundary, whose captures are the arguments; it calls the
    /// entry point with them.
    pub async_body: Option<FuncId>,
}

/// What an import of the host is (B.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind {
    External,
    Service,
    /// A function of an async host service: no results; the answer comes
    /// through `ilang_resume:…` (B.5).
    AsyncService,
    /// `print` of the standard service, `Io` of `i/io`, on the WASI target:
    /// the module defines it, with `fd_write` to the standard output (B.7).
    WasiStdout,
}

/// The host a module is made for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    /// A host of the i binary interface (Appendix B), such as `i run`, the
    /// Rust host, and the JavaScript SDK.
    #[default]
    Ilang,
    /// A WASI preview 1 command, which runs in a stock Wasmtime (B.7).
    Wasi,
}

impl Target {
    /// The name in the metadata (B.6).
    pub fn name(self) -> &'static str {
        match self {
            Target::Ilang => "ilang",
            Target::Wasi => "wasi",
        }
    }
}

/// A Wasm import of a function of the host (B.4).
#[derive(Clone, Debug)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub abi: ExportAbi,
    pub kind: ImportKind,
    pub request: bool,
    pub request_result: bool,
    pub awaited: bool,
}

/// A host service that the host gives (§9.7): its value is put in the root
/// environment, a record of closures of `fields`, which call the imports.
#[derive(Clone, Debug)]
pub struct HostService {
    pub service: u32,
    pub ctor: u32,
    pub ty: u32,
    pub fields: Vec<FuncId>,
}

/// One position of the site table: module index, line, column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SitePos {
    pub module: u32,
    pub line: u32,
    pub column: u32,
}

/// The whole program in the intermediate form.
#[derive(Clone, Debug, Default)]
pub struct Program {
    pub funcs: Vec<FuncKind>,
    /// The points of the calls that may suspend (§9.4).
    pub points: Vec<Point>,
    /// The joins of the segments of the points (§9.4).
    pub joins: Vec<Join>,
    /// The functions with one constant closure, used by `Atom::Func`.
    pub constants: Vec<FuncId>,
    /// The number of globals for top-level values.
    pub globals: u32,
    /// The number of service ids, at least 1 for `Assert` (§9.2).
    pub services: u32,
    pub request_owner: Option<u32>,
    /// The code of `ilang_start` after the tables are installed.
    pub start: Option<Func>,
    pub exports: Vec<Export>,
    pub sites: Vec<SitePos>,
    /// Optional template/call-chain notes, parallel to `sites`.
    pub site_notes: Vec<Option<String>>,
    /// The module paths of the site table, by module index (§11.7).
    pub site_modules: Vec<String>,
    /// The constructor names, from `FIRST_CONSTRUCTOR` on.
    pub constructors: Vec<String>,
    /// The `ilang.abi` metadata (B.6).
    pub abi_json: String,
    /// The functions that the host gives (B.4).
    pub imports: Vec<Import>,
    /// The host services of the root environment (§9.7).
    pub host_services: Vec<HostService>,
    /// The host the module is made for.
    pub target: Target,
    /// On the WASI target: the export of the program entry, which `_start`
    /// calls after `ilang_start` (B.7).
    pub entry: Option<String>,
}

#[path = "ir_tree.rs"]
mod tree;
