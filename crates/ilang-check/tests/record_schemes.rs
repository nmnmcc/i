mod common;

use ilang_check::{
    hir::{BuiltinType, DefId, TypeRef},
    types::{SchemeArtifact, SchemeEnvironment},
};

#[test]
fn exported_label_scheme_round_trips_with_cross_module_caller() {
    if common::syn3_pending(
        "record_schemes::exported_label_scheme_round_trips_with_cross_module_caller",
    ) {
        return;
    }
    let (graph, program, diags) = common::check_files(&[
        (
            "src/defs.i",
            "(\n\ti/record\n)\n\n(\n\tget\n)\nget = <K>(r: (k = Int), key: K) -> Int { record.get(r, key) }\n",
        ),
        (
            "src/main.i",
            "(\n\tapp/defs\n)\n\n(\n\tmain\n)\nmain = () { assert(defs.get((a = 1), \"a\") == 1) }\n",
        ),
    ]);
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let program = program.unwrap();
    let (typed, diags) = ilang_check::types::check(&program, &graph.sources);
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let module = common::module(&program, "app/defs");
    let index = program
        .module(module)
        .defs
        .iter()
        .position(|d| d.name == "get")
        .unwrap();
    let scheme = &typed.scheme_artifacts[&DefId {
        module,
        index: index as u32,
    }];
    let name = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().to_string(),
        TypeRef::Custom(id) => format!("custom:{}", id.0),
    };
    let bundle = scheme.text_with(&mut name.clone());
    let copy = SchemeArtifact::from_bundle(&bundle, &mut |name| match name {
        "Int" => Some(TypeRef::Builtin(BuiltinType::Int)),
        "String" => Some(TypeRef::Builtin(BuiltinType::String)),
        _ => None,
    })
    .unwrap();
    let mut before: serde_json::Value = serde_json::from_str(&bundle).unwrap();
    let mut after: serde_json::Value =
        serde_json::from_str(&copy.text_with(&mut name.clone())).unwrap();
    before.as_object_mut().unwrap().remove("graph");
    after.as_object_mut().unwrap().remove("graph");
    assert_eq!(before, after);
}

#[test]
fn exports_from_one_nondirect_binding_retain_shared_type_identity() {
    let (graph, program, diags) = common::check_files(&[(
        "src/defs.i",
        "(a, b)\nmake = (f) { (f, f) }\n(a, b) = make((x) { x })\n",
    )]);
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let program = program.unwrap();
    let (typed, diags) = ilang_check::types::check(&program, &graph.sources);
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let module = common::module(&program, "app/defs");
    let artifact = |name: &str| {
        let index = program
            .module(module)
            .defs
            .iter()
            .position(|d| d.name == name)
            .unwrap();
        &typed.scheme_artifacts[&DefId {
            module,
            index: index as u32,
        }]
    };
    let mut name = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().to_string(),
        TypeRef::Custom(id) => format!("custom:{}", id.0),
    };
    let a = artifact("a").text_with(&mut name);
    let b = artifact("b").text_with(&mut name);
    let ids = |text: &str| -> std::collections::BTreeSet<u64> {
        let json: serde_json::Value = serde_json::from_str(text).unwrap();
        json["variables"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v["external"].as_u64())
            .collect()
    };
    assert!(!ids(&a).is_empty());
    assert_eq!(ids(&a), ids(&b));
    let mut env = SchemeEnvironment::default();
    env.read(&a, &mut |_| None).unwrap();
    env.read(&b, &mut |_| None).unwrap();
}
