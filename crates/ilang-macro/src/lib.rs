//! Compile-time syntax macro programs and expansion.
pub mod body_walk;
pub mod body_walk3;
pub mod prepare;
pub mod prepare3;
pub mod run;
pub mod session;

pub mod body_engine;
pub mod body_engine3;
pub mod call;
pub mod call3;
pub mod deep3;
pub mod collections;
pub mod definition_world;
pub mod definition_world3;
pub mod diagnostics;
pub mod item_phase;
pub mod item_phase3;
pub mod module_order;
pub mod project;
pub mod registry;
pub mod registry3;
pub mod validate;

pub mod typed;
pub mod typed3;
