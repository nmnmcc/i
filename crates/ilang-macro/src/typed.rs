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
    hir::{self, DefId, ModuleId, Program},
    typed_macro::{Expander, Output},
    types::{MacroTruth, TypedMacroQuery},
};
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::{
    ast,
    lisp::{
        expansion::MacroArgument,
        scoped::ScopedSource,
        syntax::{Node, Scope, Syntax},
    },
    span::Span,
    walk::{Visitor, Walk},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

struct State {
    session: Session,
    scoped: HashMap<ModuleId, ScopedSource>,
}
pub struct Driver {
    graph: PackageGraph,
    parsed: Vec<Parsed>,
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
            graph: graph.clone(),
            parsed: parsed.to_vec(),
            state: Mutex::new(State {
                session: Session::default(),
                scoped: HashMap::new(),
            }),
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
struct Expressions(HashMap<Span, ast::Expr>);
impl Visitor for Expressions {
    fn expr(&mut self, e: &mut ast::Expr, enter: bool) {
        if enter {
            self.0.insert(e.span, e.clone());
        }
    }
}
fn children(s: &Syntax) -> &[Syntax] {
    match s.node() {
        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => xs,
        _ => &[],
    }
}
fn syntax_for(p: &Parsed, graph: &PackageGraph, e: &ast::Expr) -> Syntax {
    if let Some(origin) = p.scoped.as_ref().and_then(|s| s.origin(e.span)) {
        return origin.syntax.clone();
    }
    let file = graph.sources.get(p.source);
    ilang_syntax::lisp::source_syntax::source_expression_syntax(
        e,
        &Scope {
            module: p.source.0,
            marks: Default::default(),
        },
        &file.text,
        file.display.ends_with(".is"),
    )
}
impl Expander for Driver {
    fn expand(
        &self,
        program: &Program,
        module: ModuleId,
        span: Span,
        definition: DefId,
        query: Arc<TypedMacroQuery>,
        var_count: u32,
        ty_var_count: u32,
    ) -> Result<Output, Vec<Diagnostic>> {
        let p = &self.parsed[module.0 as usize];
        let mut expressions = Expressions(HashMap::new());
        p.ast.clone().walk(&mut expressions);
        let original = expressions
            .0
            .get(&span)
            .expect("resolved typed call has source syntax");
        let written = crate::call::expression(original).expect("typed expression is a call");
        let mut arguments: Vec<_> = written
            .prefix
            .into_iter()
            .chain(written.args.iter().map(|a| &a.value))
            .map(|e| syntax_for(p, &self.graph, e))
            .collect();
        let order = query.argument_order();
        arguments = order
            .iter()
            .map(|i| arguments[*i as usize].clone())
            .collect();
        query.set_arguments(&arguments);
        let call = syntax_for(p, &self.graph, original);
        let parser_graph = self.graph.clone();
        let parser_parsed = self.parsed.clone();
        let parser_program = program.clone();
        let parser_call = call.clone();
        query.set_parser(Arc::new(move |syntax| {
            let mut parsed = parser_parsed.clone();
            let index = module.0 as usize;
            let mut scoped = parsed[index].scoped.clone().unwrap_or_else(|| {
                ScopedSource::new(parser_graph.sources.get(parsed[index].source).text.clone())
            });
            let ty = scoped.ty(syntax, &parser_call).map_err(|e| e.message)?;
            parsed[index].scoped = Some(scoped);
            let expr = ast::Expr {
                span: ty.span,
                kind: ast::ExprKind::Fn(Box::new(ast::Function {
                    params: Vec::new(),
                    ret: Some(ast::RetType {
                        value: ty,
                        err: None,
                        env: None,
                    }),
                    body: ast::Block {
                        stmts: Vec::new(),
                        span: Span::default(),
                    },
                })),
            };
            let (resolved, _, _) = ilang_check::resolve::typed_output(
                &parser_graph,
                &parsed,
                &parser_program,
                module,
                span,
                &expr,
                var_count,
                ty_var_count,
            )
            .map_err(|ds| {
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
        state.session.types = Some(Arc::new(move |operation, args| {
            let answer = match operation {
                "type_of" => q.type_of(&args[0]).map(|s| vec![s]),
                "signature" => Ok(vec![q.signature()]),
                "members" => q.members(&args[0]),
                "subtype" => q.subtype(&args[0], &args[1]).map(|truth| {
                    let name = match truth {
                        MacroTruth::Yes => "Yes",
                        MacroTruth::No => "No",
                        MacroTruth::NotKnown => "NotKnown",
                    };
                    let d = ilang_syntax::lisp::parse(name).expect("truth atom");
                    vec![Syntax::from_datum(&d.datums[0], at.scope())]
                }),
                _ => unreachable!("typed service operation"),
            };
            answer.map_err(|message| (message, args.first().cloned().unwrap_or_else(|| at.clone())))
        }));
        let registry = Registry::new(&self.graph, &self.parsed)?;
        let name = &program.def(definition).name;
        let target_source = program.module(definition.module).source;
        let target = &self.parsed[definition.module.0 as usize];
        let definitions = ilang_syntax::lisp::expansion::definitions(&target.ast, true)
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
            &registry,
            p.source,
            scoped,
            resolver,
            &mut state.session,
        );
        engine.use_completed_definitions(
            Arc::new(DefinitionWorld::new(&self.graph, &self.parsed)),
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
            .map_err(|e| crate::diagnostics::body(p.source, e))?;
        let mut syntax = engine
            .scoped
            .expression_metered(&result, &call, |work, words| {
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
        let start = syntax.span.start;
        engine.scoped.inherit_call_chain(syntax.span, span, name);
        // Ordinary syntax macros in the result still run in expression/type
        // position. No Items service exists in this phase.
        let mut wrapper = ast::Module::default();
        wrapper.items.push(ast::Item {
            doc: Vec::new(),
            attributes: Vec::new(),
            span: syntax.span,
            kind: ast::ItemKind::Binding(ast::Binding {
                pattern: ast::Pattern {
                    kind: ast::PatternKind::Discard("_".into()),
                    span: syntax.span,
                },
                value: syntax,
                type_params: Vec::new(),
                span: span,
            }),
        });
        crate::body_walk::expressions(&mut wrapper, &mut engine)
            .map_err(|e| crate::diagnostics::body(p.source, e))?;
        let ast::ItemKind::Binding(binding) = wrapper.items.pop().expect("output wrapper").kind
        else {
            unreachable!()
        };
        syntax = binding.value;
        let scoped = engine.scoped.clone();
        drop(engine);
        state.session.types = None;
        state.scoped.insert(module, scoped.clone());
        let mut parsed = self.parsed.clone();
        parsed[module.0 as usize].scoped = Some(scoped.clone());
        let (mut expression, var_count, ty_var_count) = ilang_check::resolve::typed_output(
            &self.graph,
            &parsed,
            program,
            module,
            span,
            &syntax,
            var_count,
            ty_var_count,
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
        let mut holes = Vec::new();
        let mut capture = None;
        hir::visit_expr_mut(&mut expression, &mut |e| {
            if let Some(origin) = scoped.origin(e.span) {
                if let Some(input) = input_nodes.iter().find(|s| s.same_input(&origin.syntax)) {
                    if let Some(old) = query.input_span(input).and_then(|span|original_nodes.get(&span)) {
                        if matches!(e.kind, hir::ExprKind::Name(_))
                            && matches!(old.kind, hir::ExprKind::Name(_))
                        {
                            // Forwarded free names keep their original resolved identity.
                            *e = old.clone();
                        }
                        holes.push((old.span, old.span));
                        if !matches!(e.kind, hir::ExprKind::Name(_)) {
                            *e = old.clone();
                            return false;
                        }
                    }
                }
            }
            if e.span.start < start {
                return false;
            }
            if matches!(e.kind, hir::ExprKind::Error) {
                capture = Some(e.span);
            }
            true
        });
        if capture.is_some() {
            return Err(diagnostic(
                program,
                module,
                span,
                "typed macro output cannot capture or escape an argument binder",
            ));
        }
        struct Delayed<'a> {
            depth: usize,
            holes: &'a [(Span, Span)],
            query: &'a TypedMacroQuery,
            invalid: bool,
        }
        impl Visitor for Delayed<'_> {
            fn expr(&mut self, e: &mut ast::Expr, enter: bool) {
                if enter {
                    if self.depth > 0
                        && let Some((_, old)) = self.holes.iter().find(|(new, _)| *new == e.span)
                    {
                        self.invalid |= self.query.argument_is_effectful(*old);
                    }
                    if matches!(e.kind, ast::ExprKind::Fn(_) | ast::ExprKind::Clauses(_)) {
                        self.depth += 1;
                    }
                } else if matches!(e.kind, ast::ExprKind::Fn(_) | ast::ExprKind::Clauses(_)) {
                    self.depth -= 1;
                }
            }
        }
        // The source output uses virtual spans; match each forwarded input by
        // its retained origin before inspecting function boundaries.
        let source_holes = Vec::new();
        let mut delayed = Delayed {
            depth: 0,
            holes: &source_holes,
            query: &query,
            invalid: false,
        };
        // Populate the source occurrences using their immutable Syntax origins.
        struct HoleSpans<'a> {
            scoped: &'a ScopedSource,
            inputs: &'a [Syntax],
            query: &'a TypedMacroQuery,
            out: Vec<(Span, Span)>,
        }
        impl Visitor for HoleSpans<'_> {
            fn expr(&mut self, e: &mut ast::Expr, enter: bool) {
                if enter
                    && let Some(o) = self.scoped.origin(e.span)
                    && let Some(s) = self.inputs.iter().find(|s| s.same_input(&o.syntax))
                {
                    if let Some(span)=self.query.input_span(s) {self.out.push((e.span,span));}
                }
            }
        }
        let mut source = syntax.clone();
        let mut spans = HoleSpans {
            scoped: &scoped,
            inputs: &input_nodes,
            query: &query,
            out: Vec::new(),
        };
        source.walk(&mut spans);
        delayed.holes = &spans.out;
        source.walk(&mut delayed);
        if delayed.invalid {
            return Err(diagnostic(
                program,
                module,
                span,
                "a typed macro cannot delay an argument that has errors or needs an environment",
            ));
        }
        check_dependencies(program, module, span, &expression)?;
        Ok(Output {
            expression,
            syntax,
            scoped,
            var_count,
            ty_var_count,
            holes,
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
                if let hir::ExprKind::Name(hir::ValueRef::Def(d)) = e.kind {
                    if let Some(j) = groups.get(&d) {
                        work.push(*j);
                    }
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
