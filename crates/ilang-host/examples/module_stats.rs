//! Measurement only: how many functions of a module escape (are declared in
//! an element segment, exported, or imported), and how many of them a
//! `ref.func` actually names. Wasmtime compiles an array-to-wasm trampoline
//! for every escaping defined function.
//!
//! `cargo run --release -p ilang-host --example module_stats -- <module.wasm>`

use std::collections::{BTreeMap, BTreeSet};
use wasmparser::{
    CompositeInnerType, ElementItems, ExternalKind, KnownCustom, Name, Operator, Parser, Payload,
};

fn main() {
    let path = std::env::args().nth(1).expect("a module path");
    let bytes = std::fs::read(path).unwrap();
    let mut imported = 0u32;
    let mut defined = 0u32;
    let mut declared = BTreeSet::new();
    let mut exported = BTreeSet::new();
    let mut referenced = BTreeSet::new();
    let mut names = BTreeMap::new();
    let mut bodies = Vec::new();
    let mut types = 0;
    let mut gc_types = 0;
    let mut tables = 0;
    let mut segments = 0;
    for payload in Parser::new(0).parse_all(&bytes) {
        match payload.unwrap() {
            Payload::CustomSection(s) => {
                if let KnownCustom::Name(reader) = s.as_known() {
                    for section in reader {
                        if let Name::Function(map) = section.unwrap() {
                            for name in map {
                                let name = name.unwrap();
                                names.insert(name.index, name.name.to_owned());
                            }
                        }
                    }
                }
            }
            Payload::TypeSection(s) => {
                for group in s {
                    for ty in group.unwrap().types() {
                        types += 1;
                        if matches!(
                            ty.composite_type.inner,
                            CompositeInnerType::Array(_) | CompositeInnerType::Struct(_)
                        ) {
                            gc_types += 1;
                        }
                    }
                }
            }
            Payload::TableSection(s) => tables += s.count(),
            Payload::ImportSection(s) => {
                for import in s.into_imports() {
                    if matches!(import.unwrap().ty, wasmparser::TypeRef::Func(_)) {
                        imported += 1;
                    }
                }
            }
            Payload::FunctionSection(s) => defined = s.count(),
            Payload::ExportSection(s) => {
                for e in s {
                    let e = e.unwrap();
                    if e.kind == ExternalKind::Func {
                        exported.insert(e.index);
                    }
                }
            }
            Payload::ElementSection(s) => {
                segments += s.count();
                for e in s {
                    match e.unwrap().items {
                        ElementItems::Functions(fs) => {
                            for f in fs {
                                declared.insert(f.unwrap());
                            }
                        }
                        ElementItems::Expressions(..) => {}
                    }
                }
            }
            Payload::GlobalSection(s) => {
                for g in s {
                    for op in g.unwrap().init_expr.get_operators_reader() {
                        if let Operator::RefFunc { function_index } = op.unwrap() {
                            referenced.insert(function_index);
                        }
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let locals: u32 = body
                    .get_locals_reader()
                    .unwrap()
                    .into_iter()
                    .map(|l| l.unwrap().0)
                    .sum();
                bodies.push((
                    (body.range().end - body.range().start) as usize,
                    imported + bodies.len() as u32,
                    locals,
                ));
                for op in body.get_operators_reader().unwrap() {
                    if let Operator::RefFunc { function_index } = op.unwrap() {
                        referenced.insert(function_index);
                    }
                }
            }
            _ => {}
        }
    }
    let escaping: BTreeSet<u32> = declared.union(&exported).copied().collect();
    let needed: BTreeSet<u32> = referenced.union(&exported).copied().collect();
    let defined_only = |s: &BTreeSet<u32>| s.iter().filter(|f| **f >= imported).count();
    println!(
        "types: {types}, GC array/struct types: {gc_types}, tables: {tables}, element segments: {segments}"
    );
    println!("code bytes: {}", bodies.iter().map(|b| b.0).sum::<usize>());
    bodies.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (size, index, locals) in bodies.iter().take(30) {
        println!(
            "{size:8} bytes {locals:6} locals f{index}: {}",
            names.get(index).map(String::as_str).unwrap_or("<unnamed>")
        );
    }
    println!("functions: {imported} imported, {defined} defined");
    println!("declared in element segments: {}", declared.len());
    println!("exported: {}", exported.len());
    println!("named by ref.func: {}", referenced.len());
    println!(
        "escaping defined functions (trampolines now): {}",
        defined_only(&escaping)
    );
    println!(
        "escaping if only ref.func targets were declared: {}",
        defined_only(&needed)
    );
}
