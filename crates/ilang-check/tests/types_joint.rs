//! T1/T2 contracts from types-joint J1-J7 and §4. T3-only rules wait for T3.
mod common;
use ilang_check::{Checked, check, code, hir::*, order, query, types};
use ilang_package::{Diagnostic, PackageGraph};

fn checked(files: &[(&str, &str)]) -> (PackageGraph, Checked) {
    let (g, _, _) = common::check_files(files);
    let c = check(&g);
    (g, c)
}
fn good(source: &str) -> (PackageGraph, Checked) {
    let (g, c) = checked(&[("src/main.i", source)]);
    assert!(
        !c.diags.iter().any(Diagnostic::is_error),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
    (g, c)
}
fn bad(source: &str, code: &str) {
    let (g, c) = checked(&[("src/main.i", source)]);
    assert!(
        c.diags.iter().any(|d| d.code == code),
        "{:?}",
        g.sources.render_all(&c.diags)
    );
}
fn alias(c: &Checked, name: &str) -> String {
    let p = c.program.as_ref().unwrap();
    let (i, t) = p
        .types
        .iter()
        .enumerate()
        .find(|(_, t)| t.name == name)
        .unwrap();
    c.types.as_ref().unwrap().alias_types[&TypeId(i as u32)].text(&types::type_names(p, t.module))
}

#[test]
fn j1_value_reads_and_patterns_preserve_exact_literal_sets() {
    // §2.2: no reconstruction defaults an existing Con's set to All.
    let (_, c) = good(
        "v=\"GET\"\nx:v=\"GET\"\nMethod=v\nBox(A){Box(value:A)}\nboxed=Box(8080)\nBox(Port)=boxed\nPair(A,B)=(A,B)\nparts=(\"GET\",8080)\nPair(Verb,Number)=parts\nchoice=True |> {True -> Box(1) False -> Box(2)}\nBox(Choice)=choice\nConfig(A,B){Config(method:A,port:B)}\nconfig=Config(\"GET\",8080)\nConfigType=config\nagain:ConfigType=Config(\"GET\",8080)",
    );
    for (name, want) in [
        ("Method", "\"GET\""),
        ("Port", "8080"),
        ("Verb", "\"GET\""),
        ("Number", "8080"),
        ("Choice", "1 | 2"),
        ("ConfigType", "Config(\"GET\", 8080)"),
    ] {
        assert_eq!(alias(&c, name), want);
    }
    let (_, c) = good("Header(A)=(A,\"GET\")\nv=(8080,\"GET\")\nHeader(Port)=v");
    assert_eq!(alias(&c, "Port"), "8080");
    bad(
        "Header(A)=(A,\"GET\")\nv=(8080,\"POST\")\nHeader(Port)=v",
        code::TYPE_PATTERN_MISMATCH,
    );
    bad("v=\"GET\"\nx:v=\"POST\"", code::TYPE_MISMATCH);
    bad(
        "Box(A){Box(A)}\nv=Box(1)\nBox(Item)=v\nx:Item=2",
        code::TYPE_MISMATCH,
    );
    bad(
        "Box(A){Box(A)}\nv=True |> {True -> Box(1) False -> Box(2)}\nBox(Item)=v\nx:Item=3",
        code::TYPE_MISMATCH,
    );
    let (_, c) = good("v: Int - 0=1\nNonzero=v\nx:Nonzero=2");
    assert_eq!(alias(&c, "Nonzero"), "Int - 0");
    bad("v:Int-0=1\nNonzero=v\nx:Nonzero=0", code::TYPE_MISMATCH);
}

#[test]
fn j1_j2_flow_facts_do_not_become_value_reads_through_access_or_calls() {
    // J2: inferred access parts come from the bound forms, known parts separately.
    good(
        "Box(A){Box(value:A)}\ncopy=(s:String){x=\"a\"=s\np=(Box(x),0).0.value\nq:p=\"b\"\nq}\nanswer=(){copy(\"a\")}",
    );
    good("apply(A,B)=(x:A,f:(A)->B){f(x)}\nf=(){apply(\"a\",(x){y:x=\"b\" y})}");
    bad(
        "Sink(A){Sink(put:(A)->Nil)}\nsink:Sink(\"GET\"|\"POST\")=Sink((x:\"GET\"|\"POST\"){Nil})\nbad=(){sink.put(\"DELETE\")}",
        code::TYPE_MISMATCH,
    );
}

#[test]
fn j1_j5_new_narrowed_binders_have_literal_types_original_subjects_do_not() {
    good("read=(s:String){s |> {other:String-\"a\" -> 0 key -> {same:key=\"a\" 1}}}");
    let source = "read=(s:String){s |> {other:String-\"a\" -> 0 key -> {same:key=\"a\" 1}}}";
    let (g, _) = good(source);
    let c = query::check_for_queries(&g);
    let p = c.program.as_ref().unwrap();
    let q = query::Query::new(&g, p, c.types.as_ref());
    assert!(
        q.type_at(
            common::module(p, "app/main"),
            source.find("key=\"a\"").unwrap() as u32
        )
        .unwrap()
        .code
        .ends_with("\"a\"")
    );
    bad(
        "only=(x:\"a\")->\"a\"{x}\nread=(s:String){s |> {other:String-\"a\" -> 0 key -> {v=only(key) same:v=\"b\" 1}}}",
        code::TYPE_MISMATCH,
    );
    good("read=(s:String){s |> {other:String-\"a\" -> 0 key -> {same:s=\"b\" 1}}}");
}

#[test]
fn j4_literal_fixation_orders_completion_and_rejects_later_widening() {
    let source = "Cell(A){Cell(get:()->A,set:(A)->Nil)}\nmake(A)=(x:A){Cell(get:(){x},set:(y:A){Nil})}\nr=make(1)\nRead=r\nlater=(){r.set(2)}";
    bad(source, code::TYPE_MISMATCH);
    good(&source.replace("r=make(1)", "r:Cell(Int)=make(1)"));
    // The earlier annotation-demand ruling still applies; T1's literal is now 1.
    good("id=(x){x}\nv=id(1)\nx:v=1");
    bad("id=(x){x}\nv=id(1)\nx:v=2", code::TYPE_MISMATCH);
    let source = "Box(A){Box(A)}\nid=(x){x}\nx:Both=(1,2)\nBoth=(First,Second)\nBox(Second)=b\nb=id(Box(2))\nFirst=a\na=id(1)";
    let (g, c) = good(source);
    assert_eq!(alias(&c, "Both"), "(1, 2)");
    let (p, ds) = ilang_check::resolve_program(&g);
    assert!(!ds.iter().any(Diagnostic::is_error));
    let mut p = p.unwrap();
    let (normal, ds) = types::check_keeping_locals_with_errors(&p, &g.sources, true);
    order::reverse_type_ties(&mut p);
    let (reverse, rs) = types::check_keeping_locals_with_errors(&p, &g.sources, true);
    assert!(ds.is_empty() && rs.is_empty());
    assert_eq!(normal.alias_types, reverse.alias_types);
    assert_eq!(normal.schemes, reverse.schemes);
}

#[test]
fn j5_completed_literal_aliases_produce_exact_runtime_tests() {
    let source = "source:\"GET\"|\"POST\"=\"GET\"\nMethods=source\nf=(x:Int|String){x |> {m:Methods -> 1 _ -> 0}}";
    let (_, c) = good(source);
    let p = c.program.as_ref().unwrap();
    let module = common::module(p, "app/main");
    let t = c.types.as_ref().unwrap();
    let tests = t
        .type_tests
        .iter()
        .find(|((m, _), _)| *m == module)
        .unwrap()
        .1;
    assert_eq!(tests.len(), 1);
    assert_eq!(
        tests[0].head,
        types::Head::Named(TypeRef::Builtin(BuiltinType::String))
    );
    let text = format!("{:?}", tests[0].values);
    assert!(
        text.contains("GET") && text.contains("POST") && !text.contains("All"),
        "{text}"
    );
    bad(
        "source:\"GET\"|\"POST\"=\"GET\"\nMethods=source\nf=(x:Methods){x |> {\"GET\" ? True -> 1}}",
        code::CASE_NOT_COVERED,
    );
}

#[test]
fn j6_early_context_provenance_wins_over_literal_boundary_checks() {
    bad(
        "v=1\nA=v\n@external(\"env\",\"f\")\nf:()->A",
        code::VALUE_TYPE_PLACE,
    );
    bad("v=1\nA=v\nT{T(A)}", code::VALUE_TYPE_PLACE);
    bad("v=1\nA=v\nf:()->Nil!A=(){Nil}", code::VALUE_TYPE_PLACE);
    // A scheme that already needs an environment can still be read normally.
    good("Env{Env}\nf=(e!Env){e}\ng:f=(e!Env){e}");
}

#[test]
fn j7_invariant_scheme_reads_keep_fresh_instances_and_weak_sharing() {
    good(
        "Cell(A){Cell(run:(A)->A)}\nmake=(){Cell((x){x})}\nf:make=(){Cell((x:Int){x})}\ng:make=(){Cell((x:String){x})}\na:Cell(Int)=make()\nb:Cell(String)=make()",
    );
    bad(
        "Cell(A){Cell(run:(A)->A)}\nmake=(){Cell((x){x})}\nSame=make",
        code::VALUE_TYPE_OPEN,
    );
    bad(
        "Cell(A){Cell(run:(A)->A)}\nwrong=(c:Cell(Int))->Cell(Never){c}",
        code::TYPE_MISMATCH,
    );
}

#[test]
fn joint_queries_keep_full_literal_aliases_and_nested_head_targets() {
    let source = "Box(A){Box(A)}\nv=Box(\"GET\")\nBox(Method)=v\nx:Method=\"GET\"";
    let (g, c) = good(source);
    let p = c.program.as_ref().unwrap();
    let m = common::module(p, "app/main");
    let q = query::Query::new(&g, p, c.types.as_ref());
    let pos = source.find("Method)=v").unwrap() as u32;
    assert!(q.type_at(m, pos).unwrap().code.contains("\"GET\""));
    let pos = source.find("x:Method").unwrap() as u32 + 2;
    assert_eq!(
        q.definition_at(m, pos).unwrap().span.start as usize,
        source.find("Method)=v").unwrap()
    );
}

#[test]
fn closed_substitution_survives_cached_literal_algebra() {
    // Joint §2.2 and J4: substitute completed reads before exact normalization.
    let source = "id=(x){x}\nn=id(1)\nDifference(A)=A-n\nIntersection(A)=A&n\nf=(x:Difference(1|2)){x}\ng=(x:Intersection(1|2)){x}\na=f(2)\nb=g(1)";
    good(source);
    bad(&source.replace("a=f(2)", "a=f(1)"), code::TYPE_MISMATCH);
    bad(&source.replace("b=g(1)", "b=g(2)"), code::TYPE_MISMATCH);
}
