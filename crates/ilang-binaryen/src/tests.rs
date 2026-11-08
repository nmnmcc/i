use super::*;

/// A module that uses every Wasm feature that i programs need: GC structs,
/// typed function references, a deep chain of tail calls, and exception
/// handling with `try_table`.
const SMOKE: &str = r#"
(module
  (type $box (struct (field i64)))
  (type $count (func (param i64 i64) (result i64)))
  (tag $error (param eqref))

  ;; Counts down from n with tail calls; without real tail calls a large n
  ;; overflows the stack.
  (func $loop (type $count) (param $n i64) (param $acc i64) (result i64)
    (if (result i64) (i64.eqz (local.get $n))
      (then (local.get $acc))
      (else
        (return_call_ref $count
          (i64.sub (local.get $n) (i64.const 1))
          (i64.add (local.get $acc) (i64.const 1))
          (ref.func $loop)))))
  (elem declare func $loop)

  (func (export "count") (param $n i64) (result i64)
    (call $loop (local.get $n) (i64.const 0)))

  ;; Throws a boxed number and catches it again.
  (func (export "catch") (param $n i64) (result i64)
    (block $caught (result eqref)
      (try_table (catch $error $caught)
        (throw $error (struct.new $box (local.get $n))))
      (unreachable))
    (struct.get $box 0 (ref.cast (ref $box)))))
"#;

fn with_metadata(mut wasm: Vec<u8>) -> Vec<u8> {
    let name = b"ilang.abi";
    let data = br#"{"version":2}"#;
    let mut content = vec![name.len() as u8];
    content.extend_from_slice(name);
    content.extend_from_slice(data);
    wasm.push(0);
    wasm.push(content.len() as u8);
    wasm.extend_from_slice(&content);
    wasm
}

fn validate(wasm: &[u8]) {
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(wasm)
        .expect("the module must be valid");
}

fn custom_sections(wasm: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut sections = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(wasm) {
        if let wasmparser::Payload::CustomSection(section) = payload.expect("parse") {
            sections.push((section.name().to_owned(), section.data().to_vec()));
        }
    }
    sections
}

fn run(wasm: &[u8]) -> (i64, i64) {
    let mut config = wasmtime::Config::new();
    config
        .wasm_gc(true)
        .wasm_function_references(true)
        .wasm_tail_call(true)
        .wasm_exceptions(true);
    let engine = wasmtime::Engine::new(&config).expect("engine");
    let module = wasmtime::Module::new(&engine, wasm).expect("compile");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).expect("instantiate");
    let count = instance
        .get_typed_func::<i64, i64>(&mut store, "count")
        .expect("count");
    let catch = instance
        .get_typed_func::<i64, i64>(&mut store, "catch")
        .expect("catch");
    (
        count.call(&mut store, 1_000_000).expect("count runs"),
        catch.call(&mut store, 42).expect("catch runs"),
    )
}

#[test]
fn level_zero_gives_the_module_back() {
    let wasm = with_metadata(wat::parse_str(SMOKE).expect("wat"));
    assert_eq!(optimize(&wasm, Level::O0).expect("O0"), wasm);
}

#[test]
fn optimized_module_is_valid_keeps_metadata_and_behaves_the_same() {
    let wasm = with_metadata(wat::parse_str(SMOKE).expect("wat"));
    validate(&wasm);
    let expected = run(&wasm);
    assert_eq!(expected, (1_000_000, 42));

    for level in [Level::O2, Level::O3] {
        let optimized = optimize(&wasm, level).expect("optimize");
        validate(&optimized);
        assert_eq!(run(&optimized), expected, "{level:?} changed behavior");
        let metadata: Vec<_> = custom_sections(&optimized)
            .into_iter()
            .filter(|(name, _)| name.starts_with("ilang."))
            .collect();
        assert_eq!(
            metadata,
            vec![("ilang.abi".to_owned(), br#"{"version":2}"#.to_vec())]
        );
    }
}

#[test]
fn malformed_input_is_an_error() {
    assert!(matches!(
        optimize(b"not wasm", Level::O2),
        Err(Error::Malformed(_))
    ));
    let mut cut = wat::parse_str(SMOKE).expect("wat");
    cut.truncate(cut.len() - 3);
    assert!(optimize(&cut, Level::O2).is_err());
}
