//! Native tree comparison for surface properties (§14.1). No lowering to
//! the frozen tree: that tree cannot represent keyed data patterns (§7.1).
extern crate ilang_syntax as native;
use native::{ast, surface, walk};

#[derive(Default)]
struct Floats(Vec<u64>);
impl walk::Visitor for Floats {
    fn expr(&mut self, e: &mut ast::Expr, enter: bool) {
        if enter && let ast::ExprKind::Float(value) = &e.kind {
            self.0.push(value.to_bits());
        }
    }
    fn pattern(&mut self, p: &mut ast::Pattern, enter: bool) {
        if enter && let ast::PatternKind::Float { value, .. } = &p.kind {
            self.0.push(value.to_bits());
        }
    }
}

/// Compare every native field, ignoring only source positions and the
/// transparent groups/interpolation of the surface conversion contract.
/// Float bits retain the old canonical comparison's signed-zero check.
pub fn equivalent(a: &ast::Module, b: &ast::Module) -> bool {
    native::deep::deep(|| {
        let mut a = surface::ungrouped(a);
        let mut b = surface::ungrouped(b);
        let mut af = Floats::default();
        let mut bf = Floats::default();
        walk::module(&mut a, &mut af);
        walk::module(&mut b, &mut bf);
        af.0 == bf.0 && a == b
    })
}
