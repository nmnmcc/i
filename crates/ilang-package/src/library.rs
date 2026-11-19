//! The built-in package, which every program has: only the prelude of
//! built-in names. The library modules `i/...` are not built in; they come
//! from the package named `i` (`stdlib/`), a dependency of every package
//! that is not itself `i` (see [`crate::code::NO_LIBRARY`]).

use crate::{ModuleKind, ModuleSource, Package, PackageGraph};

/// The name of the built-in package. `$` is not in a name, so no
/// dependency, and no import, can name it.
pub const BUILTIN_NAME: &str = "$builtin";

/// The library modules whose functions the runtime calls
/// (`ilang_wasm::ir::RUNTIME_CALLS`). Every program has them, also a
/// compiler program that selects only an import closure. They are in the
/// package `i`, found by module path.
pub const RUNTIME: &[&str] = &["i/float_text", "i/big", "i/bits"];

/// The module path of the prelude. No import can name it: `$` is not in a
/// name, so the prelude is only ever reached as the built-in names.
pub const PRELUDE_PATH: &str = "$prelude";

pub const PRELUDE: (&str, &str) = (PRELUDE_PATH, include_str!("../../../lib/prelude.i"));

/// Adds the built-in package, with only the prelude, to an empty graph, as
/// package 0 ([`PackageGraph::library`]).
pub(crate) fn add(graph: &mut PackageGraph) {
    assert!(graph.packages.is_empty());
    let (path, text) = PRELUDE;
    let source = graph
        .sources
        .add("<i>/prelude.i".to_string(), text.to_string());
    graph.packages.push(Package {
        name: BUILTIN_NAME.to_string(),
        dir: None,
        manifest_source: None,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        modules: vec![ModuleSource {
            path: path.to_string(),
            source,
            kind: ModuleKind::Prelude,
        }],
        wasm_exports: Vec::new(),
        wasm_synchronous: Vec::new(),
        entries: Vec::new(),
        location: None,
    });
}
