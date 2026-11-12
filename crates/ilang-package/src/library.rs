//! The library package `i` (§2.1), which every program has: the library
//! modules (§9, §13) and the prelude of built-in names (§12).

use crate::{ModuleKind, ModuleSource, Package, PackageGraph};

/// The module paths and sources of the library modules, and of the prelude.
pub const MODULES: &[(&str, &str)] = &[
    ("i/fail", include_str!("../../../lib/i/fail.i")),
    ("i/control", include_str!("../../../lib/i/control.i")),
    ("i/resource", include_str!("../../../lib/i/resource.i")),
    ("i/assertion", include_str!("../../../lib/i/assertion.i")),
    ("i/request", include_str!("../../../lib/i/request.i")),
    ("i/key", include_str!("../../../lib/i/key.i")),
    ("i/core", include_str!("../../../lib/i/core.i")),
    ("i/float_text", include_str!("../../../lib/i/float_text.i")),
    ("i/big", include_str!("../../../lib/i/big.i")),
    ("i/bits", include_str!("../../../lib/i/bits.i")),
    ("i/record", include_str!("../../../lib/i/record.i")),
    ("i/io", include_str!("../../../lib/i/io.i")),
    ("i/task", include_str!("../../../lib/i/task.i")),
    ("i/clock", include_str!("../../../lib/i/clock.i")),
    ("i/random", include_str!("../../../lib/i/random.i")),
    ("i/syntax", include_str!("../../../lib/i/syntax.i")),
    ("i/fs", include_str!("../../../lib/i/fs.i")),
    ("i/process", include_str!("../../../lib/i/process.i")),
    ("i/net", include_str!("../../../lib/i/net.i")),
    ("i/tls", include_str!("../../../lib/i/tls.i")),
    ("i/fetch", include_str!("../../../lib/i/fetch.i")),
    ("i/stdio", include_str!("../../../lib/i/stdio.i")),
];

/// The library modules whose functions the runtime calls
/// (`ilang_wasm::ir::RUNTIME_CALLS`). Every program has them, also a
/// compiler program that selects only an import closure.
pub const RUNTIME: &[&str] = &["i/float_text", "i/big", "i/bits"];

pub const PRELUDE: (&str, &str) = ("i/prelude", include_str!("../../../lib/prelude.i"));

/// Adds the package `i` to an empty graph, as package 0.
pub(crate) fn add(graph: &mut PackageGraph) {
    assert!(graph.packages.is_empty());
    let mut modules = Vec::new();
    let all = std::iter::once((PRELUDE, ModuleKind::Prelude))
        .chain(MODULES.iter().map(|&m| (m, ModuleKind::Src)));
    for ((path, text), kind) in all {
        let file = path.strip_prefix("i/").unwrap_or(path);
        let source = graph.sources.add(format!("<i>/{file}.i"), text.to_string());
        modules.push(ModuleSource {
            path: path.to_string(),
            source,
            kind,
        });
    }
    let text = format!(
        "name = \"standard\"\n{}",
        include_str!("../../../lib/provider-plan.toml")
    );
    let (manifest, diagnostics) = crate::manifest::parse_manifest(&text);
    assert!(
        diagnostics.is_empty(),
        "invalid shipped provider plan: {diagnostics:?}"
    );
    let provider_plan = manifest.expect("shipped plan has a name").provider_plan;
    graph.packages.push(Package {
        name: "i".to_string(),
        dir: None,
        manifest_source: None,
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        modules,
        wasm_exports: Vec::new(),
        wasm_synchronous: Vec::new(),
        provider_plan,
        entries: Vec::new(),
        location: None,
    });
}
