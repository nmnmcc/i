//! Typed macro execution, with compiler syntax services and retained input holes.
use crate::{
    body_engine::BodyEngine,
    definition_world::DefinitionWorld,
    prepare::MacroPosition,
    registry::Registry,
    session::{DefinitionResolver, Session},
};
use ilang_check::{
    Parsed,
    hir::{self, ModuleId, Program},
    typed_macro::{Expander, Invocation, Output},
    types::{MacroTruth, TypedMacroQuery},
};
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::{
    lisp::{
        expansion::MacroArgument,
        scoped::ScopedSource,
        syntax::{Node, Scope, Syntax},
    },
    span::Span,
    syn3::ast,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

struct State {
    session: Session,
    scoped: HashMap<ModuleId, ScopedSource>,
    generated: HashMap<(ModuleId, Span), ast::Expr>,
}
pub struct Driver {
    graph: Arc<PackageGraph>,
    parsed: Arc<Vec<Parsed>>,
    /// The same modules, shared with each call's fresh definition world.
    shared: Vec<Arc<Parsed>>,
    /// The expanded program is fixed, so its macro index is built once.
    registry: std::sync::OnceLock<Result<Registry, Vec<Diagnostic>>>,
    /// A copy of `parsed` in which one call at a time installs its caller's
    /// current source map, restoring it afterwards; this avoids copying the
    /// whole program for every typed call and type parse.
    scratch: Arc<Mutex<Vec<Parsed>>>,
    state: Mutex<State>,
}
impl std::fmt::Debug for Driver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TypedMacroDriver")
    }
}
impl Driver {
    pub fn new(graph: &PackageGraph, parsed: &[Parsed]) -> Self {
        Self {
            graph: Arc::new(graph.clone()),
            parsed: Arc::new(parsed.to_vec()),
            shared: parsed.iter().cloned().map(Arc::new).collect(),
            registry: std::sync::OnceLock::new(),
            scratch: Arc::new(Mutex::new(parsed.to_vec())),
            state: Mutex::new(State {
                session: Session::default(),
                scoped: HashMap::new(),
                generated: HashMap::new(),
            }),
        }
    }
}
/// Runs `f` on the driver's modules with `scoped` installed as the source map
/// of module `index`, exactly as on a fresh copy of `parsed` changed in that one
/// place. The shared scratch copy is restored before returning; if it is in use
/// (a nested or concurrent call), a fresh copy is made instead.
fn with_scoped<T>(
    scratch: &Mutex<Vec<Parsed>>,
    parsed: &[Parsed],
    index: usize,
    scoped: ScopedSource,
    f: impl FnOnce(&[Parsed]) -> T,
) -> T {
    match scratch.try_lock() {
        Ok(mut modules) => {
            let previous = modules[index].scoped.replace(scoped);
            let result = f(&modules);
            modules[index].scoped = previous;
            result
        }
        Err(_) => {
            let mut modules = parsed.to_vec();
            modules[index].scoped = Some(scoped);
            f(&modules)
        }
    }
}

fn diagnostic(
    program: &Program,
    module: ModuleId,
    span: Span,
    message: impl Into<String>,
) -> Vec<Diagnostic> {
    vec![Diagnostic::error(
        ilang_check::code::MACRO_RESERVED,
        Some(program.module(module).source),
        span,
        message,
    )]
}
fn children(s: &Syntax) -> &[Syntax] {
    match s.node() {
        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => xs,
        _ => &[],
    }
}
fn syntax_for(p: &Parsed, graph: &PackageGraph, e: &ast::Expr) -> Syntax {
    if let Some(origin) = p.scoped.as_ref().and_then(|s| s.origin(e.span.0)) {
        return origin.syntax.clone();
    }
    let file = graph.sources.get(p.source);
    let scope = Scope {
        module: p.source.0,
        marks: Default::default(),
    };
    let syntax = if file.display.ends_with(".is") {
        ilang_syntax::lisp::source_syntax::written_lisp_syntax(e.span.0, &scope, &file.text)
    } else {
        let tokens = Arc::new(ilang_syntax::syn3::lisp::TokenIndex::new(&file.text));
        ilang_syntax::syn3::lisp::with_tokens(&tokens, || {
            Syntax::from_datum(&ilang_syntax::syn3::lisp::expr_datum(e), &scope)
        })
    };
    ilang_syntax::lisp::source_syntax::normalize_expression(&syntax)
}
impl Expander for Driver {
    fn expand(
        &self,
        call: Invocation<'_>,
        query: Arc<TypedMacroQuery>,
    ) -> Result<Output, Vec<Diagnostic>> {
        ilang_syntax::syn3::deep::deep(|| self.expand_here(call, query))
    }
}
impl Driver {
    fn expand_here(
        &self,
        call: Invocation<'_>,
        query: Arc<TypedMacroQuery>,
    ) -> Result<Output, Vec<Diagnostic>> {
        let Invocation {
            program,
            module,
            span,
            definition,
            var_count,
            ty_var_count,
        } = call;
        let mut caller = self.parsed[module.0 as usize].clone();
        caller.scoped = program.module(module).scoped.clone();
        let p = &caller;
        let expressions = crate::typed3::module_expressions(&p.ast);
        let original = expressions
            .get(&span)
            .cloned()
            .or_else(|| {
                self.state
                    .lock()
                    .expect("typed session")
                    .generated
                    .get(&(module, span))
                    .cloned()
            })
            .expect("resolved typed call has source syntax");
        let written = crate::call3::expression(&original).expect("typed expression is a call");
        let mut arguments: Vec<_> =
            crate::body_engine3::expression_arguments(&written, |e| syntax_for(p, &self.graph, e))
                .into_iter()
                .map(|arg| arg.syntax)
                .collect();
        let order = query.argument_order();
        arguments = order
            .iter()
            .map(|i| arguments[*i as usize].clone())
            .collect();
        query.set_arguments(&arguments);
        let call = syntax_for(p, &self.graph, &original);
        let parser_graph = self.graph.clone();
        let parser_parsed = self.parsed.clone();
        let parser_scratch = self.scratch.clone();
        let parser_program = program.clone();
        let parser_call = call.clone();
        query.set_parser(Arc::new(move |syntax| {
            let index = module.0 as usize;
            let mut scoped = parser_program
                .module(module)
                .scoped
                .clone()
                .unwrap_or_else(|| {
                    ScopedSource::new(
                        parser_graph
                            .sources
                            .get(parser_parsed[index].source)
                            .text
                            .clone(),
                    )
                });
            let ty = scoped.ty3(syntax, &parser_call).map_err(|e| e.message)?;
            let expr = crate::typed3::output_type_function(ty);
            let resolved = with_scoped(&parser_scratch, &parser_parsed, index, scoped, |parsed| {
                ilang_check::resolve::typed_output(
                    &parser_graph,
                    parsed,
                    &parser_program,
                    module,
                    span,
                    &expr,
                    var_count,
                    ty_var_count,
                    &ilang_check::resolve::CheckedInputs::default(),
                )
            })
            .map(|(resolved, _)| resolved)
            .map_err(|ds| {
                ds.into_iter()
                    .map(|d| d.message)
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            check_dependencies(&parser_program, module, span, &resolved).map_err(|ds| {
                ds.into_iter()
                    .map(|d| d.message)
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            let hir::ExprKind::Fn(f) = resolved.kind else {
                unreachable!("type parser wrapper")
            };
            Ok(f.ret.expect("type query annotation").value)
        }));
        let mut state = self.state.lock().expect("typed expansion session");
        // Spans remain unique across independent calls and consumer rechecks.
        let scoped = state
            .scoped
            .get(&module)
            .cloned()
            .or_else(|| p.scoped.clone())
            .unwrap_or_else(|| ScopedSource::new(self.graph.sources.get(p.source).text.clone()));
        let q = query.clone();
        let at = call.clone();
        let unknown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let saw_unknown = unknown.clone();
        state.session.types = Some(Arc::new(move |operation, args| {
            let answer = match operation {
                "type_of" => q.type_of(&args[0]).map(|s| vec![s]),
                "signature" => Ok(vec![q.signature()]),
                "members" => q.members(&args[0]),
                "subtype" => q.subtype(&args[0], &args[1]).map(|truth| {
                    let name = match truth {
                        MacroTruth::Yes => "Yes",
                        MacroTruth::No => "No",
                        MacroTruth::NotKnown => {
                            saw_unknown.store(true, std::sync::atomic::Ordering::Relaxed);
                            "NotKnown"
                        }
                    };
                    let d = ilang_syntax::lisp::parse(name).expect("truth atom");
                    vec![Syntax::from_datum(&d.datums[0], at.scope())]
                }),
                _ => unreachable!("typed service operation"),
            };
            answer.map_err(|message| (message, args.first().cloned().unwrap_or_else(|| at.clone())))
        }));
        let registry = self
            .registry
            .get_or_init(|| Registry::new(&self.graph, &self.parsed))
            .as_ref()
            .map_err(Clone::clone)?;
        let name = &program.def(definition).name;
        let target_source = program.module(definition.module).source;
        let target = &self.parsed[definition.module.0 as usize];
        let definitions = ilang_syntax::lisp::expansion3::definitions(&target.ast, true)
            .expect("typed definition validated");
        let mac = crate::registry::Macro {
            source: target_source,
            definition: definitions
                .into_iter()
                .find(|d| d.name == *name)
                .expect("typed implementation exists"),
        };
        let resolver: Arc<DefinitionResolver> =
            Arc::new(|_, _| panic!("typed definition resolver not installed"));
        let mut engine = BodyEngine::new(
            &self.graph,
            &self.parsed,
            registry,
            p.source,
            scoped,
            resolver,
            &mut state.session,
        );
        engine.use_completed_definitions(
            Arc::new(DefinitionWorld::new_shared(
                &self.graph,
                self.shared.clone(),
                Arc::new(std::sync::atomic::AtomicU64::new(0)),
            )),
            &p.ast,
        );
        let args = arguments
            .iter()
            .cloned()
            .map(|syntax| MacroArgument {
                label: None,
                syntax,
            })
            .collect();
        let (result, budget, transactions) = engine
            .execute(&mac, args, &call, span, None, MacroPosition::Expression)
            .map_err(|e| {
                let mut ds = crate::diagnostics::body(p.source, e);
                if unknown.load(std::sync::atomic::Ordering::Relaxed) {
                    for d in ds.iter_mut().filter(|d| d.is_error()) {
                        d.related.push(ilang_package::DiagnosticNote {
                            source: p.source,
                            span,
                            message: "a typed macro cannot choose by the type inside a binding with type variables; a type query answered NotKnown here (§15.7.5)".into(),
                        });
                    }
                }
                ds
            })?;
        let mut syntax = engine
            .scoped
            .expression3_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|e| {
                diagnostic(
                    program,
                    module,
                    span,
                    format!("typed macro must return expression syntax: {e:?}"),
                )
            })?;
        engine.scoped.inherit_call_chain(syntax.span.0, span, name);
        // Ordinary syntax macros in the result still run in expression/type
        // position. No Items service exists in this phase.
        let mut wrapper = crate::typed3::wrapper(syntax, span);
        crate::body_walk3::expressions(&mut wrapper, &mut engine)
            .map_err(|e| crate::diagnostics::body(p.source, e))?;
        syntax = crate::typed3::unwrap(wrapper);
        let scoped = engine.scoped.clone();
        drop(engine);
        state.session.types = None;
        state.scoped.insert(module, scoped.clone());
        let mut input_nodes = Vec::new();
        let mut work = arguments.clone();
        while let Some(s) = work.pop() {
            work.extend(children(&s).iter().cloned());
            input_nodes.push(s);
        }
        let mut original_nodes = HashMap::new();
        for b in &program.module(module).bindings {
            let mut e = b.binding.value.clone();
            hir::visit_expr_mut(&mut e, &mut |e| {
                original_nodes.insert(e.span, e.clone());
                true
            });
        }

        let mut checked = ilang_check::resolve::CheckedInputs::default();
        let mut input_binders = HashMap::new();
        for binding in &program.module(module).bindings {
            for binder in hir::expression_binders(&binding.binding.value) {
                input_binders.insert(binder.span, binder.var);
            }
        }
        crate::typed3::for_each_expression_span(&syntax, |new| {
            if let Some(origin) = scoped.origin(new)
                && let Some(input) = input_nodes.iter().find(|s| s.same_input(&origin.syntax))
                && let Some(old) = query
                    .input_span(input)
                    .and_then(|span| original_nodes.get(&span))
            {
                checked.expressions.insert(new, old.clone());
            }
        });
        crate::typed3::for_each_span(&syntax, |new| {
            if let Some(origin) = scoped.origin(new)
                && input_nodes
                    .iter()
                    .any(|input| input.same_input(&origin.syntax))
                && let Some((_, var)) = input_binders.iter().find(|(old, _)| {
                    if let Some(original) = program
                        .module(module)
                        .scoped
                        .as_ref()
                        .and_then(|s| s.origin(**old))
                    {
                        original.syntax.same_input(&origin.syntax)
                    } else {
                        **old == origin.syntax.span()
                    }
                })
            {
                checked.binders.insert(new, *var);
            }
        });
        let holes = checked
            .expressions
            .values()
            .map(|old| (old.span, old.span))
            .collect();
        let (expression, mut resolution) = with_scoped(
            &self.scratch,
            &self.parsed,
            module.0 as usize,
            scoped.clone(),
            |parsed| {
                ilang_check::resolve::typed_output(
                    &self.graph,
                    parsed,
                    program,
                    module,
                    span,
                    &syntax,
                    var_count,
                    ty_var_count,
                    &checked,
                )
            },
        )
        .map_err(|mut ds| {
            for d in &mut ds {
                if let Some(o) = scoped.origin(d.span) {
                    d.related.push(ilang_package::DiagnosticNote {
                        source: ilang_package::SourceId(o.syntax.scope().module),
                        span: o.syntax.span(),
                        message: "typed output originates here".into(),
                    });
                }
                d.source = Some(p.source);
                d.span = span;
            }
            ds
        })?;
        let mut spans = Vec::new();
        crate::typed3::for_each_expression_span(&syntax, |new| {
            if let Some(origin) = scoped.origin(new)
                && let Some(input) = input_nodes.iter().find(|s| s.same_input(&origin.syntax))
                && let Some(old) = query.input_span(input)
            {
                spans.push((new, old));
            }
        });
        let delayed = crate::typed3::hole_inside_function(&syntax, |new| {
            spans
                .iter()
                .find(|(span, _)| *span == new)
                .is_some_and(|(_, old)| query.argument_is_effectful(*old))
        });
        if delayed {
            return Err(diagnostic(
                program,
                module,
                span,
                "a typed macro cannot delay an argument that has errors or needs an environment",
            ));
        }
        check_dependencies(program, module, span, &expression)?;
        let generated = crate::typed3::expressions(&syntax);
        state
            .generated
            .extend(generated.into_iter().map(|(span, e)| ((module, span), e)));
        resolution.modules[module.0 as usize].scoped = Some(scoped.clone());
        for binding in &mut resolution.modules[module.0 as usize].bindings {
            hir::visit_expr_mut(&mut binding.binding.value, &mut |e| {
                if e.span == span {
                    *e = expression.clone();
                    return false;
                }
                true
            });
        }
        let var_count = resolution.var_count;
        let ty_var_count = resolution.ty_var_count;
        Ok(Output {
            expression,
            syntax,
            scoped,
            var_count,
            ty_var_count,
            holes,
            resolution: Arc::new(resolution),
            trace: transactions.iter().map(|t| t.trace.clone()).collect(),
            notes: transactions.into_iter().flat_map(|t| t.notes).collect(),
        })
    }
}
fn check_dependencies(
    program: &Program,
    module: ModuleId,
    span: Span,
    output: &hir::Expr,
) -> Result<(), Vec<Diagnostic>> {
    let mut groups = HashMap::new();
    for (i, g) in program.order.groups.iter().enumerate() {
        for b in g {
            for d in &program.binding(*b).defs {
                groups.insert(*d, i);
            }
        }
    }
    let own = program
        .order
        .groups
        .iter()
        .position(|g| {
            g.iter().any(|b| {
                if b.module != module {
                    return false;
                }
                let mut found = false;
                let mut e = program.binding(*b).binding.value.clone();
                hir::visit_expr_mut(&mut e, &mut |e| {
                    found |= e.span == span;
                    true
                });
                found
            })
        })
        .expect("typed call belongs to a binding group");
    let mut allowed = HashSet::new();
    let mut work = vec![own];
    while let Some(i) = work.pop() {
        if !allowed.insert(i) {
            continue;
        }
        for b in &program.order.groups[i] {
            let mut e = program.binding(*b).binding.value.clone();
            hir::visit_expr_mut(&mut e, &mut |e| {
                if let hir::ExprKind::Name(hir::ValueRef::Def(d)) = e.kind
                    && let Some(j) = groups.get(&d)
                {
                    work.push(*j);
                }
                true
            });
        }
    }
    let mut invalid = false;
    let mut e = output.clone();
    hir::visit_expr_mut(&mut e, &mut |e| {
        if let hir::ExprKind::Name(hir::ValueRef::Def(d)) = e.kind
            && d.module == module
            && let Some(g) = groups.get(&d)
        {
            invalid |= !allowed.contains(g);
        }
        true
    });
    let mut alias_invalid = false;
    let temporary = hir::Binding {
        pattern: hir::Pattern {
            kind: hir::PatternKind::Discard,
            span,
        },
        value: output.clone(),
        direct: false,
        ty_vars: Vec::new(),
        span,
    };
    hir::binding_types(&temporary, &mut |t| {
        hir::walk_type(t, &mut |t| {
            if let hir::TypeKind::Named {
                ty: hir::TypeRef::Custom(id),
                ..
            } = t.kind
            {
                for source in &program.alias_sources[id.0 as usize] {
                    let site = &program.of_sites[source.0 as usize];
                    if site.module == module {
                        match site.value {
                            hir::ValueRef::Def(d) => {
                                alias_invalid |= groups
                                    .get(&d)
                                    .is_none_or(|g| *g == own || !allowed.contains(g));
                            }
                            _ => alias_invalid = true,
                        }
                    }
                }
            }
        })
    });
    if alias_invalid {
        return Err(diagnostic(
            program,
            module,
            span,
            "a typed macro result may use a value-dependent alias only from strict ancestor groups (T5)",
        ));
    }
    if invalid {
        Err(diagnostic(
            program,
            module,
            span,
            "typed macro output references an unrelated same-module dependency group (T5)",
        ))
    } else {
        Ok(())
    }
}
