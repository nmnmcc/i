//! Old node assertions over the production native parser and validation.
#![allow(dead_code)]
#[allow(unused_imports)]
pub use migration_syntax::*;
extern crate ilang_syntax as native;

/// Keep the frozen validation oracle for the formatter as its tree comparison
/// moves to the native AST. Project only the new §7.1 pattern forms into the
/// old model, retaining their children and every other validation diagnostic.
pub fn check_native_validation(source: &str, module: &native::ast::Module) {
    use native::ast::{DPItem, Pattern, PatternKind as P};
    use native::walk::{self, Visitor};
    fn data_core(p: &Pattern) -> bool {
        match &p.kind {
            P::Data(_) => true,
            P::And(parts) => parts
                .iter()
                .find(|p| !matches!(p.kind, P::Var(_) | P::Discard(_)))
                .is_some_and(|p| matches!(p.kind, P::Data(_))),
            _ => false,
        }
    }
    #[derive(Default)]
    struct Project {
        order_errors: Vec<native::span::Span>,
    }
    impl Visitor for Project {
        fn pattern(&mut self, p: &mut Pattern, enter: bool) {
            if enter {
                if let P::Data(items) = &p.kind {
                    let mut labelled = false;
                    for item in items {
                        match item {
                            DPItem::Key(..) => labelled = true,
                            DPItem::Pos(p) if labelled => self.order_errors.push(p.span.0),
                            _ => {}
                        }
                    }
                }
                return;
            }
            match &mut p.kind {
                P::Data(items) => {
                    for item in items {
                        if matches!(item, DPItem::Key(..)) {
                            let DPItem::Key(_, p) = std::mem::replace(item, DPItem::Rest(None))
                            else {
                                unreachable!()
                            };
                            *item = DPItem::Pos(p);
                        }
                    }
                }
                P::Ctor {
                    args: Some(items), ..
                } => {
                    for item in items {
                        if matches!(item, DPItem::Key(_, p) if data_core(p)) {
                            let DPItem::Key(_, p) = std::mem::replace(item, DPItem::Rest(None))
                            else {
                                unreachable!()
                            };
                            *item = DPItem::Pos(p);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    native::deep::deep(|| {
        let mut projected = module.clone();
        let mut projection = Project::default();
        walk::module(&mut projected, &mut projection);
        let (_, old_errors) = migration_syntax::syn3::lower::lower(&projected);
        let (_, validation) = native::syn3::facts::Transition::capture_module(module, false);
        let validation: Vec<_> = validation
            .into_iter()
            .filter(|d| {
                if d.code == native::Code::Unexpected
                    && d.message == "an item by position may not come after an item by label (§7.1)"
                {
                    assert!(
                        projection.order_errors.contains(&d.span),
                        "{source:?}: {d:?}"
                    );
                    false
                } else {
                    true
                }
            })
            .collect();
        let records = |ds: &[Diagnostic]| {
            ds.iter()
                .map(|d| (d.code, d.span, d.message.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            records(&validation),
            records(&old_errors),
            "native validation, with only §7.1 projected, of {source:?}"
        );
    });
}

pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (module, mut errors) = native::parse_module(source);
    let (old, old_errors) = migration_syntax::syn3::lower::lower(&module);
    let (_, validation) = native::syn3::facts::Transition::capture_module(&module, false);
    let records = |ds: &[Diagnostic]| {
        ds.iter()
            .map(|d| (d.code, d.span, d.message.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        records(&validation),
        records(&old_errors),
        "native validation of {source:?}"
    );
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (old, errors)
}
