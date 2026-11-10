//! Prepare a macro module's parsed import closure for compiler execution.
use ilang_check::Parsed;
use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::lisp::{expansion, quote_lower, syntax::Syntax};

/// One source macro and its written argument names.
#[derive(Clone, Debug)]
pub struct MacroBinding {
    pub module: String,
    pub name: String,
    pub parameters: Vec<String>,
}

/// Macro bindings and source-located template nodes for one compiler program.
pub struct Prepared {
    pub bindings: Vec<MacroBinding>,
    pub templates: Vec<Syntax>,
}

/// Validate macro declarations, remove their phase markers for the compiler
/// program, and lower quotes. `parsed` is the exact import closure selected by
/// the caller; module indices are the same as those used by the resolver.
pub fn prepare(graph: &PackageGraph, parsed: &mut [Parsed]) -> Result<Prepared, Vec<Diagnostic>> {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    // Validate the whole closure before mutating it, so an invalid declaration
    // never leaves a partially transformed module in the caller's state.
    for module in parsed.iter() {
        let lisp = graph.sources.get(module.source).display.ends_with(".is");
        match expansion::definitions(&module.ast, lisp) {
            Ok(bindings) => found.push(bindings),
            Err(problems) => {
                errors.extend(problems.into_iter().map(|e| {
                    Diagnostic::error(
                        ilang_check::code::ATTRIBUTE_PLACE,
                        Some(module.source),
                        e.span,
                        e.message,
                    )
                }));
                found.push(Vec::new());
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut bindings = Vec::new();
    let mut templates = Vec::new();
    for (index, (module, definitions)) in parsed.iter_mut().zip(found).enumerate() {
        for definition in definitions {
            let item = &mut module.ast.items[definition.item];
            item.attributes
                .retain(|a| !(a.module.is_none() && a.name.name == "macro"));
            bindings.push(MacroBinding {
                module: module.path.clone(),
                name: definition.name,
                parameters: definition.parameters,
            });
        }
        quote_lower::lower_expression_quotes(&mut module.ast, index as u32, &mut templates);
    }
    Ok(Prepared {
        bindings,
        templates,
    })
}
