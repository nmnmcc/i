use ilang_package::{Diagnostic, LoadOptions};
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("package path");
    let mut options = LoadOptions::from_env();
    options.tests = true;
    let (graph, diags) = ilang_package::load_with(Path::new(&path), &options);
    let checked = ilang_check::check(&graph);
    if diags.iter().chain(&checked.diags).any(Diagnostic::is_error) {
        println!("{{}}");
        return;
    }
    let program = checked.program.expect("checked program");
    let types = checked.types.expect("checked types");
    let mut records = std::collections::BTreeMap::<String, Vec<u32>>::new();
    for (module, span) in types.empty_records {
        let source = graph.sources.get(program.module(module).source);
        records
            .entry(source.display.clone())
            .or_default()
            .push(span.start);
    }
    println!("{}", serde_json::to_string(&records).unwrap());
}
