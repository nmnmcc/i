// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2022 The Gleam contributors

/// The current version of the gleam compiler. If this does not match what is
/// already in the build folder we will not reuse any cached artefacts and
/// instead build from scratch
pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Source revision including the core's shared local crates, manifests and lockfiles.
/// Consumers combine it with their own revision when caching semantic results.
pub const COMPILER_REVISION: &str = env!("ILANG_CORE_REVISION");
