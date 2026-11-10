//! Helpers shared by the tests.

#![allow(dead_code)]

use ilang_syntax::ast::Module;

/// The debug form of a module without spans, to compare trees.
pub fn shape(module: &Module) -> String {
    ilang_syntax::shape::shape(module)
}
