//! Structural normalization for write validation and syntax-tree tests.
use crate::{
    ast::*,
    span::Span,
    walk::{Visitor, Walk},
};

struct Unspan;
impl Visitor for Unspan {
    fn span(&mut self, span: &mut Span) {
        *span = Span::default();
    }
}

/// The debug form of a module with its spans normalized structurally.
/// String literals, names and documentation are never interpreted as spans.
pub fn shape(module: &Module) -> String {
    let mut module = module.clone();
    module.walk(&mut Unspan);
    format!("{module:?}")
}

struct Canon;
impl Visitor for Canon {
    fn expr(&mut self, e: &mut Expr, enter: bool) {
        if !enter
            && let ExprKind::Block(b) = &mut e.kind
            && let [Stmt::Expr(inner)] = &mut b.stmts[..]
        {
            // Children are already normalized. Move the child out rather
            // than cloning it at every level of a nested group chain.
            let child = std::mem::replace(
                inner,
                Expr {
                    kind: ExprKind::Tuple(Vec::new()),
                    span: Span::default(),
                },
            );
            *e = child;
        }
    }
}

/// Remove transparent expression groups and normalize spans. All other
/// data, including every string payload, participates in AST equality.
pub(crate) fn canonical(module: &Module) -> Module {
    let mut module = module.clone();
    module.walk(&mut Canon);
    module.walk(&mut Unspan);
    module
}

struct Floats(Vec<String>);
impl Visitor for Floats {
    fn float(&mut self, value: &mut f64) {
        self.0.push(format!("{value:?}"));
        *value = 0.0;
    }
}

/// Compare syntax after removing transparent groups and source spans.
/// Unlike the debug form, this is safe for deeply nested generated syntax.
pub fn equivalent(a: &Module, b: &Module) -> bool {
    // `canon` historically compares Debug text, which distinguishes signed
    // zero and prints all NaNs alike. Normalize floating fields separately to
    // retain exactly that contract while using stack-safe structural equality.
    let mut a = canonical(a);
    let mut b = canonical(b);
    let mut af = Floats(Vec::new());
    let mut bf = Floats(Vec::new());
    a.walk(&mut af);
    b.walk(&mut bf);
    af.0 == bf.0 && a == b
}

/// Compare expression structure with the same normalization as `equivalent`.
pub fn equivalent_expression(a: &Expr, b: &Expr) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    a.walk(&mut Canon);
    b.walk(&mut Canon);
    a.walk(&mut Unspan);
    b.walk(&mut Unspan);
    let mut af = Floats(Vec::new());
    let mut bf = Floats(Vec::new());
    a.walk(&mut af);
    b.walk(&mut bf);
    af.0 == bf.0 && a == b
}

/// The canonical debug form, for readable assertions in tests.
pub fn canon(module: &Module) -> String {
    format!("{:?}", canonical(module))
}

#[cfg(test)]
mod deep_tests {
    use super::*;
    #[test]
    fn equivalent_matches_debug_canonical_form_on_representative_values() {
        let samples = [
            "value = 1\n",
            "value = { 1 }\n",
            "value = 2\n",
            "value = (1, 2)\n",
            "value = (2, 1)\n",
            "value = (x : Int) { x + 1 }\n",
            "value = (x : String) { x + 1 }\n",
        ];
        let modules: Vec<_> = samples
            .iter()
            .map(|text| {
                let (module, errors) = crate::parse_module(text);
                assert!(errors.is_empty(), "{text}: {errors:?}");
                module
            })
            .collect();
        for a in &modules {
            for b in &modules {
                assert_eq!(equivalent(a, b), canon(a) == canon(b));
                let ItemKind::Binding(left) = &a.items[0].kind else {
                    unreachable!()
                };
                let ItemKind::Binding(right) = &b.items[0].kind else {
                    unreachable!()
                };
                assert_eq!(
                    equivalent_expression(&left.value, &right.value),
                    equivalent(a, b)
                );
            }
        }
        let mut plus = modules[0].clone();
        let mut minus = plus.clone();
        let set = |m: &mut Module, value| {
            let ItemKind::Binding(b) = &mut m.items[0].kind else {
                unreachable!()
            };
            b.value.kind = ExprKind::Float(value);
        };
        set(&mut plus, 0.0);
        set(&mut minus, -0.0);
        assert_ne!(canon(&plus), canon(&minus));
        assert!(!equivalent(&plus, &minus));
        let ItemKind::Binding(p) = &plus.items[0].kind else {
            unreachable!()
        };
        let ItemKind::Binding(m) = &minus.items[0].kind else {
            unreachable!()
        };
        assert!(!equivalent_expression(&p.value, &m.value));
        set(&mut plus, f64::NAN);
        set(&mut minus, f64::NAN);
        assert_eq!(canon(&plus), canon(&minus));
        assert!(equivalent(&plus, &minus));
        let ItemKind::Binding(p) = &plus.items[0].kind else {
            unreachable!()
        };
        let ItemKind::Binding(m) = &minus.items[0].kind else {
            unreachable!()
        };
        assert!(equivalent_expression(&p.value, &m.value));
    }
}
