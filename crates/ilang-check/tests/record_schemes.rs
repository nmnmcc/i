mod common;

use ilang_check::{
    hir::{BuiltinType, DefId, TypeRef},
    types::SchemeArtifact,
};

#[test]
fn exported_label_scheme_round_trips_with_cross_module_caller() {
    let (graph, program, diags) = common::check_files(&[
        (
            "src/defs.i",
            "(i/record)\n(get)\nget(K) = (r: (K: Int), key: K) -> Int { record.get(r, key) }\n",
        ),
        (
            "src/main.i",
            "(app/defs)\n(main)\nmain = () { assert(defs.get((a: 1), \"a\") == 1) }\n",
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
