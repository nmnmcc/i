//! Reachability-driven lowering of the type-checked i syntax tree.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
    sync::Arc,
};

use ilang_core::{
    analyse::Inferred,
    ast::{self, Constant, TypedConstant, TypedExpr, TypedFunction, TypedModule, TypedStatement},
    strings::convert_string_escape_chars,
    type_::{ModuleValueConstructor, ValueConstructor, ValueConstructorVariant},
};
use src_span::{LineNumbers, SrcSpan};

use crate::{
    Error,
    ir::{self, Constructor, Expr, Function, Location, Pattern, Program, Statement},
};

#[path = "ir_cache.rs"]
mod cache;
#[path = "optimize.rs"]
mod optimize;
#[path = "lower_pattern.rs"]
mod patterns;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    /// Reachable definitions reused without lowering their typed trees.
    pub hits: usize,
    pub lowered: usize,
    /// Newly lowered definitions successfully persisted to their module file.
    pub writes: usize,
}

/// Lower only reachable functions and constants. Constructor identities are
/// assigned over the entire checked project, so removing an unreachable function
/// cannot change an existing record's representation.
pub fn lower(modules: &[Arc<TypedModule>], roots: &[String]) -> Result<Program> {
    lower_impl(modules, roots, None).map(|(program, _)| program)
}

/// Reuse module IR when its typed tree, constructor registry and compiler ABI
/// are unchanged. Cache failures degrade to normal lowering.
pub fn lower_cached(
    modules: &[Arc<TypedModule>],
    roots: &[String],
    cache_dir: &Path,
    compiler_fingerprint: &str,
) -> Result<(Program, CacheStats)> {
    lower_impl(modules, roots, Some((cache_dir, compiler_fingerprint)))
}

fn lower_impl(
    modules: &[Arc<TypedModule>],
    roots: &[String],
    cache_options: Option<(&Path, &str)>,
) -> Result<(Program, CacheStats)> {
    let constructors = constructor_registry(modules)?;
    let external_types: BTreeSet<_> = modules
        .iter()
        .flat_map(|module| {
            module
                .definitions
                .custom_types
                .iter()
                .filter(|custom| custom.constructors.is_empty())
                .map(|custom| symbol(&module.name, &custom.name))
        })
        .collect();
    let mut cache = cache_options.and_then(|(directory, fingerprint)| {
        cache::Cache::new(directory, fingerprint, &constructors, &external_types).ok()
    });
    let mut stats = CacheStats::default();
    let mut functions = BTreeMap::new();
    let mut constants = BTreeMap::new();
    for module in modules {
        for function in &module.definitions.functions {
            let name = function.name.as_ref().ok_or_else(|| {
                Error::Lowering(format!("unnamed module function in {}", module.name))
            })?;
            let key = symbol(&module.name, &name.1);
            if functions
                .insert(key.clone(), (module.as_ref(), function))
                .is_some()
            {
                return Err(Error::Lowering(format!("duplicate function {key}")));
            }
        }
        for constant in &module.definitions.constants {
            let key = symbol(&module.name, &constant.name);
            if constants
                .insert(key.clone(), (module.as_ref(), constant.value.as_ref()))
                .is_some()
            {
                return Err(Error::Lowering(format!("duplicate constant {key}")));
            }
        }
    }
    for root in roots {
        if !functions.contains_key(root) {
            return Err(Error::Lowering(format!(
                "entry or export function {root:?} does not exist"
            )));
        }
    }

    let constant_names = constants.keys().cloned().collect::<BTreeSet<_>>();
    let mut queue: VecDeque<_> = roots.iter().cloned().collect();
    let mut visited = BTreeSet::new();
    let mut lowered_functions = BTreeMap::new();
    let mut lowered_constants = BTreeMap::new();
    while let Some(key) = queue.pop_front() {
        if !visited.insert(key.clone()) {
            continue;
        }
        if let Some((module, function)) = functions.get(&key) {
            let cached = cached_definition(&mut cache, module, &key);
            let lowered = match cached {
                Some(cache::Definition::Function(function)) if function.symbol == key => {
                    stats.hits += 1;
                    function
                }
                _ => {
                    let mut lowerer = Lowerer::new(&module.name, &constructors, &constant_names)
                        .with_lines(&module.type_info.line_numbers);
                    let mut lowered = lowerer.function(&key, function)?;
                    lowered.body = optimize::expression(lowered.body);
                    stats.lowered += 1;
                    cache_definition(
                        &mut cache,
                        module,
                        key.clone(),
                        cache::Definition::Function(lowered.clone()),
                    );
                    lowered
                }
            };
            queue.extend(optimize::references(&lowered.body));
            lowered_functions.insert(key, lowered);
        } else if let Some((module, constant)) = constants.get(&key) {
            let cached = cached_definition(&mut cache, module, &key);
            let lowered = match cached {
                Some(cache::Definition::Constant(constant)) => {
                    stats.hits += 1;
                    constant
                }
                _ => {
                    let mut lowerer = Lowerer::new(&module.name, &constructors, &constant_names)
                        .with_lines(&module.type_info.line_numbers);
                    let lowered = optimize::expression(lowerer.constant(constant)?);
                    stats.lowered += 1;
                    cache_definition(
                        &mut cache,
                        module,
                        key.clone(),
                        cache::Definition::Constant(lowered.clone()),
                    );
                    lowered
                }
            };
            queue.extend(optimize::references(&lowered));
            lowered_constants.insert(key, lowered);
        } else {
            return Err(Error::Lowering(format!(
                "referenced definition {key:?} is missing from the checked project"
            )));
        }
    }
    if let Some(mut cache) = cache {
        stats.writes = cache.flush();
    }
    Ok((
        Program {
            functions: lowered_functions.into_values().collect(),
            constants: lowered_constants,
            constructors: constructors.into_values().collect(),
            external_types,
        },
        stats,
    ))
}

fn cached_definition(
    cache: &mut Option<cache::Cache>,
    module: &TypedModule,
    key: &str,
) -> Option<cache::Definition> {
    match cache.as_mut()?.get(module, key) {
        Ok(definition) => definition,
        Err(_) => {
            // An unfamiliar serialisation format must never reject a program.
            // Disable this cache session rather than retrying every definition.
            *cache = None;
            None
        }
    }
}

fn cache_definition(
    cache: &mut Option<cache::Cache>,
    module: &TypedModule,
    key: String,
    definition: cache::Definition,
) {
    if cache
        .as_mut()
        .is_some_and(|cache| cache.insert(module, key, definition).is_err())
    {
        *cache = None;
    }
}

pub fn constructor_registry(modules: &[Arc<TypedModule>]) -> Result<BTreeMap<String, Constructor>> {
    let mut descriptions = BTreeMap::new();
    let mut types = BTreeSet::new();
    for module in modules {
        for custom in &module.definitions.custom_types {
            let type_name = symbol(&module.name, &custom.name);
            types.insert(type_name.clone());
            for constructor in &custom.constructors {
                let key = symbol(&module.name, &constructor.name);
                let value = (
                    constructor.name.to_string(),
                    type_name.clone(),
                    constructor.arguments.len(),
                );
                if descriptions.insert(key.clone(), value).is_some() {
                    return Err(Error::Lowering(format!("duplicate constructor {key}")));
                }
            }
        }
    }
    let type_ids: BTreeMap<_, _> = types
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, index as u32 + 32))
        .collect();
    let mut result = BTreeMap::new();
    for (name, tag) in [("Ok", 0), ("Error", 1)] {
        let key = symbol("i", name);
        result.insert(
            key.clone(),
            Constructor {
                symbol: key,
                name: name.into(),
                type_name: "i.Result".into(),
                tag,
                type_id: 0,
                arity: 1,
            },
        );
    }
    for (index, (key, (name, type_name, arity))) in descriptions.into_iter().enumerate() {
        let type_id = type_ids[&type_name];
        result.insert(
            key.clone(),
            Constructor {
                symbol: key,
                name,
                type_name,
                tag: index as u32 + 32,
                type_id,
                arity,
            },
        );
    }
    Ok(result)
}

pub(super) struct Lowerer<'a> {
    pub(super) module: String,
    pub(super) locals: BTreeMap<String, String>,
    pub(super) constructors: &'a BTreeMap<String, Constructor>,
    pub(super) next_local: u64,
    pub(super) reused_pattern_bindings: Option<BTreeMap<String, String>>,
    pub(super) pattern_outer_bindings: Option<BTreeMap<String, String>>,
    constant_names: &'a BTreeSet<String>,
    lines: Option<&'a LineNumbers>,
}

impl<'a> Lowerer<'a> {
    fn new(
        module: &str,
        constructors: &'a BTreeMap<String, Constructor>,
        constant_names: &'a BTreeSet<String>,
    ) -> Self {
        Self {
            module: module.into(),
            locals: BTreeMap::new(),
            constructors,
            next_local: 0,
            reused_pattern_bindings: None,
            pattern_outer_bindings: None,
            constant_names,
            lines: None,
        }
    }

    fn with_lines(mut self, lines: &'a LineNumbers) -> Self {
        self.lines = Some(lines);
        self
    }

    fn bind(&mut self, name: &str) -> String {
        let binding = format!("v{}_{}", self.next_local, name);
        self.next_local += 1;
        self.locals.insert(name.into(), binding.clone());
        binding
    }

    fn local(&self, name: &str) -> Result<String> {
        self.locals
            .get(name)
            .cloned()
            .ok_or_else(|| Error::Lowering(format!("unbound local {name:?} in {}", self.module)))
    }

    fn constructor(&self, module: &str, name: &str) -> Result<&Constructor> {
        let module = if module.is_empty() { "i" } else { module };
        self.constructors
            .get(&symbol(module, name))
            .ok_or_else(|| Error::Lowering(format!("unknown record constructor {module}.{name}")))
    }

    fn location(&self, location: SrcSpan) -> Location {
        let coordinates = self
            .lines
            .map(|lines| lines.line_and_utf16_column_number(location.start));
        Location {
            module: self.module.clone(),
            start: location.start,
            end: location.end,
            line: coordinates
                .as_ref()
                .map_or(0, |coordinates| coordinates.line),
            column: coordinates
                .as_ref()
                .map_or(0, |coordinates| coordinates.column),
        }
    }

    fn function(&mut self, key: &str, function: &TypedFunction) -> Result<Function> {
        let arguments = function
            .arguments
            .iter()
            .enumerate()
            .map(|(i, argument)| {
                self.bind(
                    argument
                        .get_variable_name()
                        .map(|name| name.as_str())
                        .unwrap_or(&format!("discard_{i}")),
                )
            })
            .collect();
        let argument_types = function
            .arguments
            .iter()
            .map(|argument| argument.type_.clone())
            .collect();
        let external = function
            .external_wasm
            .as_ref()
            .map(|(module, name, _)| (module.to_string(), name.to_string()));
        let body = if external.is_some() {
            Expr::Nil
        } else {
            self.statements(&function.body)?
        };
        Ok(Function {
            symbol: key.into(),
            arguments,
            argument_types,
            return_type: function.return_type.clone(),
            body,
            external,
            location: self.location(function.location),
        })
    }

    fn statements(&mut self, statements: &[TypedStatement]) -> Result<Expr> {
        let mut result = Vec::with_capacity(statements.len());
        for statement in statements {
            match statement {
                ast::Statement::Expression(expression) => {
                    result.push(Statement::Expr(self.expr(expression)?))
                }
                ast::Statement::Use(use_) => result.push(Statement::Expr(self.expr(&use_.call)?)),
                ast::Statement::Assert(assertion) => result.push(Statement::Assert {
                    value: self.expr(&assertion.value)?,
                    message: assertion
                        .message
                        .as_ref()
                        .map(|message| self.expr(message))
                        .transpose()?,
                    location: self.location(assertion.location),
                }),
                ast::Statement::Assignment(assignment) => {
                    let value = self.expr(&assignment.value)?;
                    let message = match &assignment.kind {
                        ast::AssignmentKind::Assert { message, .. } => message
                            .as_ref()
                            .map(|message| self.expr(message))
                            .transpose()?,
                        _ => None,
                    };
                    let pattern = self.pattern(&assignment.pattern, false)?;
                    result.push(Statement::Let {
                        value,
                        pattern,
                        assert: assignment.kind.is_assert(),
                        message,
                        location: self.location(assignment.location),
                    });
                }
            }
        }
        if result.is_empty() {
            result.push(Statement::Expr(Expr::Nil));
        }
        Ok(Expr::Block(result))
    }

    fn expr(&mut self, expression: &TypedExpr) -> Result<Expr> {
        Ok(match expression {
            TypedExpr::Int { int_value, .. } => Expr::Int(int_value.clone()),
            TypedExpr::Float { float_value, .. } => self.float(float_value.value())?,
            TypedExpr::String { value, .. } => {
                Expr::String(convert_string_escape_chars(value).to_string())
            }
            TypedExpr::Var {
                constructor, name, ..
            } => self.value_constructor(constructor, name)?,
            TypedExpr::ModuleSelect {
                module_name,
                label,
                constructor,
                ..
            } => match constructor {
                ModuleValueConstructor::Fn { module, name, .. } => self.function_ref(module, name),
                ModuleValueConstructor::Constant { literal, .. } => {
                    self.constant_ref_or_literal(module_name, label, literal)?
                }
                ModuleValueConstructor::Record { name, arity, .. } => {
                    self.record_constructor(module_name, name, *arity as usize)?
                }
            },
            TypedExpr::Block { statements, .. } => {
                let old_locals = self.locals.clone();
                let result = self.statements(statements)?;
                self.locals = old_locals;
                result
            }
            TypedExpr::Pipeline {
                first_value,
                assignments,
                finally,
                ..
            } => {
                let old_locals = self.locals.clone();
                let mut result = Vec::new();
                let mut previous = None;
                for assignment in
                    std::iter::once(first_value).chain(assignments.iter().map(|(value, _)| value))
                {
                    let value = self.pipeline_expression(&assignment.value, previous.as_deref())?;
                    if matches!(
                        assignment.value.as_ref(),
                        TypedExpr::Echo {
                            expression: None,
                            ..
                        }
                    ) {
                        result.push(Statement::Expr(value));
                    } else {
                        let variable = self.bind(&assignment.name);
                        result.push(Statement::Let {
                            pattern: Pattern::Variable(variable.clone()),
                            value,
                            assert: false,
                            message: None,
                            location: self.location(assignment.location),
                        });
                        previous = Some(variable);
                    }
                }
                result.push(Statement::Expr(
                    self.pipeline_expression(finally, previous.as_deref())?,
                ));
                self.locals = old_locals;
                Expr::Block(result)
            }
            TypedExpr::Fn {
                arguments, body, ..
            } => {
                let old_locals = self.locals.clone();
                let args: Vec<_> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, argument)| {
                        self.bind(
                            argument
                                .get_variable_name()
                                .map(|name| name.as_str())
                                .unwrap_or(&format!("discard_{i}")),
                        )
                    })
                    .collect();
                let body = self.statements(body)?;
                let captures = free_variables(&body, &args);
                self.locals = old_locals;
                Expr::Lambda {
                    args,
                    captures,
                    body: Box::new(body),
                }
            }
            TypedExpr::List { elements, tail, .. } => Expr::List {
                elements: elements
                    .iter()
                    .map(|element| self.expr(element))
                    .collect::<Result<_>>()?,
                tail: tail
                    .as_ref()
                    .map(|tail| self.expr(tail).map(Box::new))
                    .transpose()?,
            },
            TypedExpr::Call { fun, arguments, .. } => {
                let function = self.expr(fun)?;
                let args = arguments
                    .iter()
                    .map(|argument| self.expr(&argument.value))
                    .collect::<Result<_>>()?;
                call(function, args)
            }
            TypedExpr::BinOp {
                operator,
                left,
                right,
                ..
            } => Expr::BinOp {
                op: *operator,
                lhs: Box::new(self.expr(left)?),
                rhs: Box::new(self.expr(right)?),
            },
            TypedExpr::Case {
                subjects, clauses, ..
            } => {
                let subjects = subjects
                    .iter()
                    .map(|subject| self.expr(subject))
                    .collect::<Result<_>>()?;
                let outer = self.locals.clone();
                let mut lowered = Vec::with_capacity(clauses.len());
                for clause in clauses {
                    self.locals = outer.clone();
                    self.pattern_outer_bindings = Some(outer.clone());
                    let first = clause
                        .pattern
                        .iter()
                        .map(|pattern| self.pattern(pattern, false))
                        .collect::<Result<_>>()?;
                    let primary = self.locals.clone();
                    let mut alternatives = vec![first];
                    for patterns in &clause.alternative_patterns {
                        self.locals = outer.clone();
                        self.reused_pattern_bindings = Some(primary.clone());
                        alternatives.push(
                            patterns
                                .iter()
                                .map(|pattern| self.pattern(pattern, true))
                                .collect::<Result<_>>()?,
                        );
                    }
                    self.reused_pattern_bindings = None;
                    self.pattern_outer_bindings = None;
                    self.locals = primary;
                    let guard = clause
                        .guard
                        .as_ref()
                        .map(|guard| self.guard(guard))
                        .transpose()?;
                    let body = self.expr(&clause.then)?;
                    lowered.push(ir::Clause {
                        alternatives,
                        guard,
                        body,
                    });
                }
                self.locals = outer;
                Expr::Case {
                    subjects,
                    clauses: lowered,
                }
            }
            TypedExpr::RecordAccess { record, index, .. }
            | TypedExpr::PositionalAccess { record, index, .. } => Expr::Field {
                record: Box::new(self.expr(record)?),
                index: *index as usize,
            },
            TypedExpr::TupleIndex { tuple, index, .. } => Expr::Field {
                record: Box::new(self.expr(tuple)?),
                index: *index as usize,
            },
            TypedExpr::Tuple { elements, .. } => Expr::Tuple(
                elements
                    .iter()
                    .map(|element| self.expr(element))
                    .collect::<Result<_>>()?,
            ),
            TypedExpr::Todo {
                location, message, ..
            }
            | TypedExpr::Panic {
                location, message, ..
            } => Expr::Panic {
                message: Box::new(
                    message
                        .as_ref()
                        .map(|message| self.expr(message))
                        .transpose()?
                        .unwrap_or_else(|| {
                            Expr::String(
                                if matches!(expression, TypedExpr::Todo { .. }) {
                                    "not implemented"
                                } else {
                                    "panic"
                                }
                                .into(),
                            )
                        }),
                ),
                location: self.location(*location),
            },
            TypedExpr::Echo {
                expression,
                message,
                location,
                ..
            } => Expr::Echo {
                value: Box::new(self.expr(expression.as_ref().ok_or_else(|| {
                    Error::Lowering("echo without input outside pipeline".into())
                })?)?),
                message: message
                    .as_ref()
                    .map(|message| self.expr(message).map(Box::new))
                    .transpose()?,
                location: self.location(*location),
            },
            TypedExpr::BitArray { segments, .. } => Expr::BitArray(
                segments
                    .iter()
                    .map(|segment| {
                        Ok(ir::BitSegment {
                            value: self.expr(&segment.value)?,
                            options: self.bit_options_expr(&segment.options)?,
                        })
                    })
                    .collect::<Result<_>>()?,
            ),
            TypedExpr::RecordUpdate {
                updated_record,
                updated_record_assigned_name,
                constructor,
                arguments,
                location,
                ..
            } => {
                let outer = self.locals.clone();
                let assignment = if let Some(name) = updated_record_assigned_name {
                    let value = self.expr(updated_record)?;
                    let binding = self.bind(name);
                    Some(Statement::Let {
                        pattern: Pattern::Variable(binding),
                        value,
                        assert: false,
                        message: None,
                        location: self.location(*location),
                    })
                } else {
                    None
                };
                let function = self.expr(constructor)?;
                let args = arguments
                    .iter()
                    .map(|argument| self.expr(&argument.value))
                    .collect::<Result<_>>()?;
                let result = call(function, args);
                self.locals = outer;
                if let Some(assignment) = assignment {
                    Expr::Block(vec![assignment, Statement::Expr(result)])
                } else {
                    result
                }
            }
            TypedExpr::NegateBool { value, .. } => Expr::NegateBool(Box::new(self.expr(value)?)),
            TypedExpr::NegateInt { value, .. } => Expr::NegateInt(Box::new(self.expr(value)?)),
            TypedExpr::Invalid { location, .. } => {
                return Err(Error::Lowering(format!(
                    "invalid typed expression in {} at {}",
                    self.module, location.start
                )));
            }
        })
    }

    fn pipeline_expression(
        &mut self,
        expression: &TypedExpr,
        previous: Option<&str>,
    ) -> Result<Expr> {
        if let TypedExpr::Echo {
            expression: None,
            message,
            location,
            ..
        } = expression
        {
            Ok(Expr::Echo {
                value: Box::new(Expr::Var(
                    previous
                        .ok_or_else(|| {
                            Error::Lowering("pipeline echo has no preceding value".into())
                        })?
                        .into(),
                )),
                message: message
                    .as_ref()
                    .map(|message| self.expr(message).map(Box::new))
                    .transpose()?,
                location: self.location(*location),
            })
        } else {
            self.expr(expression)
        }
    }

    fn function_ref(&mut self, module: &str, name: &str) -> Expr {
        let key = symbol(module, name);
        Expr::Function(key)
    }

    fn constant_ref_or_literal(
        &mut self,
        module: &str,
        name: &str,
        literal: &TypedConstant,
    ) -> Result<Expr> {
        let key = symbol(module, name);
        if self.constant_names.contains(&key) {
            Ok(Expr::Constant(key))
        } else {
            self.constant(literal)
        }
    }

    fn record_constructor(&self, module: &str, name: &str, arity: usize) -> Result<Expr> {
        if module.is_empty() || module == "i" {
            match name {
                "True" => return Ok(Expr::Bool(true)),
                "False" => return Ok(Expr::Bool(false)),
                "Nil" => return Ok(Expr::Nil),
                _ => {}
            }
        }
        let constructor = self.constructor(module, name)?;
        if constructor.arity != arity {
            return Err(Error::Lowering(format!(
                "constructor {} arity mismatch",
                constructor.symbol
            )));
        }
        if arity == 0 {
            Ok(Expr::Record {
                tag: constructor.tag,
                args: vec![],
            })
        } else {
            Ok(Expr::Constructor {
                tag: constructor.tag,
                arity,
            })
        }
    }

    fn value_constructor(&mut self, value: &ValueConstructor, fallback_name: &str) -> Result<Expr> {
        match &value.variant {
            ValueConstructorVariant::LocalVariable { .. } => {
                Ok(Expr::Var(self.local(fallback_name)?))
            }
            ValueConstructorVariant::ModuleFn { module, name, .. } => {
                Ok(self.function_ref(module, name))
            }
            ValueConstructorVariant::ModuleConstant {
                module,
                name,
                literal,
                ..
            } => self.constant_ref_or_literal(module, name, literal),
            ValueConstructorVariant::Record {
                module,
                name,
                arity,
                ..
            } => self.record_constructor(module, name, *arity as usize),
        }
    }

    fn float(&self, value: f64) -> Result<Expr> {
        if !value.is_finite() {
            return Err(Error::Lowering(
                "floating point literal is not finite".into(),
            ));
        }
        Ok(Expr::Float(value))
    }

    fn constant(&mut self, constant: &TypedConstant) -> Result<Expr> {
        Ok(match constant {
            Constant::Int { int_value, .. } => Expr::Int(int_value.clone()),
            Constant::Float { float_value, .. } => self.float(float_value.value())?,
            Constant::String { value, .. } => {
                Expr::String(convert_string_escape_chars(value).to_string())
            }
            Constant::Tuple { elements, .. } => Expr::Tuple(
                elements
                    .iter()
                    .map(|value| self.constant(value))
                    .collect::<Result<_>>()?,
            ),
            Constant::List { elements, tail, .. } => Expr::List {
                elements: elements
                    .iter()
                    .map(|value| self.constant(value))
                    .collect::<Result<_>>()?,
                tail: tail
                    .as_ref()
                    .map(|value| self.constant(value).map(Box::new))
                    .transpose()?,
            },
            Constant::Var {
                name,
                constructor: Some(constructor),
                ..
            } => self.value_constructor(constructor, name)?,
            Constant::Var { name, .. } => {
                return Err(Error::Lowering(format!("unresolved constant {name}")));
            }
            Constant::Record {
                name,
                arguments,
                record_constructor,
                module,
                ..
            } => {
                let constructor = if let Some(constructor) = record_constructor {
                    self.value_constructor(constructor, name)?
                } else {
                    let module = module
                        .as_ref()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or(&self.module);
                    self.record_constructor(module, name, arguments.as_ref().map_or(0, Vec::len))?
                };
                if let Some(arguments) = arguments {
                    let args = arguments
                        .iter()
                        .map(|argument| self.constant(&argument.value))
                        .collect::<Result<_>>()?;
                    match constructor {
                        Expr::Record { tag, .. } => Expr::Record { tag, args },
                        Expr::Bool(value) if args.is_empty() => Expr::Bool(value),
                        Expr::Nil if args.is_empty() => Expr::Nil,
                        constructor => call(constructor, args),
                    }
                } else {
                    constructor
                }
            }
            Constant::RecordUpdate {
                record,
                arguments,
                field_map,
                ..
            } => {
                let base = self.constant(&record.base)?;
                let Inferred::Known(fields) = field_map else {
                    return Err(Error::Lowering(
                        "unresolved constant record update fields".into(),
                    ));
                };
                let updates = arguments
                    .iter()
                    .map(|argument| {
                        let index = fields.fields.get(&argument.label).ok_or_else(|| {
                            Error::Lowering(format!(
                                "unknown constant record update field {}",
                                argument.label
                            ))
                        })?;
                        Ok((*index as usize, self.constant(&argument.value)?))
                    })
                    .collect::<Result<_>>()?;
                Expr::RecordUpdate {
                    record: Box::new(base),
                    updates,
                }
            }
            Constant::BitArray { segments, .. } => Expr::BitArray(
                segments
                    .iter()
                    .map(|segment| {
                        Ok(ir::BitSegment {
                            value: self.constant(&segment.value)?,
                            options: self.bit_options_constant(&segment.options)?,
                        })
                    })
                    .collect::<Result<_>>()?,
            ),
            Constant::BinaryOperator {
                operator,
                left,
                right,
                ..
            } => Expr::BinOp {
                op: *operator,
                lhs: Box::new(self.constant(left)?),
                rhs: Box::new(self.constant(right)?),
            },
            Constant::Todo {
                location, message, ..
            } => Expr::Panic {
                message: Box::new(
                    message
                        .as_ref()
                        .map(|message| self.constant(message))
                        .transpose()?
                        .unwrap_or_else(|| Expr::String("not implemented".into())),
                ),
                location: self.location(*location),
            },
            Constant::Invalid { location, .. } => {
                return Err(Error::Lowering(format!(
                    "invalid constant in {} at {}",
                    self.module, location.start
                )));
            }
        })
    }
}

fn call(fun: Expr, args: Vec<Expr>) -> Expr {
    if let Expr::Constructor { tag, .. } = fun {
        Expr::Record { tag, args }
    } else {
        Expr::Call {
            fun: Box::new(fun),
            args,
        }
    }
}

fn symbol(module: &str, name: &str) -> String {
    format!("{module}.{name}")
}

/// Alpha-renaming makes capture calculation independent of spelling and avoids
/// the usual shadowing ambiguity: all bound names can safely be subtracted from
/// all referenced names, including nested functions.
pub fn free_variables(expression: &Expr, arguments: &[String]) -> Vec<String> {
    let mut used = BTreeSet::new();
    let mut bound = arguments.iter().cloned().collect();
    visit_variables(expression, &mut used, &mut bound);
    used.difference(&bound).cloned().collect()
}

fn visit_variables(expression: &Expr, used: &mut BTreeSet<String>, bound: &mut BTreeSet<String>) {
    match expression {
        Expr::Var(name) => {
            used.insert(name.clone());
        }
        Expr::Call { fun, args } => {
            visit_variables(fun, used, bound);
            for arg in args {
                visit_variables(arg, used, bound);
            }
        }
        Expr::Lambda {
            args,
            captures,
            body,
        } => {
            bound.extend(args.iter().cloned());
            used.extend(captures.iter().cloned());
            visit_variables(body, used, bound);
        }
        Expr::Block(statements) => {
            for statement in statements {
                match statement {
                    Statement::Expr(value) => visit_variables(value, used, bound),
                    Statement::Let {
                        pattern,
                        value,
                        message,
                        ..
                    } => {
                        visit_variables(value, used, bound);
                        visit_pattern_variables(pattern, used, bound);
                        if let Some(message) = message {
                            visit_variables(message, used, bound);
                        }
                    }
                    Statement::Assert { value, message, .. } => {
                        visit_variables(value, used, bound);
                        if let Some(message) = message {
                            visit_variables(message, used, bound);
                        }
                    }
                }
            }
        }
        Expr::Tuple(values) | Expr::Record { args: values, .. } => {
            for value in values {
                visit_variables(value, used, bound);
            }
        }
        Expr::List { elements, tail } => {
            for value in elements {
                visit_variables(value, used, bound);
            }
            if let Some(tail) = tail {
                visit_variables(tail, used, bound);
            }
        }
        Expr::Field { record, .. } | Expr::NegateBool(record) | Expr::NegateInt(record) => {
            visit_variables(record, used, bound)
        }
        Expr::RecordUpdate { record, updates } => {
            visit_variables(record, used, bound);
            for (_, value) in updates {
                visit_variables(value, used, bound);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            visit_variables(lhs, used, bound);
            visit_variables(rhs, used, bound);
        }
        Expr::Case { subjects, clauses } => {
            for subject in subjects {
                visit_variables(subject, used, bound);
            }
            for clause in clauses {
                for patterns in &clause.alternatives {
                    for pattern in patterns {
                        visit_pattern_variables(pattern, used, bound);
                    }
                }
                if let Some(guard) = &clause.guard {
                    visit_variables(guard, used, bound);
                }
                visit_variables(&clause.body, used, bound);
            }
        }
        Expr::Panic { message, .. } => visit_variables(message, used, bound),
        Expr::Echo { value, message, .. } => {
            visit_variables(value, used, bound);
            if let Some(message) = message {
                visit_variables(message, used, bound);
            }
        }
        Expr::BitArray(segments) => {
            for segment in segments {
                visit_variables(&segment.value, used, bound);
                if let Some(size) = &segment.options.size {
                    visit_variables(size, used, bound);
                }
            }
        }
        Expr::Nil
        | Expr::Bool(_)
        | Expr::Int(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Function(_)
        | Expr::Constant(_)
        | Expr::Constructor { .. } => {}
    }
}

fn visit_pattern_variables(
    pattern: &Pattern,
    used: &mut BTreeSet<String>,
    bound: &mut BTreeSet<String>,
) {
    match pattern {
        Pattern::Variable(name) => {
            bound.insert(name.clone());
        }
        Pattern::Alias { name, pattern } => {
            bound.insert(name.clone());
            visit_pattern_variables(pattern, used, bound);
        }
        Pattern::Tuple(patterns)
        | Pattern::Record {
            arguments: patterns,
            ..
        } => {
            for pattern in patterns {
                visit_pattern_variables(pattern, used, bound);
            }
        }
        Pattern::List { elements, tail } => {
            for pattern in elements {
                visit_pattern_variables(pattern, used, bound);
            }
            if let Some(tail) = tail {
                visit_pattern_variables(tail, used, bound);
            }
        }
        Pattern::StringPrefix { left, rest, .. } => {
            bound.extend(left.iter().cloned());
            bound.extend(rest.iter().cloned());
        }
        Pattern::BitArray(segments) => {
            for segment in segments {
                visit_pattern_variables(&segment.value, used, bound);
                if let Some(size) = &segment.options.size {
                    visit_variables(size, used, bound);
                }
            }
        }
        Pattern::Value(value) => visit_variables(value, used, bound),
        Pattern::Discard
        | Pattern::Nil
        | Pattern::Bool(_)
        | Pattern::Int(_)
        | Pattern::Float(_)
        | Pattern::String(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checked(source: &str) -> Vec<Arc<TypedModule>> {
        let directory = tempfile::tempdir().expect("temporary project");
        std::fs::create_dir(directory.path().join("src")).expect("source directory");
        std::fs::write(
            directory.path().join("i.toml"),
            "name = \"fixture\"\nversion = \"1.0.0\"\n",
        )
        .expect("manifest");
        std::fs::write(directory.path().join("src/fixture.i"), source).expect("source");
        ilang_frontend::check_project(
            directory.path(),
            &ilang_frontend::CheckOptions {
                use_cache: false,
                ..Default::default()
            },
        )
        .expect("type check fixture")
        .modules
    }

    #[test]
    fn source_locations_use_one_based_utf16_columns() {
        let source = "first\n😀é panic";
        let lines = LineNumbers::new(source);
        let constructors = BTreeMap::new();
        let constants = BTreeSet::new();
        let lowerer = Lowerer::new("fixture", &constructors, &constants).with_lines(&lines);
        let start = source.find("panic").unwrap() as u32;
        let location = lowerer.location(SrcSpan::new(start, start + 5));
        assert_eq!((location.line, location.column), (2, 5));
        assert_eq!((location.start, location.end), (start, start + 5));
    }

    #[test]
    fn checked_panic_retains_source_line_and_column() {
        let source = "pub fn main() {\n  panic as \"failure\"\n}\n";
        let modules = checked(source);
        let program = lower(&modules, &["fixture.main".into()]).unwrap();
        let function = &program.functions[0];
        assert_eq!(function.location.line, 1);
        assert_ne!(function.location.column, 0);
        let Expr::Panic { location, .. } = &function.body else {
            panic!("expected lowered panic: {:?}", function.body)
        };
        assert_eq!(location.module, "fixture");
        assert_eq!((location.line, location.column), (2, 3));
        assert_eq!(location.start, source.find("panic").unwrap() as u32);
    }

    #[test]
    fn checked_program_preserves_shadowing_and_reachable_dependencies() {
        let modules = checked(
            r#"
pub type Box(a) { Box(value: a) }
fn helper(value: Int) { value + 1 }
fn unused() { 99 }
pub fn main() {
  let x = 2
  let make = fn(x) { fn(y) { Box(helper(x + y)) } }
  let boxed = make(x)(3)
  let Box(result) = Box(..boxed, value: boxed.value + 4)
  result
}
"#,
        );
        let program = lower(&modules, &["fixture.main".into()]).expect("lower");
        assert_eq!(
            program
                .functions
                .iter()
                .map(|function| function.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["fixture.helper", "fixture.main"]
        );
        let main = program
            .functions
            .iter()
            .find(|function| function.symbol == "fixture.main")
            .unwrap();
        assert!(
            free_variables(&main.body, &main.arguments).is_empty(),
            "{:#?}",
            main.body
        );
        assert_eq!(
            program
                .constructors
                .iter()
                .find(|constructor| constructor.symbol == "fixture.Box")
                .unwrap()
                .arity,
            1
        );
        let json = serde_json::to_string(&program).expect("serializable IR");
        let decoded: Program = serde_json::from_str(&json).expect("IR deserialization");
        assert_eq!(decoded.functions.len(), program.functions.len());
    }

    #[test]
    fn checked_use_pipeline_and_alternatives_have_no_unbound_locals() {
        let modules = checked(
            r#"
fn with(value, callback) { callback(value) }
pub fn main() {
  use value <- with(4)
  let result = value |> with(fn(n) { n + 1 })
  case #(result, "prefix-tail") {
    #(5, "prefix-" <> rest) | #(6, "prefix-" <> rest) if rest == "tail" -> Nil
    _ -> panic as "unexpected result"
  }
}
"#,
        );
        let program = lower(&modules, &["fixture.main".into()]).expect("lower");
        for function in &program.functions {
            assert!(
                free_variables(&function.body, &function.arguments).is_empty(),
                "{:#?}",
                function
            );
        }
    }

    #[test]
    fn ir_cache_reuses_definitions_and_recovers_from_corruption() {
        let modules = checked("fn helper() { 1 }\npub fn main() { helper() }\n");
        let directory = tempfile::tempdir().unwrap();
        let roots = ["fixture.main".into()];
        let (_, cold) = lower_cached(&modules, &roots, directory.path(), "compiler-a").unwrap();
        assert_eq!(
            cold,
            CacheStats {
                hits: 0,
                lowered: 2,
                writes: 2
            }
        );
        let (_, warm) = lower_cached(&modules, &roots, directory.path(), "compiler-a").unwrap();
        assert_eq!(
            warm,
            CacheStats {
                hits: 2,
                lowered: 0,
                writes: 0
            }
        );
        for file in std::fs::read_dir(directory.path()).unwrap() {
            std::fs::write(file.unwrap().path(), b"invalid cache contents").unwrap();
        }
        let (_, repaired) = lower_cached(&modules, &roots, directory.path(), "compiler-a").unwrap();
        assert_eq!(repaired, cold);
        let (_, compiler_changed) =
            lower_cached(&modules, &roots, directory.path(), "compiler-b").unwrap();
        assert_eq!(compiler_changed, cold);
    }

    #[test]
    fn ir_cache_invalidates_when_global_constructor_layout_changes() {
        let mut modules = checked("pub fn main() { 1 }\n");
        let directory = tempfile::tempdir().unwrap();
        let roots = ["fixture.main".into()];
        lower_cached(&modules, &roots, directory.path(), "compiler").unwrap();
        let mut extra = checked("pub type Extra { Extra }\n").pop().unwrap();
        Arc::make_mut(&mut extra).name = "extra".into();
        modules.push(extra);
        let (_, stats) = lower_cached(&modules, &roots, directory.path(), "compiler").unwrap();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.lowered, 1);
    }

    #[test]
    fn dependency_implementation_change_reuses_consumer_ir() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("src")).unwrap();
        std::fs::write(
            directory.path().join("i.toml"),
            "name = \"fixture\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(directory.path().join("src/dep.i"), "pub fn value() { 1 }\n").unwrap();
        std::fs::write(
            directory.path().join("src/fixture.i"),
            "import dep\npub fn main() { dep.value() }\n",
        )
        .unwrap();
        let mut project = ilang_frontend::Project::load(directory.path()).unwrap();
        let initial = project.check(&Default::default()).unwrap();
        let cache_dir = directory.path().join("ir-cache");
        let roots = ["fixture.main".into()];
        lower_cached(&initial.modules, &roots, &cache_dir, "compiler").unwrap();
        std::fs::write(directory.path().join("src/dep.i"), "pub fn value() { 2 }\n").unwrap();
        let changed = project.check(&Default::default()).unwrap();
        let (_, stats) = lower_cached(&changed.modules, &roots, &cache_dir, "compiler").unwrap();
        assert_eq!(
            stats,
            CacheStats {
                hits: 1,
                lowered: 1,
                writes: 1
            }
        );
    }

    #[test]
    fn ir_cache_supports_the_complete_upstream_language_corpus() {
        let project = ilang_frontend::check_project(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/conformance/projects/upstream_language"),
            &ilang_frontend::CheckOptions {
                include_tests: true,
                use_cache: false,
                ..Default::default()
            },
        )
        .expect("check upstream language fixtures");
        let roots = project
            .modules
            .iter()
            .flat_map(|module| {
                module.definitions.functions.iter().filter_map(|function| {
                    function
                        .name
                        .as_ref()
                        .map(|(_, name)| symbol(&module.name, name))
                })
            })
            .collect::<Vec<_>>();
        let directory = tempfile::tempdir().unwrap();
        let (_, cold) = lower_cached(&project.modules, &roots, directory.path(), "corpus").unwrap();
        assert!(cold.lowered >= 490);
        assert_eq!(cold.writes, cold.lowered);
        let (_, warm) = lower_cached(&project.modules, &roots, directory.path(), "corpus").unwrap();
        assert_eq!(warm.hits, cold.lowered);
        assert_eq!(warm.lowered, 0);
    }

    #[test]
    fn captures_outer_binding_despite_nested_shadowing() {
        let body = Expr::Block(vec![
            Statement::Let {
                pattern: Pattern::Variable("v1_x".into()),
                value: Expr::Var("v0_x".into()),
                assert: false,
                message: None,
                location: Location::default(),
            },
            Statement::Expr(Expr::Lambda {
                args: vec!["v2_x".into()],
                captures: vec!["v1_x".into()],
                body: Box::new(Expr::Tuple(vec![
                    Expr::Var("v2_x".into()),
                    Expr::Var("v1_x".into()),
                ])),
            }),
        ]);
        assert_eq!(free_variables(&body, &[]), vec!["v0_x"]);
    }

    #[test]
    fn captures_variables_used_only_in_bit_pattern_sizes() {
        let body = Expr::Case {
            subjects: vec![Expr::Var("input".into())],
            clauses: vec![ir::Clause {
                alternatives: vec![vec![Pattern::BitArray(vec![ir::BitSegment {
                    value: Pattern::Variable("byte".into()),
                    options: ir::BitOptions {
                        size: Some(Box::new(Expr::Var("size".into()))),
                        ..Default::default()
                    },
                }])]],
                guard: None,
                body: Expr::Var("byte".into()),
            }],
        };
        assert_eq!(free_variables(&body, &["input".into()]), vec!["size"]);
    }
}
