//! Helpers shared by the tests.

#![allow(dead_code)]

use migration_syntax::ast::Module;

/// The debug form of a module without spans, to compare trees.
pub fn shape(module: &Module) -> String {
    migration_syntax::shape::shape(module)
}
