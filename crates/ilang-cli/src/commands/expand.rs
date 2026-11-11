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
    let expanded = match ilang_macro::project::expand(&mut graph) {
        Ok(expanded) => expanded,
        Err(more) => {
            diags.extend(more);
            for line in ilang_cli::diagnostic_lines(&graph, &diags) {
                eprintln!("{line}");
            }
            return Ok(ExitCode::FAILURE);
        }
    };
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
        let lisp = ilang_syntax::lisp::print_module(&module.ast);
        let text = if to == "is" {
            ilang_syntax::lisp::format(&lisp).expect("expanded AST prints valid Lisp")
        } else {
            let document =
                ilang_syntax::lisp::parse(&lisp).expect("expanded AST prints valid Lisp");
            ilang_syntax::lisp::normal_source(&document).map_err(|error| {
                anyhow::anyhow!(
                    "{}: {}; use --to is for quotations or other Lisp-only forms",
                    module.path,
                    error.message
                )
            })?
        };
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
