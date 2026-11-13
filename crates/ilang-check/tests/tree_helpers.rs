//! `common::tree::new` changes the same places as `common::tree::old`: a
//! change made in the tree of syntax three and then lowered is the change
//! made in the lowered tree (docs/design/syn3-final-ast.md, step 2d).
mod common;

use common::tree::{new, old};
use ilang_syntax::shape::equivalent;
use ilang_syntax::syn3::{self, lower};

/// The module of `source` in the two trees.
fn both(source: &str) -> (syn3::ast::Module, ilang_syntax::ast::Module) {
    let (module, errors) = syn3::parse_module(source);
    assert!(errors.is_empty(), "{source}: {errors:?}");
    let (lowered, errors) = lower::lower(&module);
    assert!(errors.is_empty(), "{source}: {errors:?}");
    (module, lowered)
}

/// An expression in the two trees.
fn expression(source: &str) -> (syn3::ast::Expr, ilang_syntax::ast::Expr) {
    let (mut module, _) = both(&format!("value = {source}\n"));
    let expr = new::value(&mut module, 0).clone();
    let lowered = lower::lower_expr(&expr).unwrap();
    (expr, lowered)
}

/// A type in the two trees.
fn written_type(source: &str) -> (syn3::ast::Type, ilang_syntax::ast::TypeExpr) {
    let (mut module, _) = both(&format!("value: {source} = 1\n"));
    let ty = new::written_type(&mut module, 0).clone();
    let lowered = lower::lower_type(&ty).unwrap();
    (ty, lowered)
}

fn same(new: &syn3::ast::Module, old: &ilang_syntax::ast::Module) {
    let (lowered, errors) = lower::lower(new);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        equivalent(&lowered, old),
        "the trees differ:\n{}\n{}",
        ilang_syntax::print::print_module(&lowered),
        ilang_syntax::print::print_module(old)
    );
}

const SOURCE: &str = "(app/template)\nfirst = (x: String) { 0 }\nsecond = () { log(1) template.helper }\nthird: Int = 1\n";

#[test]
fn a_statement_is_replaced_in_the_same_place() {
    let (expr, lowered) = expression("marker(1, name = 2)");
    for (item, index) in [(0, 0), (1, 0), (1, 1)] {
        let (mut a, mut b) = both(SOURCE);
        new::set_statement(&mut a, item, index, expr.clone());
        old::set_statement(&mut b, item, index, lowered.clone());
        same(&a, &b);
        // The changed tree is not the written one.
        let (_, written) = both(SOURCE);
        assert!(!equivalent(&written, &b));
    }
}

#[test]
fn the_last_expression_is_the_same_statement() {
    let (expr, lowered) = expression("[marker]");
    for item in [0, 1] {
        let (mut a, mut b) = both(SOURCE);
        assert_eq!(
            new::span(new::last_expression(&mut a, item)),
            old::span(old::last_expression(&mut b, item))
        );
        *new::last_expression(&mut a, item) = expr.clone();
        *old::last_expression(&mut b, item) = lowered.clone();
        same(&a, &b);
        assert_eq!(new::statements(&mut a, item).len(), old::statements(&mut b, item).len());
    }
}

#[test]
fn a_value_is_replaced_in_the_same_place() {
    let (mut a, mut b) = both(SOURCE);
    new::value(&mut a, 2).kind = new::string("changed");
    old::value(&mut b, 2).kind = old::string("changed");
    same(&a, &b);
    let (_, written) = both(SOURCE);
    assert!(!equivalent(&written, &b));
}

#[test]
fn a_written_type_is_replaced_in_the_same_place() {
    let (ty, lowered) = written_type("List(Secret)");
    let (mut a, mut b) = both(SOURCE);
    *new::written_type(&mut a, 2) = ty;
    *old::written_type(&mut b, 2) = lowered;
    same(&a, &b);
    let (_, written) = both(SOURCE);
    assert!(!equivalent(&written, &b));
}

#[test]
fn an_import_alias_and_its_use_are_renamed_in_the_same_place() {
    let (mut a, mut b) = both(SOURCE);
    new::set_import_alias(&mut a, 0, "\0compiler");
    old::set_import_alias(&mut b, 0, "\0compiler");
    *new::field_target_name(new::expression(&mut a, 1, 1)) = "\0compiler".into();
    *old::field_target_name(old::expression(&mut b, 1, 1)) = "\0compiler".into();
    same(&a, &b);
    assert_eq!(
        a.imports[0].alias.as_ref().unwrap().span.0,
        b.imports[0].alias.as_ref().unwrap().span
    );
    let (_, written) = both(SOURCE);
    assert!(!equivalent(&written, &b));
}
