//! Exact record operations (records.md §§20–21).
//!
//! The pure kernel is tested before production type conversion is connected.

mod bdd;
mod bridge;
mod graph;
mod proof;
#[cfg(test)]
mod relations;
mod shown;

pub use shown::ShownGraph;

pub(crate) use bdd::{Demand, ResourceLimit};
pub(crate) use bridge::{Field, Graph, Pool};
#[cfg(test)]
pub(crate) use graph::LabelParam;
pub(crate) use graph::{Id, Label, Node};
pub(crate) use proof::{Verdict, prove};
