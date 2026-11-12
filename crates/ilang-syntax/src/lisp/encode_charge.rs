//! Streaming preflight for a written source item. This walks the exact datum
//! shape produced by `encode`, without constructing Syntax or copying atoms.
use super::{Datum, DatumKind};
use crate::ast::*;
use num_bigint::{BigInt, BigUint};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxCharge {
    EnterFrame(u64),
    Charge { work: u64, words: u64 },
    LeaveFrame,
}

#[derive(Clone, Copy)]
enum Task<'a> {
    Leave,
    Atom(&'a str),
    Int(&'a BigUint),
    SignedInt(&'a BigInt),
    Float,
    Datum(&'a Datum),
    Item(&'a Item, usize),
    AttrHead(&'a Attribute),
    Head(&'a Ident, &'a [Ident]),
    TypeHead(&'a TypeDef),
    Constructor(&'a Constructor),
    Field(&'a Field),
    Binding(&'a Binding),
    BindingHead(&'a Binding),
    BindingNameHead(&'a Binding),
    Expr(&'a Expr),
    Pattern(&'a Pattern),
    Type(&'a TypeExpr),
    TypeParam(&'a FnParamType),
    Param(&'a Param),
    Result(&'a TypeExpr, Option<&'a TypeExpr>, Option<&'a TypeExpr>),
    ResultError(&'a TypeExpr, Option<&'a TypeExpr>),
    Block(&'a Block),
    Statement(&'a Stmt),
    ClauseList(&'a [Clause]),
    Clause(&'a Clause),
    ClauseHead(&'a Clause),
    ClausePatterns(&'a Clause),
    Arg(&'a Arg),
    SegmentExpr(&'a Segment<Expr>),
    SegmentPattern(&'a Segment<Pattern>),
    Format(&'a Format),
    RecordField(&'a Ident, &'a Expr),
    RecordTypeField(&'a RecordLabel, &'a TypeExpr),
    RecordRemoved(&'a RecordLabel),
    Qualified(Option<&'a Ident>, &'a Ident),
    StringPrefix(&'a str, &'a Binder),
    BracketParams(&'a [Param]),
    BracketTypeParams(&'a [FnParamType]),
    Constructors(&'a [Constructor]),
}

fn event<E>(
    f: &mut impl FnMut(SyntaxCharge) -> Result<(), E>,
    work: u64,
    words: u64,
) -> Result<(), E> {
    f(SyntaxCharge::Charge { work, words })
}
fn enter<E>(f: &mut impl FnMut(SyntaxCharge) -> Result<(), E>) -> Result<(), E> {
    f(SyntaxCharge::EnterFrame(1))?;
    event(f, 1, 80)
}
fn atom<E>(
    f: &mut impl FnMut(SyntaxCharge) -> Result<(), E>,
    bytes: u64,
    integer: bool,
) -> Result<(), E> {
    enter(f)?;
    if integer {
        let limbs = bytes.div_ceil(4);
        event(f, limbs.saturating_mul(limbs), 0)?;
    }
    let bytes = bytes.saturating_mul(12);
    event(f, bytes, 4 + bytes.div_ceil(8))?;
    f(SyntaxCharge::LeaveFrame)
}
fn bracket<'a, E>(
    f: &mut impl FnMut(SyntaxCharge) -> Result<(), E>,
    work: &mut Vec<Task<'a>>,
    count: usize,
) -> Result<(), E> {
    enter(f)?;
    let n = count as u64;
    event(f, n, 4 + n.saturating_mul(8))?;
    event(f, 0, 4)?;
    work.push(Task::Leave);
    Ok(())
}
fn quote<'a, E>(
    f: &mut impl FnMut(SyntaxCharge) -> Result<(), E>,
    work: &mut Vec<Task<'a>>,
) -> Result<(), E> {
    enter(f)?;
    event(f, 1, 8)?;
    event(f, 0, 4)?;
    work.push(Task::Leave);
    Ok(())
}

/// Visit the exact ordered charges of the Syntax that encoding `item` gives.
/// No Syntax nodes or atom copies are constructed; after each group charge
/// succeeds, only that group's pending traversal tasks are placed on the stack.
pub(super) fn item<E>(
    item: &Item,
    mut f: impl FnMut(SyntaxCharge) -> Result<(), E>,
) -> Result<(), E> {
    use Task::*;
    let mut work = vec![Item(item, 0)];
    while let Some(task) = work.pop() {
        match task {
            Leave => f(SyntaxCharge::LeaveFrame)?,
            StringPrefix(prefix, rest) => {
                bracket(&mut f, &mut work, 3)?;
                work.push(Atom(&rest.name));
                work.push(Atom(prefix));
                work.push(Atom("<>"));
            }
            Atom(text) => atom(&mut f, text.len() as u64, false)?,
            Int(value) => atom(&mut f, value.bits().div_ceil(8), true)?,
            SignedInt(value) => atom(&mut f, value.magnitude().bits().div_ceil(8), true)?,
            Float => atom(&mut f, 8, false)?,
            Datum(d) => match &d.kind {
                DatumKind::Name(s)
                | DatumKind::Discard(s)
                | DatumKind::UpName(s)
                | DatumKind::Symbol(s)
                | DatumKind::String(s) => atom(&mut f, s.len() as u64, false)?,
                DatumKind::Int(n) => atom(&mut f, n.bits().div_ceil(8), true)?,
                DatumKind::Float(_) => atom(&mut f, 8, false)?,
                DatumKind::Round(xs)
                | DatumKind::Square(xs)
                | DatumKind::Curly(xs)
                | DatumKind::Bits(xs) => {
                    bracket(&mut f, &mut work, xs.len())?;
                    work.extend(xs.iter().rev().map(Datum));
                }
                DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => {
                    quote(&mut f, &mut work)?;
                    work.push(Datum(x));
                }
            },
            Item(item, index) => {
                if let Some(attr) = item.attributes.get(index) {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Item(item, index + 1));
                    work.push(AttrHead(attr));
                    work.push(Atom("@"));
                } else {
                    match &item.kind {
                        ItemKind::Binding(b) => work.push(Binding(b)),
                        ItemKind::Declaration {
                            name,
                            type_params,
                            ty,
                        } => {
                            bracket(&mut f, &mut work, 3)?;
                            work.push(Type(ty));
                            work.push(Head(name, type_params));
                            work.push(Atom(":"));
                        }
                        ItemKind::TypeDef(t) => match &t.body {
                            TypeBody::Alias(ty) => {
                                bracket(&mut f, &mut work, 3)?;
                                work.push(Type(ty));
                                work.push(TypeHead(t));
                                work.push(Atom("="));
                            }
                            TypeBody::Custom(cs) => {
                                bracket(&mut f, &mut work, 2)?;
                                work.push(Constructors(cs));
                                work.push(TypeHead(t));
                            }
                        },
                        ItemKind::MacroItem { module, name, args } => {
                            bracket(&mut f, &mut work, 1 + args.len())?;
                            work.extend(args.iter().rev().map(Arg));
                            work.push(Qualified(module.as_ref(), name));
                        }
                    }
                }
            }
            AttrHead(attr) => {
                if let Some((args, _)) = &attr.call {
                    bracket(&mut f, &mut work, 1 + args.len())?;
                    work.extend(args.iter().rev().map(Arg));
                    work.push(Qualified(attr.module.as_ref(), &attr.name));
                } else if !attr.args.is_empty() {
                    bracket(&mut f, &mut work, 1 + attr.args.len())?;
                    work.extend(attr.args.iter().rev().map(|(s, _)| Atom(s)));
                    work.push(Qualified(attr.module.as_ref(), &attr.name));
                } else {
                    work.push(Qualified(attr.module.as_ref(), &attr.name));
                }
            }
            Head(name, params) => {
                if params.is_empty() {
                    work.push(Atom(&name.name));
                } else {
                    bracket(&mut f, &mut work, 1 + params.len())?;
                    work.extend(params.iter().rev().map(|p| Atom(&p.name)));
                    work.push(Atom(&name.name));
                }
            }
            TypeHead(t) => {
                if t.head.is_empty() {
                    work.push(Head(&t.name, &t.params));
                } else {
                    bracket(&mut f, &mut work, 1 + t.head.len())?;
                    work.extend(t.head.iter().rev().map(Type));
                    work.push(Atom(&t.name.name));
                }
            }
            Constructors(cs) => {
                bracket(&mut f, &mut work, cs.len())?;
                work.extend(cs.iter().rev().map(Constructor));
            }
            Constructor(c) => {
                if c.fields.is_empty() {
                    work.push(Atom(&c.name.name));
                } else {
                    bracket(&mut f, &mut work, 1 + c.fields.len())?;
                    work.extend(c.fields.iter().rev().map(Field));
                    work.push(Atom(&c.name.name));
                }
            }
            Field(field) => {
                if let Some(label) = &field.label {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(&field.ty));
                    work.push(Atom(&label.name));
                    work.push(Atom(":"));
                } else {
                    work.push(Type(&field.ty));
                }
            }
            Binding(b) => {
                bracket(&mut f, &mut work, 3)?;
                work.push(Expr(&b.value));
                work.push(BindingHead(b));
                work.push(Atom("="));
            }
            BindingHead(b) => {
                if b.type_params.is_empty() {
                    work.push(Pattern(&b.pattern));
                } else if let PatternKind::Typed { ty, .. } = &b.pattern.kind {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(ty));
                    work.push(BindingNameHead(b));
                    work.push(Atom(":"));
                } else {
                    work.push(BindingNameHead(b));
                }
            }
            BindingNameHead(b) => {
                let name = match &b.pattern.kind {
                    PatternKind::Var(s) => s.as_str(),
                    PatternKind::Typed { binder, .. } => binder.name.as_str(),
                    _ => panic!("type parameters on a non-name binding"),
                };
                bracket(&mut f, &mut work, 1 + b.type_params.len())?;
                work.extend(b.type_params.iter().rev().map(|p| Atom(&p.name)));
                work.push(Atom(name));
            }
            Qualified(module, name) => {
                if let Some(module) = module {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Atom(&name.name));
                    work.push(Atom(&module.name));
                    work.push(Atom("."));
                } else {
                    work.push(Atom(&name.name));
                }
            }
            Arg(a) => {
                if let Some(label) = &a.label {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Expr(&a.value));
                    work.push(Atom(&label.name));
                    work.push(Atom(":"));
                } else {
                    work.push(Expr(&a.value));
                }
            }
            RecordField(name, value) => {
                bracket(&mut f, &mut work, 3)?;
                work.push(Expr(value));
                work.push(Atom(&name.name));
                work.push(Atom(":"));
            }
            RecordTypeField(name, value) => {
                bracket(&mut f, &mut work, 3)?;
                work.push(Type(value));
                work.push(Atom(&name.name));
                work.push(Atom(":"));
            }
            RecordRemoved(name) => {
                bracket(&mut f, &mut work, 2)?;
                work.push(Atom(&name.name));
                work.push(Atom("-"));
            }
            Expr(e) => match &e.kind {
                ExprKind::Quote(d) => work.push(Datum(d)),
                ExprKind::Int(n) => work.push(Int(n)),
                ExprKind::Float(_) => work.push(Float),
                ExprKind::String(s) => work.push(Atom(s)),
                ExprKind::Name { module, name } => work.push(Qualified(module.as_ref(), name)),
                ExprKind::Record { base, fields } => {
                    bracket(
                        &mut f,
                        &mut work,
                        fields.len() + usize::from(base.is_some()) * 2,
                    )?;
                    work.extend(fields.iter().rev().map(|(n, v)| RecordField(n, v)));
                    if let Some(base) = base {
                        work.push(Expr(base));
                        work.push(Atom(".."));
                    }
                }
                ExprKind::Tuple(xs) if xs.is_empty() => bracket(&mut f, &mut work, 0)?,
                ExprKind::Tuple(xs) => {
                    bracket(&mut f, &mut work, 1 + xs.len())?;
                    work.extend(xs.iter().rev().map(Expr));
                    work.push(Atom(","));
                }
                ExprKind::List { items, rest } => {
                    bracket(
                        &mut f,
                        &mut work,
                        items.len() + usize::from(rest.is_some()) * 2,
                    )?;
                    if let Some(r) = rest {
                        work.push(Expr(r));
                        work.push(Atom(".."));
                    }
                    work.extend(items.iter().rev().map(Expr));
                }
                ExprKind::BitArray(xs) => {
                    bracket(&mut f, &mut work, xs.len())?;
                    work.extend(xs.iter().rev().map(SegmentExpr));
                }
                ExprKind::Block(b) => match b.stmts.as_slice() {
                    [Stmt::Expr(inner)] => work.push(Expr(inner)),
                    _ => work.push(Block(b)),
                },
                ExprKind::Fn(function) => {
                    bracket(&mut f, &mut work, 4)?;
                    work.push(Block(&function.body));
                    if let Some(ret) = &function.ret {
                        work.push(Result(&ret.value, ret.err.as_ref(), ret.env.as_ref()));
                    } else {
                        work.push(Atom("_"));
                    }
                    work.push(BracketParams(&function.params));
                    work.push(Atom("->"));
                }
                ExprKind::Case { subject, clauses } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(ClauseList(clauses));
                    work.push(Expr(subject));
                    work.push(Atom("."));
                }
                ExprKind::Clauses(clauses) => work.push(ClauseList(clauses)),
                ExprKind::Call { callee, args } => {
                    bracket(&mut f, &mut work, 1 + args.len())?;
                    work.extend(args.iter().rev().map(Arg));
                    work.push(Expr(callee));
                }
                ExprKind::Field { target, name } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Atom(&name.name));
                    work.push(Expr(target));
                    work.push(Atom("."));
                }
                ExprKind::TupleIndex { target, index, .. } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Int(index));
                    work.push(Expr(target));
                    work.push(Atom("."));
                }
                ExprKind::RecordUpdate {
                    constructor,
                    base,
                    fields,
                } => {
                    bracket(&mut f, &mut work, 3 + fields.len())?;
                    work.extend(fields.iter().rev().map(|(n, e)| RecordField(n, e)));
                    work.push(Expr(base));
                    work.push(Atom(".."));
                    work.push(Expr(constructor));
                }
                ExprKind::Unary { op, expr } => {
                    bracket(&mut f, &mut work, 2)?;
                    work.push(Expr(expr));
                    work.push(Atom(match op {
                        UnOp::Neg => "-",
                        UnOp::NegFloat => "-.",
                    }));
                }
                ExprKind::Binary { op, lhs, rhs } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Expr(rhs));
                    work.push(Expr(lhs));
                    work.push(Atom(if *op == BinOp::Pipe { "|" } else { op.text() }));
                }
            },
            BracketParams(params) => {
                bracket(&mut f, &mut work, params.len())?;
                work.extend(params.iter().rev().map(Param));
            }
            Param(param) => match param {
                crate::ast::Param::Pattern(p) => work.push(Pattern(p)),
                crate::ast::Param::Env { name, ty } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(ty));
                    work.push(Atom(&name.name));
                    work.push(Atom("!"));
                }
            },
            Block(block) => {
                bracket(&mut f, &mut work, block.stmts.len())?;
                work.extend(block.stmts.iter().rev().map(Statement));
            }
            Statement(stmt) => match stmt {
                Stmt::Bind(b) => work.push(Binding(b)),
                Stmt::Env { ty, value, .. } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Expr(value));
                    work.push(Type(ty));
                    work.push(Atom("!"));
                }
                Stmt::Expr(e) => work.push(Expr(e)),
            },
            ClauseList(clauses) => {
                bracket(&mut f, &mut work, clauses.len())?;
                work.extend(clauses.iter().rev().map(Clause));
            }
            Clause(c) => {
                bracket(&mut f, &mut work, 3)?;
                work.push(Expr(&c.body));
                work.push(ClauseHead(c));
                work.push(Atom("->"));
            }
            ClauseHead(c) => {
                if let Some(guard) = &c.guard {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Expr(guard));
                    work.push(ClausePatterns(c));
                    work.push(Atom("?"));
                } else {
                    work.push(ClausePatterns(c));
                }
            }
            ClausePatterns(c) => {
                if c.patterns.len() == 1 {
                    work.push(Pattern(&c.patterns[0]));
                } else {
                    bracket(&mut f, &mut work, 1 + c.patterns.len())?;
                    work.extend(c.patterns.iter().rev().map(Pattern));
                    work.push(Atom("|"));
                }
            }
            Pattern(p) => match &p.kind {
                PatternKind::Quote { datum, .. } => work.push(Datum(datum)),
                PatternKind::Var(s) | PatternKind::Discard(s) | PatternKind::String(s) => {
                    work.push(Atom(s))
                }
                PatternKind::Typed { binder, ty } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(ty));
                    work.push(Atom(&binder.name));
                    work.push(Atom(":"));
                }
                PatternKind::Named { name, pattern } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Pattern(pattern));
                    work.push(Atom(&name.name));
                    work.push(Atom("="));
                }
                PatternKind::Int { negative, value } => {
                    if *negative {
                        bracket(&mut f, &mut work, 2)?;
                        work.push(Int(value));
                        work.push(Atom("-"));
                    } else {
                        work.push(Int(value));
                    }
                }
                PatternKind::Float { negative, .. } => {
                    if *negative {
                        bracket(&mut f, &mut work, 2)?;
                        work.push(Float);
                        work.push(Atom("-."));
                    } else {
                        work.push(Float);
                    }
                }
                PatternKind::Prefix {
                    prefix,
                    suffix,
                    rest,
                } => {
                    if suffix.is_empty() {
                        work.push(StringPrefix(prefix, rest));
                    } else {
                        bracket(&mut f, &mut work, 3)?;
                        work.push(Atom(suffix));
                        if prefix.is_empty() {
                            work.push(Atom(&rest.name));
                        } else {
                            work.push(StringPrefix(prefix, rest));
                        }
                        work.push(Atom("<>"));
                    }
                }
                PatternKind::Constructor {
                    module,
                    name,
                    args,
                    spread,
                } => {
                    if args.is_empty() && !spread {
                        work.push(Qualified(module.as_ref(), name));
                    } else {
                        bracket(&mut f, &mut work, 1 + args.len() + usize::from(*spread))?;
                        if *spread {
                            work.push(Atom(".."));
                        }
                        work.extend(args.iter().rev().map(Pattern));
                        work.push(Qualified(module.as_ref(), name));
                    }
                }
                PatternKind::Tuple(xs) if xs.is_empty() => bracket(&mut f, &mut work, 0)?,
                PatternKind::Tuple(xs) => {
                    bracket(&mut f, &mut work, 1 + xs.len())?;
                    work.extend(xs.iter().rev().map(Pattern));
                    work.push(Atom(","));
                }
                PatternKind::List { items, rest } => {
                    let extra = match rest {
                        None => 0,
                        Some(None) => 1,
                        Some(Some(_)) => 2,
                    };
                    bracket(&mut f, &mut work, items.len() + extra)?;
                    if let Some(rest) = rest {
                        if let Some(binder) = rest {
                            work.push(Atom(&binder.name));
                        }
                        work.push(Atom(".."));
                    }
                    work.extend(items.iter().rev().map(Pattern));
                }
                PatternKind::BitArray(xs) => {
                    bracket(&mut f, &mut work, xs.len())?;
                    work.extend(xs.iter().rev().map(SegmentPattern));
                }
            },
            SegmentExpr(segment) => {
                if let Some(format) = &segment.format {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Format(format));
                    work.push(Expr(&segment.value));
                    work.push(Atom(":"));
                } else {
                    work.push(Expr(&segment.value));
                }
            }
            SegmentPattern(segment) => {
                if let Some(format) = &segment.format {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Format(format));
                    work.push(Pattern(&segment.value));
                    work.push(Atom(":"));
                } else {
                    work.push(Pattern(&segment.value));
                }
            }
            Format(format) => {
                if let Some(arg) = &format.arg {
                    bracket(&mut f, &mut work, 2)?;
                    work.push(Expr(arg));
                    work.push(Atom(&format.name.name));
                } else {
                    work.push(Atom(&format.name.name));
                }
            }
            Type(t) => match &t.kind {
                TypeKind::Lit(LitValue::Str(s)) => work.push(Atom(s)),
                TypeKind::Lit(LitValue::Int(n)) => {
                    if n.sign() == num_bigint::Sign::Minus {
                        bracket(&mut f, &mut work, 2)?;
                        work.push(SignedInt(n));
                        work.push(Atom("-"));
                    } else {
                        work.push(SignedInt(n));
                    }
                }
                TypeKind::Named { module, name, args } => {
                    if args.is_empty() {
                        work.push(Qualified(module.as_ref(), name));
                    } else {
                        bracket(&mut f, &mut work, 1 + args.len())?;
                        work.extend(args.iter().rev().map(Type));
                        work.push(Qualified(module.as_ref(), name));
                    }
                }
                TypeKind::ValueOf { module, name } => work.push(Qualified(module.as_ref(), name)),
                TypeKind::Call { module, name, args } => {
                    bracket(&mut f, &mut work, 1 + args.len())?;
                    work.extend(args.iter().rev().map(Type));
                    work.push(Qualified(module.as_ref(), name));
                }
                TypeKind::Hole => work.push(Atom("_")),
                TypeKind::Record {
                    base,
                    fields,
                    removed,
                } => {
                    bracket(
                        &mut f,
                        &mut work,
                        fields.len() + removed.len() + usize::from(base.is_some()) * 2,
                    )?;
                    let mut fields = fields.iter().rev().peekable();
                    let mut removed = removed.iter().rev().peekable();
                    while fields.peek().is_some() || removed.peek().is_some() {
                        if removed.peek().is_some_and(|n| {
                            fields
                                .peek()
                                .is_none_or(|(field, _)| n.span.start > field.span.start)
                        }) {
                            work.push(RecordRemoved(removed.next().unwrap()));
                        } else {
                            let (n, value) = fields.next().unwrap();
                            work.push(RecordTypeField(n, value));
                        }
                    }
                    if let Some(base) = base {
                        work.push(Type(base));
                        work.push(Atom(".."));
                    }
                }
                TypeKind::Tuple(ts)
                | TypeKind::Union(ts)
                | TypeKind::Inter(ts)
                | TypeKind::Diff(ts) => {
                    let head = match &t.kind {
                        TypeKind::Tuple(_) => ",",
                        TypeKind::Union(_) => "|",
                        TypeKind::Inter(_) => "&",
                        TypeKind::Diff(_) => "-",
                        _ => unreachable!(),
                    };
                    bracket(&mut f, &mut work, 1 + ts.len())?;
                    work.extend(ts.iter().rev().map(Type));
                    work.push(Atom(head));
                }
                TypeKind::Fn {
                    params,
                    ret,
                    err,
                    env,
                } => {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Result(ret, err.as_deref(), env.as_deref()));
                    work.push(BracketTypeParams(params));
                    work.push(Atom("->"));
                }
            },
            BracketTypeParams(params) => {
                bracket(&mut f, &mut work, params.len())?;
                work.extend(params.iter().rev().map(TypeParam));
            }
            TypeParam(param) => {
                if let Some(label) = &param.label {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(&param.ty));
                    work.push(Atom(&label.name));
                    work.push(Atom(":"));
                } else {
                    work.push(Type(&param.ty));
                }
            }
            Result(value, err, env) => {
                if let Some(env) = env {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(env));
                    work.push(ResultError(value, err));
                    work.push(Atom("!"));
                } else {
                    work.push(ResultError(value, err));
                }
            }
            ResultError(value, err) => {
                if let Some(err) = err {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Type(err));
                    work.push(Type(value));
                    work.push(Atom("^"));
                } else if matches!(value.kind, TypeKind::Hole) {
                    bracket(&mut f, &mut work, 3)?;
                    work.push(Atom("Never"));
                    work.push(Type(value));
                    work.push(Atom("^"));
                } else {
                    work.push(Type(value));
                }
            }
        }
    }
    Ok(())
}
