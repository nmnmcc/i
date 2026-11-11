//! Surface-independent record code generation, including both engine limits.
//! Expected behavior is records.md §§9.1–9.4, not generated golden output.

use ilang_wasm::ir::*;
use std::{env, process::Command};

fn function(name: &str, arity: u32, locals: usize, body: Vec<Stmt>) -> FuncKind {
    FuncKind::Code(Func {
        name: name.into(),
        arity,
        locals: vec![LocalType::Value; locals],
        body,
        frame: false,
    })
}

fn let_value(dst: u32, value: Value) -> Stmt {
    Stmt::Let {
        dst,
        value,
        site: None,
    }
}

#[test]
fn record_globals_and_large_arrays_validate_and_run_at_o0_and_o2() {
    for labels in [
        vec![
            "é".into(),
            "a b".into(),
            "_x".into(),
            "".into(),
            "quote\"".into(),
        ],
        vec!["a".repeat(10_001)],
        (0..5_001).map(|i| format!("f{i:04}")).collect(),
    ] {
        let count = labels.len();
        let fields = (0..count)
            .rev()
            .map(|i| (LabelConst(i as u32), Atom::Int((i as i64).into())))
            .collect();
        let mut p = Program {
            labels,
            ..Program::default()
        };
        p.funcs = vec![
            function(
                "new",
                0,
                1,
                vec![
                    let_value(0, Value::RecNew(fields)),
                    Stmt::Return(Atom::Local(0)),
                ],
            ),
            function(
                "get",
                1,
                2,
                vec![
                    let_value(1, Value::RecGet(Atom::Local(0), LabelConst(0))),
                    Stmt::Return(Atom::Local(1)),
                ],
            ),
            function(
                "with",
                1,
                2,
                vec![
                    let_value(
                        1,
                        Value::RecWith(Atom::Local(0), vec![(LabelConst(0), Atom::Int(99.into()))]),
                    ),
                    Stmt::Return(Atom::Local(1)),
                ],
            ),
            function(
                "is record",
                1,
                1,
                vec![Stmt::If {
                    cond: Cond::IsRec(Atom::Local(0)),
                    then: vec![Stmt::Return(Atom::Bool(true))],
                    otherwise: vec![Stmt::Return(Atom::Bool(false))],
                }],
            ),
            function(
                "label",
                0,
                0,
                vec![Stmt::Return(Atom::Label(LabelConst(0)))],
            ),
        ];
        ilang_wasm::control::split(&mut p);
        let source = ilang_wasm::emit::wat(&p).replace(ilang_wasm::runtime::HOST,
            "(func $host_panic (param i32 i32) unreachable)\n(func $host_reference_id (param externref) (result i64) (i64.const 1))");
        let mut source = source.strip_suffix(")\n").expect("module end").to_string();
        source.push_str(&format!(r#"
          (func $record_test_assert (param $v i32) (if (i32.eqz (local.get $v)) (then unreachable)))
          (func (export "record_test") (result i32)
            (local $a eqref) (local $b eqref) (local $built eqref) (local $pairs (ref $values)) (local $i i32)
            (call $ilang_start)
            (local.set $a (call $f0 (ref.null $obj) (ref.null $env)))
            (call $record_test_assert (i64.eq (call $int_to_i64 (call $record_size (local.get $a))) (i64.const {count})))
            (call $record_test_assert (call $value_eq (call $f1 (ref.null $obj) (ref.null $env) (local.get $a)) (ref.i31 (i32.const 0))))
            (local.set $b (call $f2 (ref.null $obj) (ref.null $env) (local.get $a)))
            (call $record_test_assert (call $value_eq (call $f1 (ref.null $obj) (ref.null $env) (local.get $b)) (ref.i31 (i32.const 99))))
            (call $record_test_assert (call $value_eq (call $f1 (ref.null $obj) (ref.null $env) (local.get $a)) (ref.i31 (i32.const 0))))
            (call $record_test_assert (call $bool_get (call $f3 (ref.null $obj) (ref.null $env) (local.get $a))))
            (call $record_test_assert (i32.eqz (call $bool_get (call $f3 (ref.null $obj) (ref.null $env) (call $tuple (array.new_fixed $values 0))))))
            (local.set $pairs (call $record_fields (local.get $a)))
            (call $record_test_assert (ref.eq
              (array.get $values (local.get $pairs) (call $record_index (local.get $pairs) (call $f4 (ref.null $obj) (ref.null $env))))
              (call $f4 (ref.null $obj) (ref.null $env))))
            (local.set $built (call $record_new (array.new_fixed $values 0)))
            (block $done (loop $each
              (br_if $done (i32.ge_u (local.get $i) (array.len (local.get $pairs))))
              (local.set $built (call $record_put (local.get $built)
                (array.get $values (local.get $pairs) (local.get $i))
                (array.get $values (local.get $pairs) (i32.add (local.get $i) (i32.const 1)))))
              (local.set $i (i32.add (local.get $i) (i32.const 2))) (br $each)))
            (call $record_test_assert (call $value_eq (local.get $a) (local.get $built)))
            (call $record_test_assert (i32.eq (call $value_hash_u32 (local.get $a)) (call $value_hash_u32 (local.get $built))))
            (call $record_test_assert (call $value_eq (call $inspect (local.get $a)) (call $inspect (local.get $built))))
            (i32.const 1)))
        "#));
        let dir = tempfile::tempdir().unwrap();
        let wat = dir.path().join("record.wat");
        let raw = dir.path().join("record.wasm");
        std::fs::write(&wat, source).unwrap();
        let output = Command::new(env::var_os("WASM_TOOLS").unwrap_or_else(|| "wasm-tools".into()))
            .arg("parse")
            .arg(&wat)
            .arg("-o")
            .arg(&raw)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw = std::fs::read(raw).unwrap();
        for level in [ilang_binaryen::Level::O0, ilang_binaryen::Level::O2] {
            let binary = ilang_binaryen::optimize(&raw, level).unwrap();
            ilang_wasm::validate(&binary).unwrap();
            let file = dir.path().join("optimized.wasm");
            std::fs::write(&file, binary).unwrap();
            let output = Command::new(env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into()))
                .args(["run", "--invoke", "record_test"])
                .arg(file)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "count {count}, {level:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "1");
        }
    }
}
