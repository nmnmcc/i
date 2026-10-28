use super::*;
use crate::ir::{Expr, Location, Statement};
use gleam_core::ast::BinOp;
use num_bigint::{BigInt, Sign};
use num_traits::ToPrimitive;

impl Backend<'_> {
    pub(super) fn expression(
        &mut self,
        expression: &Expr,
        ctx: &mut Context,
        tail: bool,
    ) -> Result<(), Error> {
        match expression {
            Expr::Nil => ctx.emit(Instruction::RefNull(EQ)),
            Expr::Bool(value) => {
                ctx.emit(Instruction::I32Const(i32::from(*value)));
                self.call("bool_new", ctx)?;
            }
            Expr::Int(value) => self.integer(value, ctx)?,
            Expr::Float(value) => {
                ctx.emit(Instruction::F64Const((*value).into()));
                self.call("float_new", ctx)?;
            }
            Expr::String(value) => self.string(value, ctx)?,
            Expr::Var(name) => {
                ctx.emit(Instruction::LocalGet(*ctx.variables.get(name).ok_or_else(
                    || Error::Codegen(format!("unbound variable {name}")),
                )?))
            }
            Expr::Constant(name) => ctx.emit(Instruction::Call(
                self.constants
                    .get(name)
                    .ok_or_else(|| Error::Codegen(format!("unknown constant {name}")))?
                    .0,
            )),
            Expr::Function(name) => {
                let arity = self.find_function(name)?.arguments.len();
                self.closure(self.function_index(name)?, arity, &[], ctx);
            }
            Expr::Constructor { tag, arity } => {
                let index = self
                    .constructors
                    .get(tag)
                    .copied()
                    .ok_or_else(|| Error::Codegen(format!("unknown constructor {tag}")))?;
                self.closure(index, *arity, &[], ctx);
            }
            Expr::Lambda {
                args,
                captures,
                body,
            } => {
                let name = match &self.current_location {
                    Some(location) => format!(
                        "{}::lambda{} ({})",
                        self.current_name,
                        self.lambda_count,
                        location_text(location)
                    ),
                    None => format!("{}::lambda{}", self.current_name, self.lambda_count),
                };
                let index = self.reserve(self.types.call, name);
                if let Some(location) = &self.current_location {
                    self.source_locations.insert(index, location.clone());
                }
                self.lambda_count += 1;
                let captured = captures
                    .iter()
                    .map(|name| {
                        ctx.variables
                            .get(name)
                            .copied()
                            .ok_or_else(|| Error::Codegen(format!("unbound capture {name}")))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let body = self.function_body(args, captures, body)?;
                self.define(index, body);
                self.closure(index, args.len(), &captured, ctx);
            }
            Expr::Call { fun, args } => match fun.as_ref() {
                Expr::Function(name) => {
                    ctx.emit(Instruction::RefNull(EQ));
                    self.arguments(args, ctx)?;
                    let target = self.function_index(name)?;
                    ctx.emit(if tail {
                        Instruction::ReturnCall(target)
                    } else {
                        Instruction::Call(target)
                    });
                }
                Expr::Constructor { tag, .. } => {
                    self.record_prefix(*tag, ctx)?;
                    self.arguments(args, ctx)?;
                    self.call("record", ctx)?;
                }
                _ => {
                    let callable = ctx.local(EQREF);
                    self.expression(fun, ctx, false)?;
                    ctx.emit(Instruction::LocalSet(callable));
                    self.field_get(callable, 5, ctx);
                    self.arguments(args, ctx)?;
                    self.field_get(callable, 7, ctx);
                    ctx.emit(Instruction::RefAsNonNull);
                    ctx.emit(if tail {
                        Instruction::ReturnCallRef(self.types.call)
                    } else {
                        Instruction::CallRef(self.types.call)
                    });
                }
            },
            Expr::Block(statements) => {
                for (index, statement) in statements.iter().enumerate() {
                    let last = index + 1 == statements.len();
                    match statement {
                        Statement::Expr(expression) => {
                            self.expression(expression, ctx, last && tail)?;
                            if !last {
                                ctx.emit(Instruction::Drop);
                            }
                        }
                        Statement::Let {
                            pattern,
                            value,
                            message,
                            location,
                            ..
                        } => {
                            // A final irrefutable binding returns its value and
                            // has no observable work after that evaluation.
                            if last && tail && irrefutable_binding(pattern) {
                                self.expression(value, ctx, true)?;
                                continue;
                            }
                            let local = ctx.local(EQREF);
                            self.expression(value, ctx, false)?;
                            ctx.emit(Instruction::LocalSet(local));
                            let done = ctx.push_block(BlockType::Empty);
                            let failed = ctx.push_block(BlockType::Empty);
                            self.pattern(pattern, local, failed, ctx)?;
                            ctx.br(done);
                            ctx.end();
                            self.failure(message.as_ref(), location, "Pattern match failed", ctx)?;
                            ctx.emit(Instruction::Drop);
                            ctx.end();
                            if last {
                                ctx.emit(Instruction::LocalGet(local));
                            }
                        }
                        Statement::Assert {
                            value,
                            message,
                            location,
                        } => {
                            self.expression(value, ctx, false)?;
                            self.call("bool_get", ctx)?;
                            ctx.emit(Instruction::I32Eqz);
                            ctx.push_if(BlockType::Empty);
                            self.failure(message.as_ref(), location, "Assertion failed", ctx)?;
                            ctx.emit(Instruction::Drop);
                            ctx.end();
                            if last {
                                ctx.emit(Instruction::RefNull(EQ));
                            }
                        }
                    }
                }
                if statements.is_empty() {
                    ctx.emit(Instruction::RefNull(EQ));
                }
            }
            Expr::Tuple(elements) => {
                self.arguments(elements, ctx)?;
                self.call("tuple", ctx)?;
            }
            Expr::List { elements, tail } => {
                let mut locals = Vec::with_capacity(elements.len());
                for element in elements {
                    self.expression(element, ctx, false)?;
                    let local = ctx.local(EQREF);
                    ctx.emit(Instruction::LocalSet(local));
                    locals.push(local);
                }
                let rest = ctx.local(EQREF);
                if let Some(tail) = tail {
                    self.expression(tail, ctx, false)?;
                } else {
                    self.call("list_nil", ctx)?;
                }
                ctx.emit(Instruction::LocalSet(rest));
                for local in locals.into_iter().rev() {
                    ctx.emit(Instruction::LocalGet(local));
                    ctx.emit(Instruction::LocalGet(rest));
                    self.call("list_cons", ctx)?;
                    ctx.emit(Instruction::LocalSet(rest));
                }
                ctx.emit(Instruction::LocalGet(rest));
            }
            Expr::Record { tag, args } => {
                self.record_prefix(*tag, ctx)?;
                self.arguments(args, ctx)?;
                self.call("record", ctx)?;
            }
            Expr::Field { record, index } => {
                self.expression(record, ctx, false)?;
                ctx.emit(Instruction::I32Const(*index as i32));
                self.call("field", ctx)?;
            }
            Expr::RecordUpdate { record, updates } => {
                let value = ctx.local(EQREF);
                let array = ctx.local(reference(self.types.values, true));
                self.expression(record, ctx, false)?;
                ctx.emit(Instruction::LocalSet(value));
                self.field_get(value, 5, ctx);
                ctx.emit(Instruction::ArrayLen);
                ctx.emit(Instruction::ArrayNewDefault(self.types.values));
                ctx.emit(Instruction::LocalSet(array));
                ctx.emit(Instruction::LocalGet(array));
                ctx.emit(Instruction::I32Const(0));
                self.field_get(value, 5, ctx);
                ctx.emit(Instruction::I32Const(0));
                ctx.emit(Instruction::LocalGet(array));
                ctx.emit(Instruction::ArrayLen);
                ctx.emit(Instruction::ArrayCopy {
                    array_type_index_dst: self.types.values,
                    array_type_index_src: self.types.values,
                });
                for (index, expression) in updates {
                    ctx.emit(Instruction::LocalGet(array));
                    ctx.emit(Instruction::I32Const(*index as i32));
                    self.expression(expression, ctx, false)?;
                    ctx.emit(Instruction::ArraySet(self.types.values));
                }
                self.field_get(value, 1, ctx);
                self.field_get(value, 2, ctx);
                ctx.emit(Instruction::LocalGet(array));
                ctx.emit(Instruction::RefAsNonNull);
                self.call("record", ctx)?;
            }
            Expr::BinOp { op, lhs, rhs } => self.binary(*op, lhs, rhs, ctx, tail)?,
            Expr::NegateBool(value) => {
                self.expression(value, ctx, false)?;
                self.call("bool_get", ctx)?;
                ctx.emit(Instruction::I32Eqz);
                self.call("bool_new", ctx)?;
            }
            Expr::NegateInt(value) => {
                self.expression(value, ctx, false)?;
                self.call("int_neg", ctx)?;
            }
            Expr::Case { subjects, clauses } => {
                let mut locals = Vec::with_capacity(subjects.len());
                for subject in subjects {
                    self.expression(subject, ctx, false)?;
                    let local = ctx.local(EQREF);
                    ctx.emit(Instruction::LocalSet(local));
                    locals.push(local);
                }
                let done = ctx.push_block(BlockType::Result(EQREF));
                for clause in clauses {
                    let rejected = ctx.push_block(BlockType::Empty);
                    let matched = ctx.push_block(BlockType::Empty);
                    for alternative in &clause.alternatives {
                        let next = ctx.push_block(BlockType::Empty);
                        for (pattern, local) in alternative.iter().zip(&locals) {
                            self.pattern(pattern, *local, next, ctx)?;
                        }
                        if let Some(guard) = &clause.guard {
                            self.expression(guard, ctx, false)?;
                            self.call("bool_get", ctx)?;
                            ctx.emit(Instruction::I32Eqz);
                            ctx.br_if(next);
                        }
                        ctx.br(matched);
                        ctx.end();
                    }
                    ctx.br(rejected);
                    ctx.end();
                    self.expression(&clause.body, ctx, tail)?;
                    ctx.br(done);
                    ctx.end();
                }
                self.string("Non-exhaustive runtime pattern match", ctx)?;
                self.call("panic", ctx)?;
                ctx.end();
            }
            Expr::Panic { message, location } => {
                self.failure(Some(message), location, "Panic", ctx)?
            }
            Expr::Echo {
                value,
                message,
                location,
            } => {
                let local = ctx.local(EQREF);
                self.expression(value, ctx, false)?;
                ctx.emit(Instruction::LocalSet(local));
                if let Some(message) = message {
                    self.expression(message, ctx, false)?;
                    ctx.emit(Instruction::I32Const(2));
                    self.call("write_string", ctx)?;
                }
                self.string(&format!("{}\n", location_text(location)), ctx)?;
                ctx.emit(Instruction::I32Const(2));
                self.call("write_string", ctx)?;
                ctx.emit(Instruction::LocalGet(local));
                self.call("echo", ctx)?;
            }
            Expr::BitArray(segments) => self.bit_array(segments, ctx)?,
        }
        Ok(())
    }
    fn arguments(&mut self, arguments: &[Expr], ctx: &mut Context) -> Result<(), Error> {
        for argument in arguments {
            self.expression(argument, ctx, false)?;
        }
        ctx.emit(Instruction::ArrayNewFixed {
            array_type_index: self.types.values,
            array_size: arguments.len() as u32,
        });
        Ok(())
    }
    fn closure(&self, index: u32, arity: usize, captures: &[u32], ctx: &mut Context) {
        ctx.emit(Instruction::I32Const(7));
        ctx.emit(Instruction::I32Const(index as i32));
        ctx.emit(Instruction::I64Const(arity as i64));
        ctx.emit(Instruction::F64Const(0.0.into()));
        ctx.emit(Instruction::RefNull(heap(self.types.bytes)));
        for local in captures {
            ctx.emit(Instruction::LocalGet(*local));
        }
        ctx.emit(Instruction::ArrayNewFixed {
            array_type_index: self.types.values,
            array_size: captures.len() as u32,
        });
        ctx.emit(Instruction::RefNull(heap(self.types.limbs)));
        ctx.emit(Instruction::RefFunc(index));
        ctx.emit(Instruction::StructNew(self.types.obj));
    }
    fn record_prefix(&self, tag: u32, ctx: &mut Context) -> Result<(), Error> {
        let constructor = self
            .program
            .constructors
            .iter()
            .find(|constructor| constructor.tag == tag)
            .ok_or_else(|| Error::Codegen(format!("unknown constructor {tag}")))?;
        ctx.emit(Instruction::I32Const(tag as i32));
        ctx.emit(Instruction::I64Const(constructor.type_id as i64));
        Ok(())
    }
    fn field_get(&self, local: u32, index: u32, ctx: &mut Context) {
        ctx.emit(Instruction::LocalGet(local));
        ctx.emit(Instruction::RefCastNonNull(heap(self.types.obj)));
        ctx.emit(Instruction::StructGet {
            struct_type_index: self.types.obj,
            field_index: index,
        });
    }
    pub(super) fn byte_array(&mut self, bytes: &[u8], ctx: &mut Context) -> Result<(), Error> {
        if bytes.len() <= 16 {
            for byte in bytes {
                ctx.emit(Instruction::I32Const(i32::from(*byte)));
            }
            ctx.emit(Instruction::ArrayNewFixed {
                array_type_index: self.types.bytes,
                array_size: bytes.len() as u32,
            });
        } else {
            let index = self.runtime.data_count + self.addition.data.len();
            self.addition.data.passive(bytes.iter().copied());
            ctx.emit(Instruction::I32Const(0));
            ctx.emit(Instruction::I32Const(bytes.len() as i32));
            ctx.emit(Instruction::ArrayNewData {
                array_type_index: self.types.bytes,
                array_data_index: index,
            });
        }
        Ok(())
    }
    pub(super) fn string(&mut self, value: &str, ctx: &mut Context) -> Result<(), Error> {
        self.byte_array(value.as_bytes(), ctx)?;
        self.call("string_from_bytes", ctx)
    }
    pub(super) fn integer(&self, value: &BigInt, ctx: &mut Context) -> Result<(), Error> {
        if let Some(value) = value
            .to_i32()
            .filter(|value| (-1073741824..1073741824).contains(value))
        {
            ctx.emit(Instruction::I32Const(value));
            ctx.emit(Instruction::RefI31);
            return Ok(());
        }
        let (sign, limbs) = value.to_u32_digits();
        ctx.emit(Instruction::I32Const(match sign {
            Sign::Minus => -1,
            Sign::NoSign => 0,
            Sign::Plus => 1,
        }));
        for limb in &limbs {
            ctx.emit(Instruction::I32Const(*limb as i32));
        }
        ctx.emit(Instruction::ArrayNewFixed {
            array_type_index: self.types.limbs,
            array_size: limbs.len() as u32,
        });
        self.call("int_pack", ctx)
    }
    fn failure(
        &mut self,
        message: Option<&Expr>,
        location: &Location,
        default: &str,
        ctx: &mut Context,
    ) -> Result<(), Error> {
        self.string(&format!("{}: ", location_text(location)), ctx)?;
        if let Some(message) = message {
            self.expression(message, ctx, false)?;
        } else {
            self.string(default, ctx)?;
        }
        self.call("string_append", ctx)?;
        self.call("panic", ctx)
    }
    fn binary(
        &mut self,
        op: BinOp,
        lhs: &Expr,
        rhs: &Expr,
        ctx: &mut Context,
        tail: bool,
    ) -> Result<(), Error> {
        use BinOp::*;
        self.expression(lhs, ctx, false)?;
        if matches!(op, And | Or) {
            self.call("bool_get", ctx)?;
            ctx.push_if(BlockType::Result(EQREF));
            if op == And {
                self.expression(rhs, ctx, tail)?;
            } else {
                ctx.emit(Instruction::I32Const(1));
                self.call("bool_new", ctx)?;
            }
            ctx.else_();
            if op == Or {
                self.expression(rhs, ctx, tail)?;
            } else {
                ctx.emit(Instruction::I32Const(0));
                self.call("bool_new", ctx)?;
            }
            ctx.end();
            return Ok(());
        }
        if matches!(op, LtFloat | LtEqFloat | GtFloat | GtEqFloat) {
            self.call("float_get", ctx)?;
            self.expression(rhs, ctx, false)?;
            self.call("float_get", ctx)?;
            ctx.emit(match op {
                LtFloat => Instruction::F64Lt,
                LtEqFloat => Instruction::F64Le,
                GtFloat => Instruction::F64Gt,
                _ => Instruction::F64Ge,
            });
            return self.call("bool_new", ctx);
        }
        self.expression(rhs, ctx, false)?;
        match op {
            Eq | NotEq => {
                self.call("value_eq", ctx)?;
                if op == NotEq {
                    ctx.emit(Instruction::I32Eqz);
                }
                self.call("bool_new", ctx)?;
            }
            LtInt | LtEqInt | GtInt | GtEqInt => {
                self.call("int_compare", ctx)?;
                ctx.emit(Instruction::I32Const(0));
                ctx.emit(match op {
                    LtInt => Instruction::I32LtS,
                    LtEqInt => Instruction::I32LeS,
                    GtInt => Instruction::I32GtS,
                    _ => Instruction::I32GeS,
                });
                self.call("bool_new", ctx)?;
            }
            _ => {
                let name = match op {
                    AddInt => "int_add",
                    SubInt => "int_sub",
                    MultInt => "int_mul",
                    DivInt => "int_div",
                    RemainderInt => "int_rem",
                    AddFloat => "float_add",
                    SubFloat => "float_sub",
                    MultFloat => "float_mul",
                    DivFloat => "float_div",
                    Concatenate => "string_append",
                    _ => {
                        return Err(Error::Codegen(format!(
                            "unsupported binary operator {op:?}"
                        )));
                    }
                };
                self.call(name, ctx)?;
            }
        }
        Ok(())
    }
}

pub(super) fn location_text(location: &Location) -> String {
    if location.line > 0 {
        format!("{}:{}:{}", location.module, location.line, location.column)
    } else {
        format!("{}:{}", location.module, location.start)
    }
}

fn irrefutable_binding(pattern: &crate::ir::Pattern) -> bool {
    match pattern {
        crate::ir::Pattern::Discard | crate::ir::Pattern::Variable(_) => true,
        crate::ir::Pattern::Alias { pattern, .. } => irrefutable_binding(pattern),
        _ => false,
    }
}
