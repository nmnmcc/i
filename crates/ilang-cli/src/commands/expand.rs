//! Expansion is the same sandbox compiler pipeline used by check and build.
use anyhow::{Result, bail};
use ilang_package::Diagnostic;
use std::{path::Path, process::ExitCode};

pub fn run(
    project: &Path,
    offline: bool,
    to: &str,
    trace: bool,
    selected: Option<&str>,
) -> Result<ExitCode> {
    let (mut graph, mut diags) = ilang_cli::load(project, true, offline);
    let Some(root) = graph.root else {
        for line in ilang_cli::diagnostic_lines(&graph, &diags) {
            eprintln!("{line}");
        }
        return Ok(ExitCode::FAILURE);
    };
    if diags.iter().any(Diagnostic::is_error) {
        for line in ilang_cli::diagnostic_lines(&graph, &diags) {
            eprintln!("{line}");
        }
        return Ok(ExitCode::FAILURE);
    }
    let mut expanded = match ilang_macro::project::expand(&mut graph) {
        Ok(expanded) => expanded,
        Err(more) => {
            diags.extend(more);
            for line in ilang_cli::diagnostic_lines(&graph, &diags) {
                eprintln!("{line}");
            }
            return Ok(ExitCode::FAILURE);
        }
    };
    let mut for_check = expanded.parsed.clone();
    let mut templates = Vec::new();
    for module in &mut for_check {
        ilang_syntax::lisp::quote_lower::lower_expression_quotes(
            &mut module.ast,
            module.source.0,
            &mut templates,
        );
    }
    let checked = ilang_check::check_parsed_for_tools(&graph, &for_check);
    diags.extend(checked.diags);
    if diags.iter().any(Diagnostic::is_error) {
        for line in ilang_cli::diagnostic_lines(&graph, &diags) {
            eprintln!("{line}");
        }
        return Ok(ExitCode::FAILURE);
    }
    let typed = checked.types.expect("expanded program checked");
    use ilang_syntax::walk::{Visitor, Walk};
    struct Replace<'a> {
        source: u32,
        module: ilang_check::hir::ModuleId,
        typed: &'a ilang_check::types::Typed,
        scoped: Option<ilang_syntax::lisp::scoped::ScopedSource>,
    }
    impl Visitor for Replace<'_> {
        fn expr(&mut self, e: &mut ilang_syntax::ast::Expr, enter: bool) {
            if !enter {
                return;
            }
            let original = self
                .scoped
                .as_ref()
                .and_then(|s| s.origin(e.span))
                .filter(|o| {
                    o.syntax.scope().module == self.source && o.syntax.scope().marks.is_empty()
                })
                .map(|o| o.syntax.span());
            if let Some(output) = self
                .typed
                .typed_expansions
                .get(&(self.module, e.span))
                .or_else(|| {
                    original.and_then(|span| self.typed.typed_expansions.get(&(self.module, span)))
                })
            {
                *e = output.syntax.clone();
            }
        }
    }
    for (i, module) in expanded.parsed.iter_mut().enumerate() {
        let id = ilang_check::hir::ModuleId(i as u32);
        let scoped = typed
            .typed_expansions
            .iter()
            .filter(|((m, _), _)| *m == id)
            .map(|(_, o)| o.scoped.clone())
            .max_by_key(|s| s.text().len());
        module.ast.walk(&mut Replace {
            source: module.source.0,
            module: id,
            typed: &typed,
            scoped,
        });
    }
    if trace {
        for ((module, span), output) in &typed.typed_expansions {
            let source = expanded.parsed[module.0 as usize].source;
            let file = graph.sources.get(source);
            let (line, column) =
                ilang_syntax::span::LineIndex::new(&file.text).line_col(span.start);
            for item in &output.trace {
                eprintln!("{}:{line}:{column}: {item}", file.display);
            }
        }
    }
    diags.extend(expanded.diagnostics);
    for line in ilang_cli::diagnostic_lines(&graph, &diags) {
        eprintln!("{line}");
    }
    let modules: Vec<_> = expanded
        .parsed
        .iter()
        .filter(|module| {
            module.package == root
                && selected.is_none_or(|name| {
                    module.path == name
                        || graph.sources.get(module.source).display == name
                        || project.join(&graph.sources.get(module.source).display)
                            == Path::new(name)
                })
        })
        .collect();
    if modules.is_empty() {
        bail!(
            "no source module matches {}",
            selected.unwrap_or("this package")
        );
    }
    // Render completely before printing, so an unrepresentable module does not
    // leave a partial source document on stdout.
    let mut output = String::new();
    for module in modules {
        // spec §6.8: "a quote and a macro definition have no normal spelling".
        if to != "is" && lisp_only(&module.ast) {
            bail!(
                "{}: a quotation or a macro definition has no normal spelling; use --to is for quotations or other Lisp-only forms",
                graph.sources.get(module.source).display
            );
        }
        let text = ilang_syntax::syn3::print_current(&module.ast, to == "is");
        output.push_str(&format!("// {}\n{text}\n", module.path));
    }
    if trace {
        for transaction in expanded.transactions {
            let (source, span) = transaction
                .location
                .expect("completed expansion has a call location");
            let file = graph.sources.get(source);
            let (line, column) =
                ilang_syntax::span::LineIndex::new(&file.text).line_col(span.start);
            eprintln!("{}:{line}:{column}: {}", file.display, transaction.trace);
        }
    }
    print!("{output}");
    Ok(ExitCode::SUCCESS)
}

/// Whether a module has a quotation or a macro definition, which only the
/// Lisp surface can write (spec §6.8).
fn lisp_only(module: &ilang_syntax::ast::Module) -> bool {
    use ilang_syntax::ast::{Expr, ExprKind, Pattern, PatternKind};
    use ilang_syntax::walk::{Visitor, Walk};
    struct Find(bool);
    impl Visitor for Find {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            self.0 |= enter && matches!(e.kind, ExprKind::Quote(_));
        }
        fn pattern(&mut self, p: &mut Pattern) {
            self.0 |= matches!(p.kind, PatternKind::Quote { .. });
        }
    }
    if module.items.iter().any(|item| {
        item.attributes
            .iter()
            .any(|a| a.module.is_none() && a.name.name == "macro")
    }) {
        return true;
    }
    let mut find = Find(false);
    module.clone().walk(&mut find);
    find.0
}
