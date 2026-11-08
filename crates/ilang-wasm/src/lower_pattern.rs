//! Lower checked patterns, guards and bit-array options without carrying the
//! frontend's inference structures into code generation.

use std::collections::BTreeMap;

use ilang_core::{
    analyse::Inferred,
    ast::{
        AssignName, BitArrayOption, BitArraySize, ClauseGuard, Pattern as AstPattern,
        TypedBitArraySize, TypedClauseGuard, TypedConstant, TypedExpr, TypedPattern,
    },
    strings::convert_string_escape_chars,
    type_::ValueConstructorVariant,
};

use super::Lowerer;
use crate::{
    Error,
    ir::{BitKind, BitOptions, BitSegment, Expr, Pattern},
};

impl Lowerer<'_> {
    pub(super) fn pattern(
        &mut self,
        pattern: &TypedPattern,
        reuse: bool,
    ) -> Result<Pattern, Error> {
        let outer = self
            .pattern_outer_bindings
            .as_ref()
            .unwrap_or(&self.locals)
            .clone();
        let mut bit_bindings = outer.clone();
        self.pattern_in_scope(pattern, reuse, &outer, &mut bit_bindings)
    }

    fn pattern_in_scope(
        &mut self,
        pattern: &TypedPattern,
        reuse: bool,
        outer: &BTreeMap<String, String>,
        bit_bindings: &mut BTreeMap<String, String>,
    ) -> Result<Pattern, Error> {
        Ok(match pattern {
            AstPattern::Int { int_value, .. } => Pattern::Int(int_value.clone()),
            AstPattern::Float {
                float_value,
                location,
                ..
            } => {
                let value = float_value.value();
                if !value.is_finite() {
                    return Err(Error::Lowering(format!(
                        "{}:{}: a float pattern must be finite",
                        self.module, location.start,
                    )));
                }
                Pattern::Float(value)
            }
            AstPattern::String { value, .. } => {
                Pattern::String(convert_string_escape_chars(value).to_string())
            }
            AstPattern::Variable { name, .. } => {
                Pattern::Variable(self.pattern_binding(name, reuse, bit_bindings)?)
            }
            AstPattern::Discard { .. } => Pattern::Discard,
            AstPattern::Assign { name, pattern, .. } => {
                let pattern =
                    Box::new(self.pattern_in_scope(pattern, reuse, outer, bit_bindings)?);
                let name = self.pattern_binding(name, reuse, bit_bindings)?;
                Pattern::Alias { name, pattern }
            }
            AstPattern::Tuple { elements, .. } => Pattern::Tuple(
                elements
                    .iter()
                    .map(|element| self.pattern_in_scope(element, reuse, outer, bit_bindings))
                    .collect::<Result<_, _>>()?,
            ),
            AstPattern::List { elements, tail, .. } => Pattern::List {
                elements: elements
                    .iter()
                    .map(|element| self.pattern_in_scope(element, reuse, outer, bit_bindings))
                    .collect::<Result<_, _>>()?,
                tail: tail
                    .as_ref()
                    .map(|tail| {
                        self.pattern_in_scope(&tail.pattern, reuse, outer, bit_bindings)
                            .map(Box::new)
                    })
                    .transpose()?,
            },
            AstPattern::Constructor {
                constructor,
                arguments,
                ..
            } => {
                let Inferred::Known(constructor) = constructor else {
                    return Err(Error::Lowering(format!(
                        "{}: unresolved constructor in a checked pattern",
                        self.module
                    )));
                };
                if constructor.module.is_empty() || constructor.module == "i" {
                    match constructor.name.as_str() {
                        "True" => return Ok(Pattern::Bool(true)),
                        "False" => return Ok(Pattern::Bool(false)),
                        "Nil" => return Ok(Pattern::Nil),
                        _ => {}
                    }
                }
                let descriptor = self.constructor(&constructor.module, &constructor.name)?;
                let tag = descriptor.tag;
                let arity = descriptor.arity;
                // The type checker fills spread arguments and reorders labelled
                // fields. A short checked argument vector is an invalid AST.
                if arguments.len() != arity {
                    return Err(Error::Lowering(format!(
                        "{}: constructor {}.{} pattern has {} fields, expected {arity}",
                        self.module,
                        constructor.module,
                        constructor.name,
                        arguments.len(),
                    )));
                }
                Pattern::Record {
                    tag,
                    arguments: arguments
                        .iter()
                        .map(|argument| {
                            self.pattern_in_scope(&argument.value, reuse, outer, bit_bindings)
                        })
                        .collect::<Result<_, _>>()?,
                }
            }
            AstPattern::StringPrefix {
                left_side_string,
                left_side_assignment,
                right_side_assignment,
                ..
            } => {
                let prefix = convert_string_escape_chars(left_side_string).to_string();
                let left = left_side_assignment
                    .as_ref()
                    .map(|assignment| self.pattern_binding(&assignment.name, reuse, bit_bindings))
                    .transpose()?;
                let rest = match right_side_assignment {
                    AssignName::Variable(name) => {
                        Some(self.pattern_binding(name, reuse, bit_bindings)?)
                    }
                    AssignName::Discard(_) => None,
                };
                Pattern::StringPrefix { prefix, left, rest }
            }
            AstPattern::BitArray { segments, .. } => {
                // Only bindings from this bit-array pattern are visible in its
                // sizes. Entering another bit-array also ends that size scope,
                // including when it is nested inside the current pattern.
                *bit_bindings = outer.clone();
                let mut lowered = Vec::with_capacity(segments.len());
                for segment in segments {
                    // Size expressions are evaluated in the scope before this
                    // segment introduces bindings, but after preceding segments.
                    let pattern_locals = std::mem::replace(&mut self.locals, bit_bindings.clone());
                    let options = self.bit_options_pattern(&segment.options);
                    self.locals = pattern_locals;
                    let options = options?;
                    let value =
                        self.pattern_in_scope(&segment.value, reuse, outer, bit_bindings)?;
                    lowered.push(BitSegment { value, options });
                }
                Pattern::BitArray(lowered)
            }
            AstPattern::BitArraySize(size) => Pattern::Value(Box::new(self.bit_size(size)?)),
            AstPattern::Invalid { location, .. } => {
                return Err(Error::Lowering(format!(
                    "{}:{}: invalid typed pattern reached code generation",
                    self.module, location.start
                )));
            }
        })
    }

    fn pattern_binding(
        &mut self,
        name: &str,
        reuse: bool,
        bit_bindings: &mut BTreeMap<String, String>,
    ) -> Result<String, Error> {
        let renamed = if reuse {
            let renamed = self
                .reused_pattern_bindings
                .as_ref()
                .and_then(|bindings| bindings.get(name))
                .cloned()
                .ok_or_else(|| {
                    Error::Lowering(format!(
                        "{}: alternate pattern introduces an unbound variable {name}",
                        self.module
                    ))
                })?;
            self.locals.insert(name.to_owned(), renamed.clone());
            renamed
        } else {
            self.bind(name)
        };
        bit_bindings.insert(name.to_owned(), renamed.clone());
        Ok(renamed)
    }

    pub(super) fn guard(&mut self, guard: &TypedClauseGuard) -> Result<Expr, Error> {
        Ok(match guard {
            ClauseGuard::Block { value, .. } => self.guard(value)?,
            ClauseGuard::BinaryOperator {
                operator,
                left,
                right,
                ..
            } => Expr::BinOp {
                op: *operator,
                lhs: Box::new(self.guard(left)?),
                rhs: Box::new(self.guard(right)?),
            },
            ClauseGuard::Not { expression, .. } => {
                Expr::NegateBool(Box::new(self.guard(expression)?))
            }
            ClauseGuard::Var { name, .. } => Expr::Var(self.local(name)?),
            ClauseGuard::TupleIndex { index, tuple, .. } => Expr::Field {
                record: Box::new(self.guard(tuple)?),
                index: usize::try_from(*index)
                    .map_err(|_| Error::Lowering("tuple index exceeds addressable range".into()))?,
            },
            ClauseGuard::FieldAccess {
                index,
                label,
                container,
                ..
            } => {
                let index = index.ok_or_else(|| {
                    Error::Lowering(format!(
                        "{}: unresolved record field {label} in a guard",
                        self.module
                    ))
                })?;
                Expr::Field {
                    record: Box::new(self.guard(container)?),
                    index: usize::try_from(index).map_err(|_| {
                        Error::Lowering("record field index exceeds addressable range".into())
                    })?,
                }
            }
            ClauseGuard::ModuleSelect { literal, .. } | ClauseGuard::Constant(literal) => {
                self.constant(literal)?
            }
            ClauseGuard::Invalid { location, .. } => {
                return Err(Error::Lowering(format!(
                    "{}:{}: invalid typed guard reached code generation",
                    self.module, location.start
                )));
            }
        })
    }

    pub(super) fn bit_options_expr(
        &mut self,
        options: &[BitArrayOption<TypedExpr>],
    ) -> Result<BitOptions, Error> {
        self.bit_options(options, |this, value| this.expr(value))
    }

    pub(super) fn bit_options_constant(
        &mut self,
        options: &[BitArrayOption<TypedConstant>],
    ) -> Result<BitOptions, Error> {
        self.bit_options(options, |this, value| this.constant(value))
    }

    pub(super) fn bit_options_pattern(
        &mut self,
        options: &[BitArrayOption<TypedPattern>],
    ) -> Result<BitOptions, Error> {
        self.bit_options(options, |this, value| this.pattern_size(value))
    }

    fn bit_options<T>(
        &mut self,
        options: &[BitArrayOption<T>],
        mut size: impl FnMut(&mut Self, &T) -> Result<Expr, Error>,
    ) -> Result<BitOptions, Error> {
        let mut result = BitOptions::default();
        let mut unit = None;
        for option in options {
            match option {
                BitArrayOption::Bytes { .. } => result.kind = BitKind::Bytes,
                BitArrayOption::Int { .. } => result.kind = BitKind::Int,
                BitArrayOption::Float { .. } => result.kind = BitKind::Float,
                BitArrayOption::Bits { .. } => result.kind = BitKind::Bits,
                BitArrayOption::Utf8 { .. } => result.kind = BitKind::Utf8,
                BitArrayOption::Utf16 { .. } => result.kind = BitKind::Utf16,
                BitArrayOption::Utf32 { .. } => result.kind = BitKind::Utf32,
                BitArrayOption::Utf8Codepoint { .. } => result.kind = BitKind::Utf8Codepoint,
                BitArrayOption::Utf16Codepoint { .. } => result.kind = BitKind::Utf16Codepoint,
                BitArrayOption::Utf32Codepoint { .. } => result.kind = BitKind::Utf32Codepoint,
                BitArrayOption::Signed { .. } => result.signed = true,
                BitArrayOption::Unsigned { .. } => result.signed = false,
                BitArrayOption::Big { .. } => result.little_endian = false,
                BitArrayOption::Little { .. } | BitArrayOption::Native { .. } => {
                    result.little_endian = true
                }
                BitArrayOption::Size { value, .. } => {
                    result.size = Some(Box::new(size(self, value)?))
                }
                BitArrayOption::Unit { value, .. } => unit = Some(*value),
            }
        }
        result.unit = unit.unwrap_or(if result.kind == BitKind::Bytes { 8 } else { 1 });
        Ok(result)
    }

    fn pattern_size(&mut self, pattern: &TypedPattern) -> Result<Expr, Error> {
        match pattern {
            AstPattern::BitArraySize(size) => self.bit_size(size),
            AstPattern::Int { int_value, .. } => Ok(Expr::Int(int_value.clone())),
            AstPattern::Variable { name, .. } => Ok(Expr::Var(self.local(name)?)),
            other => Err(Error::Lowering(format!(
                "{}:{}: invalid bit-array size expression",
                self.module,
                other.location().start
            ))),
        }
    }

    fn bit_size(&mut self, size: &TypedBitArraySize) -> Result<Expr, Error> {
        Ok(match size {
            BitArraySize::Int { int_value, .. } => Expr::Int(int_value.clone()),
            BitArraySize::Block { inner, .. } => self.bit_size(inner)?,
            BitArraySize::BinaryOperator {
                operator,
                left,
                right,
                ..
            } => Expr::BinOp {
                op: operator.to_bin_op(),
                lhs: Box::new(self.bit_size(left)?),
                rhs: Box::new(self.bit_size(right)?),
            },
            BitArraySize::Variable {
                name, constructor, ..
            } => match constructor.as_ref().map(|constructor| &constructor.variant) {
                Some(ValueConstructorVariant::ModuleConstant { literal, .. }) => {
                    self.constant(literal)?
                }
                Some(
                    ValueConstructorVariant::ModuleFn { .. }
                    | ValueConstructorVariant::Record { .. },
                ) => {
                    return Err(Error::Lowering(format!(
                        "{}: non-integer constructor {name} used as bit-array size",
                        self.module
                    )));
                }
                Some(ValueConstructorVariant::LocalVariable { .. }) | None => {
                    Expr::Var(self.local(name)?)
                }
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use ilang_core::{
        ast::{BitArraySegment, IntOperator},
        type_::{self, error::VariableOrigin},
    };

    use super::*;

    fn variable(name: &str) -> TypedPattern {
        AstPattern::Variable {
            location: Default::default(),
            name: name.into(),
            type_: type_::int(),
            origin: VariableOrigin::generated(),
        }
    }

    fn size_variable(name: &str) -> TypedBitArraySize {
        BitArraySize::Variable {
            location: Default::default(),
            name: name.into(),
            constructor: None,
            type_: type_::int(),
        }
    }

    fn segment(
        name: &str,
        size: TypedBitArraySize,
    ) -> BitArraySegment<TypedPattern, std::sync::Arc<type_::Type>> {
        BitArraySegment {
            location: Default::default(),
            value: Box::new(variable(name)),
            options: vec![BitArrayOption::Size {
                location: Default::default(),
                value: Box::new(AstPattern::BitArraySize(size)),
                short_form: false,
            }],
            type_: type_::int(),
        }
    }

    #[test]
    fn byte_units_are_independent_of_option_order_and_native_is_little_endian() {
        let constructors = BTreeMap::new();
        let constants = BTreeSet::new();
        let mut lowerer = Lowerer::new("test", &constructors, &constants);
        let bytes = BitArrayOption::Bytes {
            location: Default::default(),
        };
        let unit = BitArrayOption::Unit {
            location: Default::default(),
            value: 3,
        };
        let native = BitArrayOption::Native {
            location: Default::default(),
        };

        let default_bytes = lowerer
            .bit_options_pattern(std::slice::from_ref(&bytes))
            .unwrap();
        assert_eq!(default_bytes.unit, 8);
        let before = lowerer
            .bit_options_pattern(&[unit.clone(), bytes.clone(), native.clone()])
            .unwrap();
        let after = lowerer.bit_options_pattern(&[bytes, native, unit]).unwrap();
        assert_eq!(before.unit, 3);
        assert_eq!(after.unit, 3);
        assert!(before.little_endian && after.little_endian);
    }

    #[test]
    fn bit_sizes_resolve_before_the_current_binding_in_primary_and_alternative_patterns() {
        let constructors = BTreeMap::new();
        let constants = BTreeSet::new();
        let mut lowerer = Lowerer::new("test", &constructors, &constants);
        let outer_x = lowerer.bind("x");
        let outer = lowerer.locals.clone();
        let pattern = AstPattern::BitArray {
            location: Default::default(),
            segments: vec![
                segment(
                    "x",
                    BitArraySize::BinaryOperator {
                        location: Default::default(),
                        operator: IntOperator::Add,
                        left: Box::new(size_variable("x")),
                        right: Box::new(BitArraySize::Int {
                            location: Default::default(),
                            value: "1".into(),
                            int_value: 1.into(),
                        }),
                    },
                ),
                segment("y", size_variable("x")),
            ],
        };

        let primary = lowerer.pattern(&pattern, false).unwrap();
        let primary_bindings = lowerer.locals.clone();
        let inner_x = primary_bindings["x"].clone();
        assert_ne!(outer_x, inner_x);
        lowerer.locals = outer;
        lowerer.reused_pattern_bindings = Some(primary_bindings);
        let alternative = lowerer.pattern(&pattern, true).unwrap();

        for pattern in [primary, alternative] {
            let Pattern::BitArray(segments) = pattern else {
                panic!("expected bit-array pattern")
            };
            let Expr::BinOp { lhs, .. } = segments[0].options.size.as_deref().unwrap() else {
                panic!("expected computed size")
            };
            assert!(matches!(lhs.as_ref(), Expr::Var(name) if name == &outer_x));
            assert!(matches!(&segments[0].value, Pattern::Variable(name) if name == &inner_x));
            assert!(
                matches!(segments[1].options.size.as_deref(), Some(Expr::Var(name)) if name == &inner_x)
            );
        }
    }

    #[test]
    fn bit_sizes_ignore_sibling_patterns_but_preserve_all_result_bindings() {
        let constructors = BTreeMap::new();
        let constants = BTreeSet::new();
        let mut lowerer = Lowerer::new("test", &constructors, &constants);
        let outer_x = lowerer.bind("x");
        let outer_y = lowerer.bind("y");
        let pattern = AstPattern::Tuple {
            location: Default::default(),
            elements: vec![
                variable("x"),
                AstPattern::BitArray {
                    location: Default::default(),
                    segments: vec![segment("y", size_variable("x"))],
                },
                AstPattern::BitArray {
                    location: Default::default(),
                    segments: vec![segment("z", size_variable("y"))],
                },
            ],
        };

        let Pattern::Tuple(patterns) = lowerer.pattern(&pattern, false).unwrap() else {
            panic!("expected tuple pattern");
        };
        for (pattern, expected_size) in patterns[1..].iter().zip([outer_x.clone(), outer_y.clone()])
        {
            let Pattern::BitArray(segments) = pattern else {
                panic!("expected bit-array pattern")
            };
            assert!(
                matches!(segments[0].options.size.as_deref(), Some(Expr::Var(name)) if name == &expected_size)
            );
        }
        assert_ne!(lowerer.local("x").unwrap(), outer_x);
        assert_ne!(lowerer.local("y").unwrap(), outer_y);
        assert!(lowerer.local("z").is_ok());
    }

    #[test]
    fn string_prefix_unescapes_and_reuses_both_aliases_in_alternatives() {
        let constructors = BTreeMap::new();
        let constants = BTreeSet::new();
        let mut lowerer = Lowerer::new("test", &constructors, &constants);
        let pattern = AstPattern::StringPrefix {
            location: Default::default(),
            left_location: Default::default(),
            right_location: Default::default(),
            left_side_string: "\\u{1F984}\\n".into(),
            left_side_assignment: Some(ilang_core::ast::StringPrefixLeftSideAssignment {
                name: "prefix".into(),
                name_start_position: 0,
                location: Default::default(),
            }),
            right_side_assignment: AssignName::Variable("suffix".into()),
        };
        let primary = lowerer.pattern(&pattern, false).unwrap();
        let bindings = lowerer.locals.clone();
        lowerer.locals.clear();
        lowerer.reused_pattern_bindings = Some(bindings.clone());
        let alternative = lowerer.pattern(&pattern, true).unwrap();

        for pattern in [primary, alternative] {
            let Pattern::StringPrefix { prefix, left, rest } = pattern else {
                panic!("expected prefix pattern")
            };
            assert_eq!(prefix, "🦄\n");
            assert_eq!(left.as_ref(), bindings.get("prefix"));
            assert_eq!(rest.as_ref(), bindings.get("suffix"));
        }
    }

    #[test]
    fn unresolved_guard_fields_fail_instead_of_reading_field_zero() {
        let constructors = BTreeMap::new();
        let constants = BTreeSet::new();
        let mut lowerer = Lowerer::new("test", &constructors, &constants);
        lowerer.bind("record");
        let guard = ClauseGuard::FieldAccess {
            label_location: Default::default(),
            index: None,
            label: "missing".into(),
            type_: type_::int(),
            container: Box::new(ClauseGuard::Var {
                location: Default::default(),
                type_: type_::int(),
                name: "record".into(),
                definition_location: Default::default(),
                origin: VariableOrigin::generated(),
            }),
        };
        assert!(
            matches!(lowerer.guard(&guard), Err(Error::Lowering(message)) if message.contains("unresolved record field missing"))
        );
    }
}
