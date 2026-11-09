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
//! in field 5 of the closure. The second is the environment, which is null
//! until environments come (M5). A closure is a `$clo_n`, a subtype of `$obj`
//! with tag 7, the arity in field 2, and a reference to the code after the
//! eight `$obj` fields.
//!
//! # Later nodes
//!
//! Effects add statements here, not a new form: getting and pushing services
//! of the environment, `raise` and catching errors (`try_table`), final
//! functions on unwinding, and `boundary`, `suspend`, `resume`, and `cancel`.

use num_bigint::BigInt;

/// A local of the function. The first `arity` locals are its parameters.
pub type Local = u32;
/// A label of a `Block`.
pub type Label = u32;
/// An index in the site table (§11.7).
pub type Site = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuncId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlobalId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalType {
    Value,
    I64,
    I32,
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
    },
    Op(Op, Vec<Atom>),
    Closure {
        func: FuncId,
        captures: Vec<Atom>,
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
    /// The length of a segment in a pattern: the Int times `unit`, or -1
    /// when it is negative or too big, which never matches (§8.4).
    WidthOf(Atom, u32),
}

/// A test, as an `i32`.
#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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
    /// A call in tail position (§11.6): the frame is gone first.
    TailCall {
        callee: Callee,
        args: Vec<Atom>,
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
}

/// A function of the program.
#[derive(Clone, Debug)]
pub struct Func {
    /// A name for reading the Wasm text.
    pub name: String,
    pub arity: u32,
    /// The types of the locals; the first `arity` are the parameters.
    pub locals: Vec<LocalType>,
    pub body: Vec<Stmt>,
    /// Whether the function has a frame on the shadow stack (§11.7).
    pub frame: bool,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbiType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    BitArray,
    Externref,
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
            AbiType::Externref => "externref",
        }
    }
}

/// An export of the module.
#[derive(Clone, Debug)]
pub struct Export {
    pub name: String,
    pub callee: Callee,
    /// `None` for the program entry and tests: no parameters and no results.
    pub abi: Option<(Vec<AbiType>, AbiType)>,
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
    /// The functions with one constant closure, used by `Atom::Func`.
    pub constants: Vec<FuncId>,
    /// The number of globals for top-level values.
    pub globals: u32,
    /// The code of `ilang_start` after the tables are installed.
    pub start: Option<Func>,
    pub exports: Vec<Export>,
    pub sites: Vec<SitePos>,
    /// The module paths of the site table, by module index (§11.7).
    pub site_modules: Vec<String>,
    /// The constructor names, from `FIRST_CONSTRUCTOR` on.
    pub constructors: Vec<String>,
    /// The `ilang.abi` metadata (B.6).
    pub abi_json: String,
}
