//! Project-wide, dependency-ordered syntax expansion.
use crate::{
    body_engine::BodyEngine,
    body_walk, collections,
    definition_world::DefinitionWorld,
    diagnostics,
    item_phase::{self, PhaseError},
    module_order,
    registry::Registry,
    session::{DefinitionResolver, Session, Transaction},
    validate,
};
use ilang_check::Parsed;
use ilang_package::{Diagnostic, PackageGraph, SourceId};
use ilang_syntax::lisp::scoped::ScopedSource;
use std::sync::Arc;

pub struct Expanded {
    pub parsed: Vec<Parsed>,
    pub diagnostics: Vec<Diagnostic>,
    pub transactions: Vec<Transaction>,
    /// Source-scoped quotation templates for a runtime test instance.
    pub templates: Vec<ilang_syntax::lisp::syntax::Syntax>,
}

impl Expanded {
    /// Lower source quotations only after expansion has finished. The original
    /// expanded tree remains available to callers such as `i expand` until
    /// this method is called.
    pub fn lower_quotes(&mut self) {
        for module in &mut self.parsed {
            ilang_syntax::lisp::quote_lower::lower_expression_quotes(
                &mut module.ast,
                module.source.0,
                &mut self.templates,
            );
        }
    }
}

/// Parse and expand each module after its dependencies. On failure the
/// diagnostics are returned without publishing partial output to the checker.
pub fn expand(graph: &mut PackageGraph) -> Result<Expanded, Vec<Diagnostic>> {
    let (mut parsed, mut diagnostics) = ilang_check::parse(graph);
    if diagnostics.iter().any(Diagnostic::is_error) {
        return Err(diagnostics);
    }
    if !parsed.iter().any(|module| {
        module.ast.items.iter().any(|item| {
            item.attributes
                .iter()
                .any(|attribute| attribute.module.is_none() && attribute.name.name == "macro")
        })
    }) {
        return Ok(Expanded {
            parsed,
            diagnostics,
            transactions: Vec::new(),
            templates: Vec::new(),
        });
    }
    let order = match module_order::order(graph, &parsed) {
        Ok(order) => order,
        Err(_) => {
            diagnostics.extend(ilang_check::check_parsed(graph, &parsed).diags);
            return Err(diagnostics);
        }
    };
    let mut transactions = Vec::new();
    let mut session = Session::default();
    let next_answer = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut definition_modules: Vec<_> = parsed.iter().cloned().map(Arc::new).collect();
    for source in order {
        let index = parsed
            .iter()
            .position(|module| module.source == source)
            .expect("ordered module belongs to project");
        let registry = match Registry::new(graph, &parsed) {
            Ok(registry) => registry,
            Err(errors) => {
                diagnostics.extend(errors);
                return Err(diagnostics);
            }
        };
        // Definition queries can name the caller and its transitive imports,
        // including the modules that supplied macro templates. Freeze that
        // existing import closure rather than copying unrelated project ASTs.
        let definition_snapshot = match crate::prepare::import_indices(graph, &parsed, source) {
            Ok(indices) => indices
                .into_iter()
                .map(|i| definition_modules[i].clone())
                .collect(),
            Err(errors) => {
                diagnostics.extend(errors);
                return Err(diagnostics);
            }
        };
        let definitions = Arc::new(DefinitionWorld::new_shared(
            graph,
            definition_snapshot,
            next_answer.clone(),
        ));
        let mut ast = parsed[index].ast.clone();
        let scoped = parsed[index]
            .scoped
            .clone()
            .unwrap_or_else(|| ScopedSource::new(graph.sources.get(source).text.clone()));
        let initial: Arc<DefinitionResolver> =
            Arc::new(|_, _| panic!("definition resolver must be installed before execution"));
        let mut engine = BodyEngine::new(
            graph,
            &parsed,
            &registry,
            source,
            scoped,
            initial,
            &mut session,
        );
        let item_transactions =
            match item_phase::expand_items(&mut engine, &mut ast, definitions.clone()) {
                Ok(transactions) => transactions,
                Err(error) => {
                    let scoped = engine.failure_scoped().unwrap_or(&engine.scoped);
                    diagnostics.extend(map_failure(
                        source,
                        scoped,
                        engine
                            .failed_notes()
                            .iter()
                            .map(|note| {
                                Diagnostic::note(
                                    Some(SourceId(note.origin.source)),
                                    note.origin.call,
                                    note.message.clone(),
                                )
                            })
                            .collect(),
                    ));
                    diagnostics.extend(map_failure(source, scoped, phase(source, error)));
                    return Err(diagnostics);
                }
            };
        let scoped = engine.scoped.clone();
        let replacements = engine.type_replacements.clone();
        let item_overrides = engine.item_overrides.clone();
        drop(engine);
        graph.sources.set_text(source, scoped.text().to_owned());
        parsed[index].ast = ast;
        parsed[index].scoped = Some(scoped);
        let registry = match Registry::new(graph, &parsed) {
            Ok(registry) => registry,
            Err(errors) => {
                diagnostics.extend(errors);
                return Err(diagnostics);
            }
        };
        let mut ast = parsed[index].ast.clone();
        let scoped = parsed[index]
            .scoped
            .clone()
            .expect("item phase installs source map");
        let initial: Arc<DefinitionResolver> =
            Arc::new(|_, _| panic!("definition resolver must be installed before execution"));
        let mut engine = BodyEngine::new(
            graph,
            &parsed,
            &registry,
            source,
            scoped,
            initial,
            &mut session,
        );
        engine.type_replacements = replacements;
        engine.item_overrides = item_overrides;
        engine.use_completed_definitions(definitions, &ast);
        if let Err(error) = body_walk::expressions(&mut ast, &mut engine) {
            diagnostics.extend(map_failure(
                source,
                &engine.scoped,
                notes(item_transactions.iter().chain(engine.transactions.iter())),
            ));
            diagnostics.extend(map_failure(
                source,
                &engine.scoped,
                diagnostics::body(source, error),
            ));
            return Err(diagnostics);
        }
        let scoped = engine.scoped.clone();
        let body_transactions = std::mem::take(&mut engine.transactions);
        drop(engine);
        graph.sources.set_text(source, scoped.text().to_owned());
        parsed[index].ast = ast;
        parsed[index].scoped = Some(scoped);
        definition_modules[index] = Arc::new(parsed[index].clone());
        diagnostics.extend(notes(
            item_transactions.iter().chain(body_transactions.iter()),
        ));
        transactions.extend(item_transactions);
        transactions.extend(body_transactions);
        if let Err(errors) = validate::module(graph, &parsed, source) {
            diagnostics.extend(errors);
            return Err(diagnostics);
        }
    }
    Ok(Expanded {
        parsed,
        diagnostics,
        transactions,
        templates: Vec::new(),
    })
}
fn notes<'a>(transactions: impl IntoIterator<Item = &'a Transaction>) -> Vec<Diagnostic> {
    transactions
        .into_iter()
        .flat_map(|transaction| {
            let (source, span) = transaction
                .location
                .expect("completed transaction has a call location");
            transaction
                .notes
                .iter()
                .map(move |message| Diagnostic::note(Some(source), span, message.clone()))
        })
        .collect()
}
fn phase(source: SourceId, error: PhaseError) -> Vec<Diagnostic> {
    match error {
        PhaseError::Step(error) => diagnostics::body(source, error),
        PhaseError::Waiting(queries) => queries
            .into_iter()
            .map(|query| {
                Diagnostic::error(
                    ilang_check::code::MACRO_RESERVED,
                    Some(SourceId(query.scope().module)),
                    query.span(),
                    "cyclic macro type-definition query",
                )
            })
            .collect(),
        PhaseError::Changed { query, .. } => vec![Diagnostic::error(
            ilang_check::code::MACRO_RESERVED,
            Some(SourceId(query.scope().module)),
            query.span(),
            "an early macro type-definition answer changed during expansion",
        )],
    }
}

/// A checked project and its complete, ordered collection contributions.
/// DefIds remain opaque and valid only for this checked program.
pub struct ProjectChecked {
    pub program: Option<ilang_check::hir::Program>,
    pub types: Option<ilang_check::types::Typed>,
    pub diags: Vec<Diagnostic>,
    pub collections: Vec<collections::Entry>,
    /// Source-scoped quote templates for test instances of this project.
    pub templates: Vec<ilang_syntax::lisp::syntax::Syntax>,
}

/// Expand and type-check a project through the same path used by builds.
pub fn check(graph: &mut PackageGraph) -> ProjectChecked {
    check_project(graph, false)
}

/// Check collection contributions before selecting an execution root.
pub fn check_for_tools(graph: &mut PackageGraph) -> ProjectChecked {
    check_project(graph, true)
}

/// Expand and check a package with all targets of one test collection as roots.
pub fn check_tests(graph: &mut PackageGraph, collection: Option<&str>) -> ProjectChecked {
    match expand(graph) {
        Ok(mut expanded) => {
            expanded.lower_quotes();
            let (program, mut diags) = ilang_check::resolve_parsed(graph, &expanded.parsed);
            let mut entries = Vec::new();
            if let Some(program) = &program
                && !diags.iter().any(Diagnostic::is_error)
            {
                match collections::resolve_with_types(
                    &expanded.parsed,
                    program,
                    None,
                    &expanded.transactions,
                ) {
                    Ok(found) => entries = found,
                    Err(errors) => diags.extend(errors),
                }
            }
            let roots: Vec<_> = entries
                .iter()
                .filter(|e| {
                    let program = program.as_ref().expect("resolved collection program");
                    let name = format!(
                        "{}.{}",
                        program.module(e.collection.module).path,
                        program.def(e.collection).name
                    );
                    Some(name.as_str()) == collection
                })
                .map(|e| (e.target, true))
                .collect();
            let mut checked = ilang_check::check_resolved_selected(graph, program, diags, &roots);
            if let Some(typed) = &checked.types
                && !checked.diags.iter().any(Diagnostic::is_error)
            {
                for entry in &mut entries {
                    entry.signature = typed
                        .schemes
                        .get(&entry.target)
                        .expect("checked target has a type scheme")
                        .clone();
                }
            }
            if let Some(program) = &checked.program {
                ilang_check::map_expansion_diagnostics(
                    program,
                    &graph.sources,
                    &mut expanded.diagnostics,
                );
            }
            expanded.diagnostics.append(&mut checked.diags);
            ProjectChecked {
                program: checked.program,
                types: checked.types,
                diags: expanded.diagnostics,
                collections: entries,
                templates: expanded.templates,
            }
        }
        Err(diagnostics) => ProjectChecked {
            program: None,
            types: None,
            diags: diagnostics,
            collections: Vec::new(),
            templates: Vec::new(),
        },
    }
}

fn check_project(graph: &mut PackageGraph, tools: bool) -> ProjectChecked {
    match expand(graph) {
        Ok(mut expanded) => {
            expanded.lower_quotes();
            let mut checked = if tools {
                ilang_check::check_parsed_for_tools(graph, &expanded.parsed)
            } else {
                ilang_check::check_parsed(graph, &expanded.parsed)
            };
            let mut entries = Vec::new();
            if let (Some(program), Some(typed)) = (&checked.program, &checked.types)
                && !checked.diags.iter().any(Diagnostic::is_error)
            {
                match collections::resolve(&expanded.parsed, program, typed, &expanded.transactions)
                {
                    Ok(found) => entries = found,
                    Err(errors) => checked.diags.extend(errors),
                }
            }
            if let Some(program) = &checked.program {
                ilang_check::map_expansion_diagnostics(
                    program,
                    &graph.sources,
                    &mut expanded.diagnostics,
                );
            }
            expanded.diagnostics.append(&mut checked.diags);
            ProjectChecked {
                program: checked.program,
                types: checked.types,
                diags: expanded.diagnostics,
                collections: entries,
                templates: expanded.templates,
            }
        }
        Err(diagnostics) => ProjectChecked {
            program: None,
            types: None,
            diags: diagnostics,
            collections: Vec::new(),
            templates: Vec::new(),
        },
    }
}

fn map_failure(
    source: SourceId,
    scoped: &ScopedSource,
    mut diagnostics: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    for diagnostic in &mut diagnostics {
        if diagnostic.source != Some(source) || !scoped.is_generated(diagnostic.span) {
            continue;
        }
        let origin = scoped
            .origin(diagnostic.span)
            .expect("expanded diagnostic has an origin");
        let syntax = &origin.syntax;
        let template = syntax.scope().module != source.0 || !syntax.scope().marks.is_empty();
        if template {
            let primary = origin
                .call_chain
                .first()
                .map_or(&origin.call_site, |call| &call.syntax);
            diagnostic.related.push(ilang_package::DiagnosticNote {
                source: SourceId(syntax.scope().module),
                span: syntax.span(),
                message: "expanded from this template".into(),
            });
            diagnostic.source = Some(SourceId(primary.scope().module));
            diagnostic.span = primary.span();
        } else {
            diagnostic.source = Some(SourceId(syntax.scope().module));
            diagnostic.span = syntax.span();
        }
    }
    diagnostics
}
