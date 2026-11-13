//! The parts of `item_phase` that read or write the tree, on the tree of
//! syntax three. The scheduler of the item phase (`expand_items`,
//! `item_step`) does not look inside an item; it calls these.
// Nothing calls these until the item phase is switched to the new tree.
#![allow(dead_code)]

use crate::{
    body_engine::{BodyEngine, Error},
    body_engine3::item_arguments,
    body_walk3::{self, Hook, Local},
    item_phase::PhaseError,
    session::Transaction,
};
use ilang_syntax::lisp::{
    Datum, DatumKind,
    expansion::MacroArgument,
    scoped::ScopedSource,
    syntax::{Node, Syntax},
};
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::{
    Attribute, DItem, Export, Expr, ExprKind, Ident, Item, ItemKind, Module, Pattern, Type,
};

pub fn builtin(attribute: &Attribute) -> bool {
    attribute.module.is_none()
        && matches!(
            attribute.name.name.as_str(),
            "external" | "js" | "host" | "deprecated" | "macro" | "main" | "wasm"
        )
}

/// The names that an item defines.
pub fn item_names(item: &Item) -> Vec<String> {
    match &item.kind {
        ItemKind::TypeDef(t) => vec![t.name.name.clone()],
        ItemKind::Decl { name, .. } => vec![name.name.clone()],
        ItemKind::Bind(b) => {
            let mut out = Vec::new();
            body_walk3::for_each_binder(&b.pattern, |name, _| out.push(name.to_owned()));
            out
        }
        ItemKind::Macro { .. } => Vec::new(),
    }
}

/// The arguments of an attribute that are all plain strings (and at least
/// one): the old tree keeps these as texts, and they are passed as written,
/// with no look at what made them.
fn plain_strings(args: &[DItem]) -> Option<Vec<(&str, Span)>> {
    if args.is_empty() {
        return None;
    }
    args.iter()
        .map(|arg| match arg {
            DItem::Pos(Expr {
                kind: ExprKind::String(text),
                span,
            }) => Some((text.as_str(), span.0)),
            _ => None,
        })
        .collect()
}

/// An expansion cannot define a macro.
pub fn reject_macro_definitions(items: &[Item]) -> Result<(), Error> {
    for item in items {
        if let Some(attribute) = item
            .attributes
            .iter()
            .find(|a| a.module.is_none() && a.name.name == "macro")
        {
            return Err(Error::InvalidItem(
                attribute.span.0,
                "an expansion cannot define a macro".into(),
            ));
        }
    }
    Ok(())
}

/// The module with the same header and no items.
pub fn header(module: &Module) -> Module {
    Module {
        doc: module.doc.clone(),
        imports: module.imports.clone(),
        exports: module.exports.clone(),
        items: Vec::new(),
    }
}

impl BodyEngine<'_> {
    pub(crate) fn attribute_arguments3(&self, attribute: &Attribute) -> Vec<MacroArgument> {
        let Some(args) = &attribute.args else {
            return Vec::new();
        };
        match plain_strings(args) {
            Some(strings) => strings
                .into_iter()
                .map(|(text, span)| MacroArgument {
                    label: None,
                    syntax: Syntax::from_datum(
                        &Datum {
                            kind: DatumKind::String(text.to_owned()),
                            span,
                            docs: Vec::new(),
                        },
                        &self.scope(),
                    ),
                })
                .collect(),
            None => item_arguments(args, |e| self.node_syntax(e)),
        }
    }

    /// The arguments of an item call `[module.]name(data)`.
    pub(crate) fn item_call_arguments3(&self, args: &[DItem]) -> Vec<MacroArgument> {
        item_arguments(args, |e| self.node_syntax(e))
    }

    /// The root syntax of an item: its override, what a macro made it from,
    /// or the written item encoded.
    pub(crate) fn item_root3(&self, item: &Item) -> (Syntax, Span) {
        if let Some(root) = self.item_overrides.get(&item.span.0) {
            return root.clone();
        }
        (self.node_syntax(item), item.span.0)
    }

    /// Whether an item needs no run of the item phase.
    pub(crate) fn item_is_finished_without_run3(&self, item: &Item, ready: bool) -> bool {
        if matches!(item.kind, ItemKind::Macro { .. }) {
            return false;
        }
        if matches!(item.kind, ItemKind::TypeDef(_)) && !ready {
            return false;
        }
        !item.attributes.iter().any(|a| {
            !builtin(a)
                && self
                    .resolve3(a.module.as_ref(), &a.name, &self.item_locals, 0)
                    .is_some()
        })
    }

    /// The values that the items made by macros define: they are the locals
    /// of the item phase.
    pub(crate) fn generated_values3<'i>(
        &self,
        items: impl IntoIterator<Item = &'i Item>,
    ) -> Vec<Local> {
        let mut out = Vec::new();
        for item in items {
            match &item.kind {
                ItemKind::Bind(binding) => {
                    body_walk3::for_each_binder(&binding.pattern, |name, span| {
                        if self.scoped.is_generated(span) {
                            out.push(Local {
                                name: name.to_owned(),
                                span,
                                fn_only: None,
                            });
                        }
                    })
                }
                ItemKind::Decl { name, .. } if self.scoped.is_generated(name.span.0) => {
                    out.push(Local {
                        name: name.name.clone(),
                        span: name.span.0,
                        fn_only: None,
                    })
                }
                _ => {}
            }
        }
        out
    }
}

/// A written type is visible immediately, and is ready as soon as none of
/// its type positions names a macro; no macro is executed by this probe.
pub(crate) fn type_ready(engine: &mut BodyEngine<'_>, item: &Item) -> Result<bool, PhaseError> {
    let ItemKind::TypeDef(definition) = &item.kind else {
        return Ok(false);
    };
    struct Probe<'a, 'b>(&'a mut BodyEngine<'b>);
    enum Found {
        Macro,
        Error(Error),
    }
    impl Hook for Probe<'_, '_> {
        type Error = Found;
        fn is_macro(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<bool, Found> {
            Ok(false)
        }
        fn expand(&mut self, _: &Expr, _: &[Local], _: u32) -> Result<Expr, Found> {
            unreachable!("type probe does not expand expressions")
        }
        fn is_type_macro(
            &mut self,
            ty: &Type,
            locals: &[Local],
            depth: u32,
        ) -> Result<bool, Found> {
            match Hook::is_type_macro(self.0, ty, locals, depth) {
                Ok(true) => Err(Found::Macro),
                Ok(false) => Ok(false),
                Err(error) => Err(Found::Error(error)),
            }
        }
        fn expand_type(&mut self, _: &Type, _: &[Local], _: u32) -> Result<Type, Found> {
            unreachable!("type probe stops before expansion")
        }
        fn depth_error(&mut self, span: Span) -> Found {
            Found::Error(Error::Depth(span))
        }
        fn is_direct(&mut self, _: &Expr, _: &[Local], _: u32) -> bool {
            false
        }
    }
    match body_walk3::type_definition(&mut definition.clone(), &mut Probe(engine)) {
        Ok(()) => Ok(true),
        Err(Found::Macro) => Ok(false),
        Err(Found::Error(error)) => Err(PhaseError::Step(error)),
    }
}

/// Export requests are checked only against the completed item phase. They do
/// not mutate the import graph and cannot publish a marked or foreign name.
pub(crate) fn commit_exports(
    module: &mut Module,
    transactions: &[Transaction],
    source: u32,
    scoped: &ScopedSource,
) -> Result<(), PhaseError> {
    for request in transactions
        .iter()
        .flat_map(|transaction| &transaction.requests)
    {
        let ilang_host::syntax::ItemRequest::Expose { name } = request else {
            continue;
        };
        let Node::Name(text) = name.node() else {
            return Err(PhaseError::Step(Error::InvalidItem(
                name.span(),
                "an export request needs a value name".into(),
            )));
        };
        if name.scope().module != source
            || !name.scope().marks.is_empty()
            || name.scope_identity().is_some()
        {
            return Err(PhaseError::Step(Error::InvalidItem(
                name.span(),
                "an export request needs an unmarked name in this module".into(),
            )));
        }
        if !defines_unmarked(module, text, source, scoped) {
            return Err(PhaseError::Step(Error::InvalidItem(
                name.span(),
                format!("the requested export `{text}` is not defined in this module"),
            )));
        }
        add_export(module, text, name.span());
    }
    Ok(())
}

/// Whether the module defines the value `text` with a name that no macro
/// marked.
pub fn defines_unmarked(module: &Module, text: &str, source: u32, scoped: &ScopedSource) -> bool {
    let unmarked = |span: Span| {
        scoped.origin(span).is_none_or(|origin| {
            origin.syntax.scope().module == source && origin.syntax.scope().marks.is_empty()
        })
    };
    module.items.iter().any(|item| match &item.kind {
        ItemKind::Bind(binding) => unmarked_binder(&binding.pattern, text, unmarked),
        ItemKind::Decl { name, .. } => name.name == *text && unmarked(name.span.0),
        _ => false,
    })
}

fn unmarked_binder(pattern: &Pattern, wanted: &str, unmarked: impl Fn(Span) -> bool) -> bool {
    let mut found = false;
    body_walk3::for_each_binder(pattern, |text, span| {
        if text == wanted && unmarked(span) {
            found = true;
        }
    });
    found
}

/// Adds the export of a value, once.
pub fn add_export(module: &mut Module, text: &str, span: Span) {
    if !module
        .exports
        .iter()
        .any(|export| export.name.name == *text)
    {
        module.exports.push(Export {
            name: Ident::new(text, span),
            constructors: false,
            span: span.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(source: &str) -> Module {
        let (module, errors) = ilang_syntax::syn3::parse_module(source);
        assert!(errors.is_empty(), "{errors:?}");
        module
    }

    #[test]
    fn item_names_are_the_binders_and_the_defined_names() {
        let m = module("T = (Int)\nx: Int\n(a, C(k = b)) = f()\nm(1)");
        let names: Vec<_> = m.items.iter().map(item_names).collect();
        assert_eq!(names, [vec!["T"], vec!["x"], vec!["a", "b", "k"], vec![]]);
    }

    #[test]
    fn plain_string_arguments_are_told_from_a_call() {
        let m = module("@a(\"x\", \"y\")\n@b(\"x\", 1)\n@c()\n@d\n@e(k = \"x\")\nf = 1");
        let shapes: Vec<_> = m.items[0]
            .attributes
            .iter()
            .map(|a| a.args.as_deref().map(|args| plain_strings(args).is_some()))
            .collect();
        assert_eq!(
            shapes,
            [Some(true), Some(false), Some(false), None, Some(false)]
        );
        assert!(builtin(&module("@main\nf = 1").items[0].attributes[0]));
        assert!(!builtin(&m.items[0].attributes[0]));
    }

    #[test]
    fn an_export_is_added_once_for_a_defined_value() {
        let mut m = module("(a)\nf = 1\ng: Int");
        let scoped = ScopedSource::new(String::new());
        assert!(defines_unmarked(&m, "f", 0, &scoped));
        assert!(defines_unmarked(&m, "g", 0, &scoped));
        assert!(!defines_unmarked(&m, "h", 0, &scoped));
        add_export(&mut m, "f", Span::default());
        add_export(&mut m, "f", Span::default());
        add_export(&mut m, "a", Span::default());
        let names: Vec<_> = m.exports.iter().map(|e| e.name.name.as_str()).collect();
        assert_eq!(names, ["a", "f"]);
        assert!(header(&m).items.is_empty());
        assert_eq!(header(&m).exports.len(), 2);
    }

    #[test]
    fn a_result_that_defines_a_macro_is_rejected() {
        let m = module("@macro\nf = (a) { a }\ng = 1");
        assert!(reject_macro_definitions(&m.items).is_err());
        assert!(reject_macro_definitions(&m.items[1..]).is_ok());
    }
}
