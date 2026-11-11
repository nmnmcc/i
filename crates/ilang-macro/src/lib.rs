//! Compile-time syntax macro programs and expansion.
pub mod body_walk;
pub mod prepare;
pub mod run;
pub mod session;

pub mod body_engine;
pub mod call;
pub mod module_order;
pub mod registry;
pub mod validate;
pub mod diagnostics;
pub mod item_phase;
pub mod definition_world;
