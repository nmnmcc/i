//! Lazy re-entry of paused frames (docs/design/lazy-reentry.md, 13.2): the
//! shape of the code that resume, the frame records and the host seam
//! make, and runs of programs whose output the spec does not state. The
//! rules that the spec states are tested by the spec suite in `tests/spec/`.
//!
//! The run tests compile at `--opt 0` for the WASI target (B.7) and run the
//! module with the `wasmtime` command (`WASMTIME`, by default `wasmtime`),
//! which reads the stop text from the standard error. The same programs at
//! `--opt 2` are in `crates/ilang-cli/tests/wasi.rs`.

use ilang_wasm::{Options, Target, compile, compile_wat};
use std::{env, process::Command};

fn checked(
    source: &str,
) -> (
    ilang_package::PackageGraph,
    ilang_check::Checked,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    (graph, checked, dir)
}

fn wat(source: &str) -> String {
    let (graph, c, _dir) = checked(source);
    compile_wat(
        &graph,
        c.program.as_ref().unwrap(),
        c.types.as_ref().unwrap(),
        Options::default(),
    )
    .unwrap()
}

/// A program that uses every control operation, `fail.catch`,
/// `resource.ensure` and environment statements.
const PROGRAM: &str = r#"(
	i/control
	i/core
	i/fail
	i/resource
)

Name = (text = String)
Tag = (text = String)

name = () <- (n: Name) { n.text }
tag = () <- (t: Tag) { t.text }

plain = (b) {
	x = control.suspend(b, (k) { control.resume(k, "v") })
	x <> name() <> tag()
}

one = (b) {
	{
		y = plain(b)
		y <> "."
	} <- (Name(text = "one"))
}

two = (b) {
	{
		y = plain(b)
		y <> "."
	} <- (Name(text = "two"), Tag(text = "two"))
}

guarded = (b) {
	r = fail.catch(() {
		resource.ensure(
			() {
				x = control.suspend(b, (k) {
					control.cancel(k)
					"c"
				})
				x <> "!"
			},
			() { Nil },
		)
	})
	core.inspect(r)
}

nested = () {
	control.boundary((outer) {
		control.boundary((inner) {
			x = control.suspend(outer, (k) { control.resume(k, 1) + 100 })
			x + 1
		})
	})
}

main = () {
	{
		a = control.boundary((b) { one(b) })
		c = control.boundary((b) { two(b) })
		d = control.boundary((b) { guarded(b) })
		e = nested()
		core.inspect((a, c, d, e))
		Nil
	} <- (Name(text = "main"), Tag(text = "main"))
}
"#;

/// The S-expression that starts at `at`, to its matching `)`.
fn sexp(text: &str, at: usize) -> &str {
    let mut depth = 0;
    for (i, ch) in text[at..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &text[at..at + i + 1];
                }
            }
            _ => {}
        }
    }
    panic!("no matching bracket")
}

/// The items of an S-expression, at its top level.
fn items(s: &str) -> Vec<String> {
    let inner = &s[1..s.len() - 1];
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = inner.as_bytes();
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\n' | b'\t' => i += 1,
            b'(' => {
                let e = sexp(inner, i);
                out.push(e.split_whitespace().collect::<Vec<_>>().join(" "));
                i += e.len();
            }
            _ => {
                let end = inner[i..]
                    .find(|c: char| c.is_whitespace() || c == '(')
                    .map_or(inner.len(), |e| i + e);
                out.push(inner[i..end].to_string());
                i = end;
            }
        }
    }
    out
}

/// The text of the function `name` (with its `$`).
fn func<'a>(wat: &'a str, name: &str) -> &'a str {
    let start = wat
        .find(&format!("(func {name} "))
        .unwrap_or_else(|| panic!("no function {name}"));
    sexp(wat, start)
}

/// Every S-expression that starts with `head`.
fn all<'a>(wat: &'a str, head: &str) -> Vec<&'a str> {
    wat.match_indices(head).map(|(i, _)| sexp(wat, i)).collect()
}

#[test]
fn reentry_rebuilds_one_frame() {
    let text = wat(PROGRAM);
    assert!(!text.contains("$reenter_next"), "$reenter_next is gone");
    let mut seen = 0;
    for (i, _) in text.match_indices("(func $r") {
        let f = sexp(&text, i);
        let name = f.split_whitespace().nth(1).unwrap();
        if !name[2..].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        seen += 1;
        assert!(!f.contains("try_table"), "{f}");
        assert!(!f.contains("call_ref"), "{f}");
        let last = items(f).pop().unwrap();
        assert!(last.starts_with("(return_call $s"), "{f}");
    }
    assert!(seen >= 3, "the program has points");
    assert!(func(&text, "$control_resume").contains("return_call $boundary_core"));
    assert!(func(&text, "$boundary_core").contains("call $tramp_run"));
    // Sections 5.5 and 5.6: a re-suspend captures the frames entered since
    // resume, then splices the records still pending in constant time.
    let tramp = func(&text, "$tramp_run");
    let splice = all(tramp, "(call $capture_splice ");
    assert_eq!(splice.len(), 1);
    assert_eq!(
        items(splice[0]),
        [
            "call",
            "$capture_splice",
            "(local.get $c)",
            "(local.get $node)",
            "(local.get $nm)",
            "(local.get $last)",
            "(local.get $mlast)",
        ]
    );
    assert!(!tramp.contains("struct.new $frame"));
    assert!(!tramp.contains("call $capture_rethrow"));
    let splice = func(&text, "$capture_splice");
    assert!(!splice.contains("loop"), "{splice}");
    assert!(!splice.contains("(call"), "{splice}");
    assert!(!splice.contains("struct.new"), "{splice}");
}

#[test]
fn record_kinds() {
    let text = wat(PROGRAM);
    let records: Vec<Vec<String>> = all(&text, "(struct.new $frame (ref.func $r")
        .into_iter()
        .map(items)
        .collect();
    assert!(!records.is_empty());
    let mut kinds = [0; 2];
    let mut pairs = Vec::new();
    for r in &records {
        // struct.new, $frame, re-entry, values, slot, kind, delta
        assert_eq!(r.len(), 7, "{r:?}");
        assert_eq!(r[4], "(local.get $slot)", "{r:?}");
        match r[5].as_str() {
            "(i32.const 0)" => {
                kinds[0] += 1;
                assert_eq!(r[6], "(ref.null $values)", "{r:?}");
            }
            "(i32.const 1)" => {
                kinds[1] += 1;
                let delta = items(&r[6]);
                assert_eq!(delta[0], "array.new_fixed");
                assert_eq!(delta[1], "$values");
                let n: usize = delta[2].parse().unwrap();
                assert_eq!(n, delta.len() - 3, "{r:?}");
                for pair in delta[3..].chunks(2) {
                    let service = pair[0]
                        .strip_prefix("(ref.i31 (i32.const ")
                        .and_then(|s| s.strip_suffix("))"))
                        .unwrap_or_else(|| panic!("{r:?}"));
                    assert!(
                        pair[1].starts_with("(array.get $env (local.get $l"),
                        "{r:?}"
                    );
                    assert!(
                        pair[1].ends_with(&format!("(i32.const {service}))")),
                        "{r:?}"
                    );
                }
                pairs.push(delta[3..].to_vec());
            }
            k => panic!("kind {k} in {r:?}"),
        }
    }
    assert!(kinds[0] > 0 && kinds[1] > 0, "{kinds:?}");
    // `one` gives one pair; `two` gives two, outermost first: the second
    // is made from the first, so its local comes later.
    assert!(pairs.iter().any(|p| p.len() == 2), "{pairs:?}");
    let two = pairs.iter().find(|p| p.len() == 4).expect("two pairs");
    let local = |s: &str| -> u32 {
        let rest = s.split("(local.get $l").nth(1).unwrap();
        rest[..rest.find(')').unwrap()].parse().unwrap()
    };
    assert!(local(&two[1]) < local(&two[3]), "{two:?}");
    assert_ne!(two[0], two[2], "two services: {two:?}");
    // The runtime records: no slot (-1), and their kinds.
    for (reenter, kind) in [
        ("$catch_reenter", 4),
        ("$ensure_reenter", 2),
        // Bit 5 marks finalizer re-entry for cancellation shielding; Wasm's
        // i32 shift uses the low five bits, so it dispatches like kind 5.
        ("$final_reenter", 37),
        ("$wait_requests_reenter", 5),
    ] {
        let made = all(&text, &format!("(struct.new $frame (ref.func {reenter})"));
        assert_eq!(made.len(), 1, "{reenter}");
        let r = items(made[0]);
        assert_eq!(r[4], "(i32.const -1)", "{r:?}");
        assert_eq!(r[5], format!("(i32.const {kind})"), "{r:?}");
    }
    for name in ["$sub_has_final", "$stamp_finalizing"] {
        assert!(func(&text, name).contains("(i32.const 37)"));
    }
    let pack = func(&text, "$capture_pack");
    assert!(pack.contains("(i32.const -1) (i32.const 3)"), "{pack}");
    assert!(func(&text, "$boundary_core").contains("(ref.func $boundary_reenter)"));
    assert!(func(&text, "$cancel_core").contains("(ref.func $cancel_reenter)"));
    let tramp = func(&text, "$tramp_run");
    assert!(tramp.contains("(i32.const 0x1C)") && tramp.contains("(i32.const 0x0C)"));
}

#[test]
fn host_seam_kept() {
    let text = wat(PROGRAM);
    let calls: Vec<Vec<String>> = text
        .match_indices("call $boundary_run ")
        .map(|(i, _)| items(sexp(&text, text[..i].rfind('(').unwrap())))
        .collect();
    assert!(!calls.is_empty());
    for c in &calls {
        // call, name, b, env, body, rest, v, cancel, frame
        assert_eq!(c.len(), 9, "{c:?}");
        assert_eq!(c[5], "(ref.null $flist)", "{c:?}");
        assert_eq!(c[7], "(ref.null eq)", "{c:?}");
        assert_eq!(c[8], "(ref.null $frame)", "{c:?}");
    }
    let resume = items(func(&text, "$host_resume_k"));
    assert!(
        resume.contains(&"(local.set $k (global.get $host_k))".to_string()),
        "{resume:?}"
    );
    assert_eq!(
        resume.last().unwrap(),
        "(return_call $control_resume (local.get $k) (ref.null eq) (global.get $root_env))"
    );
    let shim = func(&text, "$boundary_run");
    for p in ["$rest", "$cancel", "$frame"] {
        assert!(
            shim.contains(&format!(
                "(if (i32.eqz (ref.is_null (local.get {p}))) (then unreachable))"
            )),
            "{shim}"
        );
    }
    assert_eq!(shim.matches("return_call $boundary_core").count(), 1);
    assert_eq!(shim.matches("call").count(), 1, "{shim}");
}

#[test]
fn runtime_alone_validates() {
    let source = format!(
        "(module {} {} {} {})",
        ilang_wasm::runtime::TYPES,
        ilang_wasm::runtime::HOST,
        ilang_wasm::runtime::wat(),
        ilang_wasm::runtime::LIBRARY_ABSENT
    );
    let module = wat::parse_str(&source).unwrap();
    ilang_wasm::validate(&module).unwrap();
}

#[test]
fn modules_validate() {
    let (graph, c, _dir) = checked(PROGRAM);
    let wasm = compile(
        &graph,
        c.program.as_ref().unwrap(),
        c.types.as_ref().unwrap(),
        Options::default(),
    )
    .unwrap();
    ilang_wasm::validate(&wasm).unwrap();
}

// Run tests 12 to 16 of docs/design/lazy-reentry.md (13.2), and the error
// through a final record (13.3), with what eager at `1df2fe0` gives for
// them (D15, D16).

/// The start of the programs of run tests 12, 13 and 15. A cancel runs a
/// final function that suspends to `b2`, and `b0` takes the continuation
/// k2 of that suspend out of the cancel. Every computation that `put` runs
/// in needs `Out`, so `main` gives a placeholder `Out` first.
const CANCEL_OUT: &str = r#"(
	i/control
	i/fail
	i/resource
)

// Hands k2 out of the cancel, to the boundary `b0` around it.
Out = (put = (Inner) -> Nil)
Inner = (Continuation(Nil, Int, Never, Out))
Paused = (Continuation(Nil, Outer, Never, Out))
Done = ()
Outer = Paused | Done
Handed = (Inner)
Lost = ()
Handed = Handed | Lost

put = (k2) <- (o: Out) { o.put(k2) }
"#;

/// `g` of test 12: its frame waits for the ensure, so its record is pending.
const G_PLAIN: &str = r#"// A code frame with a site: it waits for the ensure, so its record is pending.
g = (b, b2) {
  x = resource.ensure(
    () {
      control.suspend(b, (k) { Paused(k) })
      0
    },
    () {
      control.suspend(b2, (k2) {
        put(Inner(k2))
        0
      })
    },
  )
  x + 1
}

"#;

/// `g` of the second part of test 12: a `fail.catch` that the cancel passes
/// over lies between the final record and the record of `g`.
const G_CATCH: &str = r#"// A code frame with a site, and a `fail.catch` that the cancel passes over.
g = (b, b2) {
	x = fail.catch(() {
		resource.ensure(
			() {
				control.suspend(b, (k) { Paused(k) })
				0
			},
			() {
				control.suspend(b2, (k2) {
					put(Inner(k2))
					0
				})
			},
		)
	})
	x |> (Ok(n)) { n + 1 }
}
"#;

/// Gives k2.
const PAUSED: &str = r#"// The cancel of `k` runs the final function, which suspends to `b2`; `b0`
// takes the continuation k2 of that suspend out of the cancel.
paused = () {
	k = {
		control.boundary((b) {
			control.boundary((b2) { g(b, b2) })
			Done
		})
	} |>
		(Paused(k)) { k }
		| (Done) { panic("not paused") }
	handed = control.boundary((b0) {
		{
			control.cancel(k)
			Lost
		} <- (Out(put = (k2) { control.suspend(b0, (rest) { Handed(k2) }) }))
	})
	handed |>
		(Handed(Inner(k2))) { k2 }
		| (Lost) { panic("k2 was not handed out") }
}
"#;

/// Test 12: the resume of k2 runs the end of the final function, which
/// throws the `$cancel` of `k`. Nothing takes it, and the entry stops.
const MAIN_OUT: &str = r#"// The resumed final function ends, and the `$cancel` of `k` goes out of
// the resume, which nothing takes.
main = () {
	// The computations need `Out`. This one is never called.
	{
		control.resume(paused(), Nil)
		Nil
	} <- (Out(put = (k2) { Nil }))
}
"#;

/// Test 13: k2 is resumed from frames that a resume of k3 entered.
const THIRD: &str = r#"Third = (Continuation(Nil, Third, Never, Out))
Over = ()
// `h` resumes k2, which needs `Out`, so k3 needs it too.
Third = Third | Over

keep = (t) { t |> (Over) { Over } | (t) { t } }

// Not in tail position, so the frame waits.
inner = (k2) { control.resume(k2, Nil) + 1 }

h = (b3, k2) {
	control.suspend(b3, (k3) { Third(k3) })
	inner(k2)
	Over
}
"#;

/// Test 13: the outer frame of k3 waits.
const O1_PLAIN: &str = r#"// The outer frame of k3, which waits.
o1 = (b3, k2) { keep(h(b3, k2)) }

"#;

/// Test 15: the ensure of k3 waits outside `h`.
const O1_ENSURE: &str = r#"// The ensure of k3 waits outside `h`: the cancel runs its final function.
o1 = (b3, k2) { keep(resource.ensure(() { h(b3, k2) }, () { panic("final") })) }

"#;

/// The entry of tests 13 and 15.
const MAIN_THIRD: &str = r#"// k2 is resumed from frames that a resume of k3 entered, while the outer
// frame of k3 waits.
main = () {
	// The computations need `Out`. This one is never called.
	{
		k2 = paused()
		k3 = { control.boundary((b3) { o1(b3, k2) }) } |>
			(Third(k3)) { k3 }
			| (Over) { panic("not paused") }
		control.resume(k3, Nil)
		Nil
	} <- (Out(put = (k2) { Nil }))
}
"#;

/// Test 14, resume: the tail call of `h_tail` leaves no frame that keeps
/// `! Name = Name("in")`, so `g` sees "resumer", against §9.4 rule 3
/// (section 7a).
const ENV_TAIL_RESUME: &str = r#"(
	i/control
)

Name = (text = String)
Paused = (Continuation(Nil, Paused, Never, Name))
Done = ()
Paused = Paused | Done

name = () <- (n: Name) { n.text }

g = (b) {
	control.suspend(b, (k) { Paused(k) })
	panic(name())
}

// The call of `g` is a tail call, so no frame keeps the environment
// statement when `g` is paused (section 7a of docs/design/lazy-reentry.md).
h_tail = (b) { g(b) <- (Name(text = "in")) }

main = () {
	{
		k = {
			control.boundary((b) {
				h_tail(b)
				Done
			})
		} |>
			(Paused(k)) { k }
			| (Done) { panic("not paused") }
		{
			control.resume(k, Nil)
			Nil
		} <- (Name(text = "resumer"))
	} <- (Name(text = "start"))
}
"#;

/// Test 14, cancel: the same for the final function of a cancel.
const ENV_TAIL_CANCEL: &str = r#"(
	i/control
	i/resource
)

Name = (text = String)
Paused = (Continuation(Nil, Paused, Never, Name))
Done = ()
Paused = Paused | Done

name = () <- (n: Name) { n.text }

g = (b) {
	resource.ensure(
		() {
			control.suspend(b, (k) { Paused(k) })
			Nil
		},
		() { panic(name()) },
	)
}

// The call of `g` is a tail call, so no frame keeps the environment
// statement when `g` is paused (section 7a of docs/design/lazy-reentry.md).
h_tail = (b) { g(b) <- (Name(text = "in")) }

main = () {
	{
		k = {
			control.boundary((b) {
				h_tail(b)
				Done
			})
		} |>
			(Paused(k)) { k }
			| (Done) { panic("not paused") }
		{
			control.cancel(k)
			Nil
		} <- (Name(text = "resumer"))
	} <- (Name(text = "start"))
}
"#;

/// Test 16: the value of `f` of `inner` goes on into the cancelled code.
const CANCEL_INNER: &str = r#"(
	i/control
	i/resource
	(Io) = i/io
)

Paused = (Continuation(Nil, Paused, Never, Io))
Done = ()
Paused = Paused | Done

say = (text) <- (io: Io) { io.print(text) }

// During the cancel, the final function suspends to `inner`, which is
// paused inside `k`; the value of its `f` goes on into the code after it.
body = (outer) {
	x = control.boundary((inner) {
		resource.ensure(
			() {
				control.suspend(outer, (k) { Paused(k) })
				"body"
			},
			() {
				say("final")
				y = control.suspend(inner, (k2) { "f of inner" })
				say("final again: " <> y)
			},
		)
	})
	say("after inner: " <> x)
	Done
}

main = () <- (io: Io) {
	k = { control.boundary((outer) { body(outer) }) } |>
		(Paused(k)) { k }
		| (Done) { panic("not paused") }
	say("cancel")
	control.cancel(k)
	say("cancel gave Nil")
}
"#;

/// An error goes through a pending final record (kind 5, skipped in mode
/// 1). A final function cannot raise an error itself (§9.5), but D15's
/// cancel through an inner boundary can run ordinary code that raises one.
/// This is checked against eager, rather than made a conformance rule.
const ERROR_THROUGH_FINAL: &str = r#"(
	i/control
	i/core
	i/fail
	i/resource
	(Io) = i/io
)

Paused = (Continuation(Nil, Paused, Int, Never))
Done = ()
Paused = Paused | Done
Held = (Continuation(Nil, Held, Never, Never))
Over = ()
Held = Held | Over

// Cancelling this continuation can return to the code after `inner`,
// because the final function suspends to `inner` during the cancel (D15).
paused = () {
	{
		control.boundary((outer) {
			control.boundary((inner) {
				resource.ensure(
					() {
						control.suspend(outer, (k) { Paused(k) })
						Nil
					},
					() { control.suspend(inner, (k2) { Nil }) },
				)
			})
			fail.raise(7)
			Done
		})
	} |>
		(Paused(k)) { k }
		| (Done) { panic("not paused") }
}

// `cancel` has no errors in its type. Here it runs the ordinary code
// after `inner`, whose error passes through this final function's record.
with = (hold, k) {
	control.suspend(hold, (k2) { Held(k2) })
	control.cancel(k)
}

main = () <- (io: Io) {
	k = paused()
	r = fail.catch(() {
		held = control.boundary((hold) {
			resource.ensure(() { 99 }, () { with(hold, k) })
			Over
		})
		held |>
			(Held(k2)) { control.resume(k2, Nil) }
			| (Over) { panic("not held") }
	})
	io.print(core.inspect(r))
}
"#;

/// Each program, and the standard output, the standard error and the exit
/// code that the eager compiler gives for it, at `--opt 0` and `--opt 2`.
fn eager(name: &str) -> (String, &'static str, &'static str, i32) {
    match name {
        "cancel_out_trace" => (
            [CANCEL_OUT, G_PLAIN, PAUSED, MAIN_OUT].concat(),
            "",
            "control: boundary is not active\napp/main:20:7\napp/main:61:3\n",
            1,
        ),
        "cancel_out_trace_catch" => (
            [CANCEL_OUT, G_CATCH, PAUSED, MAIN_OUT].concat(),
            "",
            "control: boundary is not active\napp/main:21:3\napp/main:20:6\napp/main:62:3\n",
            1,
        ),
        "cancel_out_two_trampolines" => (
            [CANCEL_OUT, G_PLAIN, PAUSED, THIRD, O1_PLAIN, MAIN_THIRD].concat(),
            "",
            "control: boundary is not active\napp/main:20:7\napp/main:64:16\napp/main:68:2\napp/main:72:22\napp/main:83:3\n",
            1,
        ),
        "cancel_out_restores_before_mode2" => (
            [CANCEL_OUT, G_PLAIN, PAUSED, THIRD, O1_ENSURE, MAIN_THIRD].concat(),
            "",
            "panic: final\napp/main:72:61\napp/main:72:22\napp/main:83:3\n",
            1,
        ),
        "env_tail_call_resume" => (
            ENV_TAIL_RESUME.to_string(),
            "",
            "panic: resumer\napp/main:14:2\napp/main:25:5\napp/main:32:4\n",
            1,
        ),
        "env_tail_call_cancel" => (
            ENV_TAIL_CANCEL.to_string(),
            "",
            "panic: resumer\napp/main:19:8\napp/main:31:5\napp/main:38:4\n",
            1,
        ),
        "cancel_suspends_to_inner_boundary" => (
            CANCEL_INNER.to_string(),
            "cancel\nfinal\nafter inner: f of inner\ncancel gave Nil\n",
            "",
            0,
        ),
        "error_through_pending_final" => (ERROR_THROUGH_FINAL.to_string(), "Error(7)\n", "", 0),
        _ => panic!("no program {name}"),
    }
}

/// The module of `source` for the WASI target at `--opt 0`. It imports only
/// what the `wasmtime` command gives.
fn wasi_module(source: &str) -> Vec<u8> {
    let (graph, c, _dir) = checked(source);
    let wasm = compile(
        &graph,
        c.program.as_ref().unwrap(),
        c.types.as_ref().unwrap(),
        Options {
            tests: false,
            target: Target::Wasi,
        },
    )
    .unwrap();
    for payload in wasmparser::Parser::new(0).parse_all(&wasm) {
        if let wasmparser::Payload::ImportSection(s) = payload.unwrap() {
            for i in s.into_imports() {
                let i = i.unwrap();
                assert!(
                    i.module == "wasi_snapshot_preview1"
                        && ["fd_write", "proc_exit"].contains(&i.name),
                    "the module imports {} {}, which the wasmtime command does not give",
                    i.module,
                    i.name
                );
            }
        }
    }
    wasm
}

/// Runs the program `name` at `--opt 0`, and checks that it gives what the
/// eager compiler gives.
fn same_as_eager(name: &str) {
    let (source, stdout, stderr, code) = eager(name);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.wasm");
    std::fs::write(&path, wasi_module(&source)).unwrap();
    let wasmtime = env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into());
    let out = Command::new(&wasmtime)
        .args([
            "run",
            "-W",
            "function-references=y,tail-call=y,exceptions=y,gc=y",
        ])
        .arg(&path)
        .output()
        .unwrap_or_else(|e| {
            panic!("cannot run {wasmtime:?} ({e}); run inside Devenv, or set WASMTIME")
        });
    let got = (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    );
    assert_eq!(
        got,
        (stdout.to_string(), stderr.to_string(), Some(code)),
        "{name}"
    );
}

/// Test 12 (D13, P4): a `$cancel` goes out of a trampoline at the end of its
/// list and reaches the entry, which prints the full trace.
#[test]
fn cancel_out_trace() {
    same_as_eager("cancel_out_trace");
    same_as_eager("cancel_out_trace_catch");
}

/// Test 13 (D13, P4): two trampolines put their slots back in turn, at
/// their own depth.
#[test]
fn cancel_out_two_trampolines() {
    same_as_eager("cancel_out_two_trampolines");
}

/// Test 14 (D16): an environment statement before a tail call is lost, as
/// in eager (section 7a).
#[test]
fn env_tail_call_as_eager() {
    same_as_eager("env_tail_call_resume");
    same_as_eager("env_tail_call_cancel");
}

/// Test 15 (D13, P3): the depth is put back just before a record is called
/// in mode 2, so the trace of the final function has no stale lines.
#[test]
fn cancel_out_restores_before_mode2() {
    same_as_eager("cancel_out_restores_before_mode2");
}

/// Test 16 (D15): a final function of a cancel suspends to a boundary that
/// was paused inside the cancelled computation.
#[test]
fn cancel_suspends_to_inner_boundary() {
    same_as_eager("cancel_suspends_to_inner_boundary");
}

/// D2 and 13.3: an error passes a final record without calling it. D15's
/// cancel behaviour supplies the error without giving a final an error type.
#[test]
fn error_through_pending_final() {
    same_as_eager("error_through_pending_final");
}

// Sections 13.1 and 13.2: use the same flat driver for fuel and deep runs.
// Keeping the source shared prevents the fuel check from measuring a toy
// runtime path instead of the benchmark's resume and re-suspend.
fn bench_source(depth: u32, rounds: u32, capture_only: bool) -> String {
    let source = include_str!("../../../bench/test/control_test.i");
    let (module, diagnostics) = ilang_syntax::parse_module(source);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let end = module
        .items
        .iter()
        .find(|item| matches!(item.kind, ilang_syntax::ast::ItemKind::Macro { .. }))
        .expect("benchmark has test registration calls")
        .span
        .0
        .start as usize;
    let import = module
        .imports
        .iter()
        .find(|import| {
            import
                .path
                .iter()
                .map(|part| part.name.as_str())
                .eq(["std", "test"])
        })
        .expect("benchmark imports its test macros")
        .span
        .0;
    let source = format!(
        "{}{}",
        &source[..import.start as usize],
        &source[import.end as usize..end]
    );
    let source = source
        .replace("rounds = 100_000", &format!("rounds = {rounds}"))
        .replace(
            "5_000_050_000",
            &(u64::from(rounds) * u64::from(rounds + 1) / 2).to_string(),
        );
    let main = if capture_only {
        "\n\nmain = () { control.boundary((b) { down(b, DEPTH) })  Nil }\n"
    } else {
        "\n\nmain = () { run(DEPTH) }\n"
    }
    .replace("DEPTH", &depth.to_string());
    format!("{source}{main}")
}

const FEATURES: &str = "function-references=y,tail-call=y,exceptions=y,gc=y";

/// Each deep child gets a native stack of 64 MiB when the OS permits it.
/// Read the limit in the same shell that execs Wasmtime. A signal is an
/// error, never evidence of a Wasm stack limit (13.2, Stack sizes).
struct Runner {
    dir: tempfile::TempDir,
    native: u64,
    stack: u64,
}

impl Runner {
    fn new(requested: u64) -> Self {
        let out = Command::new("sh")
            .args(["-c", "ulimit -s 65536 2>/dev/null; ulimit -s"])
            .output()
            .unwrap();
        assert!(out.status.success());
        let native = String::from_utf8(out.stdout)
            .unwrap()
            .trim()
            .parse::<u64>()
            .expect("native stack limit in KiB")
            * 1024;
        let stack = requested.min(native / 2);
        eprintln!("Wasm stack S={stack} bytes; native stack={native} bytes");
        Self {
            dir: tempfile::tempdir().unwrap(),
            native,
            stack,
        }
    }

    fn command(&self, verb: &str, fuel: Option<u64>) -> Command {
        let wasmtime = env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into());
        let mut cmd = Command::new("sh");
        cmd.args([
            "-c",
            "ulimit -s 65536 2>/dev/null; ulimit -s >&2; exec \"$0\" \"$@\"",
        ])
        .arg(wasmtime)
        .arg(verb)
        .args(["-W", FEATURES])
        .arg("-W")
        .arg(format!("max-wasm-stack={}", self.stack));
        if let Some(n) = fuel {
            cmd.arg("-W").arg(format!("fuel={n}"));
        }
        cmd
    }

    fn module(&self, source: &str, fuel: bool) -> std::path::PathBuf {
        let path = self.dir.path().join("main.wasm");
        std::fs::write(&path, wasi_module(source)).unwrap();
        let compiled = path.with_extension("cwasm");
        let out = self
            .command("compile", fuel.then_some(1))
            .arg(&path)
            .arg("-o")
            .arg(&compiled)
            .output()
            .unwrap();
        let err = self.stderr(&out);
        if out.status.success() {
            return compiled;
        }
        // Section 13.2 allows .wasm if this CLI cannot precompile fuel.
        // Do not turn a compile failure without fuel into a fallback.
        assert!(fuel, "wasmtime compile failed: {err}");
        eprintln!("precompiled fuel unavailable; using .wasm: {err}");
        path
    }

    fn stderr(&self, out: &std::process::Output) -> String {
        assert!(
            out.status.code().is_some(),
            "Wasmtime died by signal; native={} S={}: {:?}",
            self.native,
            self.stack,
            out
        );
        let err = String::from_utf8_lossy(&out.stderr);
        let (limit, rest) = err.split_once('\n').expect("child native stack limit");
        assert_eq!(limit.trim().parse::<u64>().unwrap() * 1024, self.native);
        rest.to_string()
    }

    fn run(&self, path: &std::path::Path, fuel: Option<u64>) -> (bool, String) {
        let mut cmd = self.command("run", fuel);
        if path.extension().unwrap() == "cwasm" {
            cmd.arg("--allow-precompiled");
        }
        let out = cmd.arg(path).output().unwrap();
        let err = self.stderr(&out);
        if out.status.success() {
            assert!(err.is_empty(), "{err}");
            assert!(out.stdout.is_empty(), "{:?}", out.stdout);
            return (true, err);
        }
        assert!(
            err.contains("all fuel consumed")
                || err.contains("call stack exhausted")
                || err.contains("There is no more memory or stack"),
            "unexpected failure: {err}"
        );
        (false, err)
    }

    fn finishes(&self, source: &str) -> bool {
        self.run(&self.module(source, false), None).0
    }

    fn fuel(&self, source: &str) -> u64 {
        let module = self.module(source, true);
        let mut high = 1_u64;
        loop {
            let (done, err) = self.run(&module, Some(high));
            if done {
                break;
            }
            assert!(err.contains("all fuel consumed"), "{err}");
            high = high.checked_mul(2).expect("fuel upper bound");
        }
        let mut low = high / 2;
        while high - low > (high / 2000).max(1) {
            let mid = low + (high - low) / 2;
            let (done, err) = self.run(&module, Some(mid));
            if done {
                high = mid;
            } else {
                assert!(err.contains("all fuel consumed"), "{err}");
                low = mid;
            }
        }
        eprintln!("fuel in ({low}, {high}], bracket <0.05%");
        high
    }
}

#[test]
fn resume_cost_is_flat() {
    let runner = Runner::new(32 * 1024 * 1024);
    let mut shallow = 0.;
    for depth in [10, 100, 1000] {
        let f1 = runner.fuel(&bench_source(depth, 1000, false));
        let f2 = runner.fuel(&bench_source(depth, 4000, false));
        let marginal = (f2 - f1) as f64 / 3000.;
        if depth == 10 {
            shallow = marginal;
        }
        eprintln!(
            "depth={depth} F1000={f1} F4000={f2} marginal={marginal} ratio={}",
            marginal / shallow
        );
        assert!(marginal / shallow <= 1.05, "13.2 test 6: depth={depth}");
    }
}

#[test]
fn deep_resume() {
    let runner = Runner::new(32 * 1024 * 1024);
    let depth = if runner.native >= 64 * 1024 * 1024 {
        100_000
    } else {
        10_000
    };
    assert!(runner.finishes(&bench_source(depth, 1000, false)));
}

// The tree has a left subtree of depth n-1 and a right leaf. Each level
// composes a child generator by resuming it inside its own generator.
// This gives a complete in-order walk with 2*n+1 values, rather than an
// exponential number of nodes, while retaining the nested-resume stack.
fn nested_source(depth: u32) -> String {
    r#"(
	i/control
)
Done = ()
More = (Int, Continuation(Nil, Step, Never, Never))
Step = Done | More
check = (ok) { ok |> (True) { Nil } | (False) { panic("tree walk") } }
emit = (b, n) { control.suspend(b, (k) { More(n, k) }) }
copy = (b, step) {
	step |>
		(Done) { Nil }
		| (More(v, k)) {
			emit(b, v)
			copy(b, control.resume(k, Nil))
		}
}
walk = (b, n) {
	n |>
		(0) { emit(b, 0) }
		| (_) {
			copy(
				b,
				control.boundary((child) {
					walk(child, n - 1)
					Done
				}),
			)
			emit(b, n)
			emit(b, -n)
		}
}
next = (n) { { n > 0 } |> (True) { -n } | (False) { 1 - n } }
drive = (step, count, sum, want) {
	step |>
		(Done) {
			check(count == TreeCount)
			check(sum == 0)
		}
		| (More(v, k)) {
			check(v == want)
			drive(control.resume(k, Nil), count + 1, sum + v, next(want))
		}
}

main = () {
	drive(
		control.boundary((b) {
			walk(b, TreeDepth)
			Done
		}),
		0,
		0,
		0,
	)
}
"#
    .replace("TreeCount", &(2 * depth + 1).to_string())
    .replace("TreeDepth", &depth.to_string())
}

// Eager 1df2fe0, Wasmtime 49.0.2, x86_64 Linux (14 step 2).
// At S=131072 the last complete tree walk has L=219. Recursion alone
// needs (5600085, 5601109] bytes; S11=1.5*5601109=8401663. At S11
// eager recursion with a resume fails with a clean Wasm stack trap.
const NESTED_STACK: u64 = 128 * 1024;
const EAGER_NESTED_DEPTH: u32 = 219;
const RECURSION_STACK: u64 = 8_401_663;

#[test]
fn nested_resume_stack() {
    let runner = Runner::new(NESTED_STACK);
    assert_eq!(runner.stack, NESTED_STACK, "baseline needs this fixed S");
    let mut low = 0;
    let mut high = 32;
    while runner.finishes(&nested_source(high)) {
        low = high;
        high *= 2;
        assert!(high <= 16384, "no nested-resume stack limit found");
    }
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if runner.finishes(&nested_source(mid)) {
            low = mid;
        } else {
            high = mid;
        }
    }
    eprintln!("largest nested depth L={low} at S={}", runner.stack);
    assert!(
        u64::from(low) * 5 >= u64::from(EAGER_NESTED_DEPTH) * 4,
        "13.2 test 10: L={low}, eager L={EAGER_NESTED_DEPTH}"
    );
}

fn recursion_source(paused: bool) -> String {
    let source = bench_source(50_000, 1, false);
    let (source, _) = source.split_once("\nmain =").expect("benchmark entry");
    let recurse = "rec = (n, body) { n |> (0) { body() } | (_) { rec(n - 1, body) + 1 } }";
    let main = if paused {
        "done = (Done(n)) { n } | (_) { panic(\"not done\") }\n\
         \n\
         main = () {\n\
         \tk = control.boundary((b) { down(b, 50_000) }) |>\n\
         \t\t(More(_, k)) { k }\n\
         \t\t| (_) { panic(\"not paused\") }\n\
         \tcheck(\n\
         \t\trec(50_000, () { done(control.resume(k, Nil)) }) == 100_000,\n\
         \t\t\"recursion lost a frame\",\n\
         \t)\n\
         }"
    } else {
        "\nmain = () { check(rec(50_000, () { 0 }) == 50_000, \"recursion lost a frame\") }"
    };
    format!("{source}\n{recurse}\n{main}\n")
}

#[test]
fn deep_resume_in_recursion() {
    let runner = Runner::new(RECURSION_STACK);
    assert_eq!(
        runner.stack, RECURSION_STACK,
        "baseline needs this fixed S11"
    );
    assert!(runner.finishes(&recursion_source(true)));
}

fn runs_cleanly(source: &str) {
    assert!(Runner::new(8 * 1024 * 1024).finishes(source));
}

/// Section 13.2 test 8: a kind-1 frame stays pending across two resumes;
/// rule 3 of spec 9.4 supplies the new base, keeping its own statement.
#[test]
fn resume_twice_from_new_places() {
    runs_cleanly(
        r#"(
	i/control
)
Name = (text = String)
Done = (String)
More = (String, Continuation(Nil, Step, Never, Name))
Step = Done | More
name = () <- (n: Name) { n.text }
leaf = (b) {
	control.suspend(b, (k) { More("start", k) })
	first = name()
	control.suspend(b, (k) { More(first, k) })
	first <> "," <> name()
}
inside = (b) {
	{
		x = leaf(b)
		name() <> "/" <> x
	} <- (Name(text = "inside"))
}
run = (b) {
	x = inside(b)
	Done(x <> "/" <> name())
}
check = (ok) { ok |> (True) { Nil } | (False) { panic("resume environment") } }

main = () {
	{
		k = control.boundary((b) { run(b) }) |>
			(More(_, k)) { k }
			| (_) { panic("not paused") }
		{
			k2 = control.resume(k, Nil) |>
				(More(s, k)) {
					check(s == "inside")
					k
				}
				| (_) { panic("not paused twice") }
			control.resume(k2, Nil) |>
				(Done(s)) { check(s == "inside/inside,inside/second") }
				| (_) { panic("not done") } <- (Name(text = "second"))
		} <- (Name(text = "first"))
	} <- (Name(text = "start"))
}
"#,
    );
}

/// Section 13.2 test 9: three inner frames finish before the next pause;
/// further resumes enter the remaining frames once, then cancel runs the
/// pending final function once (spec 9.4 rules 3/4 and spec 9.5).
#[test]
fn counted_frames() {
    runs_cleanly(
        r#"(
	i/control
	i/core
	i/resource
)
Done = ()
More = (String, Continuation(Nil, Step, Never, Never))
Step = Done | More
check = (ok) { ok |> (True) { Nil } | (False) { panic("counted frame") } }
layer = (b, n) {
	n |>
		(0) {
			control.suspend(b, (k) { More("start", k) })
			"0"
		}
		| (_) {
			x = layer(b, n - 1)
			n |> (4) { control.suspend(b, (k) { More(x, k) }) } | (_) { Nil }
			core.inspect(n) <> x
		}
}
next = (step, want) {
	step |>
		(More(s, k)) {
			check(s == want)
			k
		}
		| (Done) { panic("not paused") }
}

main = () {
	text = control.boundary((log) {
		k = next(
			control.boundary((b) {
				resource.ensure(
					() {
						x = layer(b, 6)
						control.suspend(b, (k) { More(x, k) })
						panic("cancel must not resume")
					},
					() {
						control.suspend(log, (k) {
							"final " <> control.resume(k, Nil)
						})
					},
				)
				Done
			}),
			"start",
		)
		k2 = next(control.resume(k, Nil), "3210")
		k3 = next(control.resume(k2, Nil), "6543210")
		control.cancel(k3)
		"end"
	})
	check(text == "final end")
}
"#,
    );
}
