//! The structured i scheduler on the real Rust host. Completion gates and
//! invocation teardown are tested separately from spec output expectations.

use ilang_cli::{BuildOptions, Level, build};
use ilang_host::{HostOptions, Outcome, Program, services};

fn program(source: &str, opt: Level) -> Program {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(directory.path().join("src")).unwrap();
    std::fs::write(directory.path().join("src/main.i"), source).unwrap();
    let built = build(
        directory.path(),
        BuildOptions {
            opt,
            tests: false,
            offline: false,
            target: Default::default(),
        },
    )
    .unwrap();
    Program::from_bytes(&built.wasm).unwrap()
}

#[test]
fn both_async_children_complete_on_the_rust_host() {
    let source = include_str!("../../../tests/spec/9.9/async_tasks.i");
    for opt in [Level::O0, Level::O2] {
        let program = program(source, opt);
        let log = services::Log::default();
        let outcome = program
            .run_entry(HostOptions::new(services::testing(log.clone())))
            .unwrap();
        assert!(matches!(outcome, Outcome::Ok(_)), "{outcome:?}");
        assert_eq!(*log.lock().unwrap(), ["double 21", "echo answer"]);
    }
}

use ilang_host::{HostImports, ResourceContext, Value, block_on};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct GateState {
    gates: Vec<GateRecord>,
}
struct GateRecord {
    id: i64,
    ready: bool,
    polls: usize,
    completed: bool,
    dropped: bool,
    waker: Option<Waker>,
    context: ResourceContext,
}
struct Gate {
    state: Arc<Mutex<GateState>>,
    index: usize,
}
impl Future for Gate {
    type Output = anyhow::Result<Result<Value, Value>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        let gate = &mut state.gates[self.index];
        assert!(gate.context.is_live(), "polled after invocation teardown");
        gate.polls += 1;
        if gate.ready {
            gate.completed = true;
            Poll::Ready(Ok(Ok(Value::Int(gate.id.into()))))
        } else {
            gate.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.state.lock().unwrap().gates[self.index].dropped = true;
    }
}
fn number(args: &[Value]) -> i64 {
    let [Value::Int(value)] = args else {
        panic!("one Int argument")
    };
    value.try_into().unwrap()
}
fn gate_imports(built: &ilang_cli::Built, state: Arc<Mutex<GateState>>) -> HostImports {
    let mut imports = HostImports::default();
    for declaration in &built.binding_plan.declarations {
        imports.grant(&built.binding_plan, declaration);
    }
    let waiting = state.clone();
    imports
        .context_async_service("r2:gates/v1", "wait", move |context, args| {
            let id = number(args);
            assert!(context.is_live());
            let mut state = waiting.lock().unwrap();
            let index = state.gates.len();
            state.gates.push(GateRecord {
                id,
                ready: id == 9,
                polls: 0,
                completed: false,
                dropped: false,
                waker: None,
                context,
            });
            Gate {
                state: waiting.clone(),
                index,
            }
        })
        .unwrap();
    let releasing = state.clone();
    imports
        .context_service("r2:control/v1", "release", move |context, args| {
            assert!(context.is_live());
            let id = number(args);
            let mut state = releasing.lock().unwrap();
            assert_eq!(
                state.gates.iter().map(|g| g.id).collect::<Vec<_>>(),
                [1, 2],
                "both operations must start before release"
            );
            for gate in &mut state.gates {
                if id == 0 || id == gate.id {
                    gate.ready = true;
                    if let Some(waker) = gate.waker.take() {
                        waker.wake();
                    }
                }
            }
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
        .context_service("r2:control/v1", "check", move |context, args| {
            assert!(context.is_live());
            let state = state.lock().unwrap();
            let first = &state.gates[0];
            match number(args) {
                1 => assert!(
                    first.dropped && !first.completed,
                    "owning cancellation drops producer before scope returns"
                ),
                2 | 3 => assert!(
                    !first.dropped && !first.completed,
                    "borrowed cancellation leaves producer alive"
                ),
                4 => assert_eq!(
                    first.polls, 0,
                    "raw-only runnable tasks do not authorize polls"
                ),
                phase => panic!("unknown phase {phase}"),
            }
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
}
fn fixture(opt: Level) -> ilang_cli::Built {
    build(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/browser/fixtures/r2")
            .as_path(),
        BuildOptions {
            opt,
            tests: false,
            offline: true,
            target: Default::default(),
        },
    )
    .unwrap()
}
fn expect_int(outcome: Outcome, expected: i64) {
    assert!(
        matches!(outcome, Outcome::Ok(Value::Int(ref n)) if *n == expected.into()),
        "{outcome:?}"
    );
}

#[test]
fn completion_gates_and_root_accounting_on_the_rust_host() {
    for opt in [Level::O0, Level::O2] {
        let built = fixture(opt);
        let program = built.host_program().unwrap();
        for _ in 0..200 {
            for name in [
                "overlap",
                "unlisted_answer",
                "cancel_owned",
                "cancel_borrowed",
                "race_read",
                "nested_interest",
                "owned_idle",
                "unrelated_idle",
                "raw_only",
            ] {
                let state = Arc::new(Mutex::new(GateState::default()));
                let mut instance = program
                    .instantiate(HostOptions::new(gate_imports(&built, state.clone())))
                    .unwrap();
                let outcome =
                    block_on(instance.call_async(&format!("app/r2.{name}"), &[])).unwrap();
                match name {
                    "overlap" | "unlisted_answer" => expect_int(outcome, 3),
                    "race_read" => expect_int(outcome, 2),
                    "nested_interest" => expect_int(outcome, 9),
                    "raw_only" => expect_int(outcome, 7),
                    "owned_idle" | "unrelated_idle" => {
                        let Outcome::Stop(stop) = outcome else {
                            panic!("{name}: {outcome:?}")
                        };
                        assert!(
                            stop.message
                                .as_ref()
                                .is_some_and(|message| message.lines().next()
                                    == Some("panic: task: deadlock")),
                            "{name}: {stop:?}"
                        );
                    }
                    _ => assert!(
                        matches!(outcome, Outcome::Ok(Value::Nil)),
                        "{name}: {outcome:?}"
                    ),
                }
                let state = state.lock().unwrap();
                assert!(!state.gates.is_empty());
                assert!(
                    state
                        .gates
                        .iter()
                        .all(|g| g.dropped && !g.context.is_live())
                );
                if name == "owned_idle" {
                    assert_eq!(state.gates[0].polls, 1);
                }
                if name == "unrelated_idle" {
                    assert_eq!(state.gates[0].polls, 0);
                }
            }
        }
    }
}

#[test]
fn seeded_script_replays_200_times_at_each_optimization_level() {
    let mut cross_level = None;
    for opt in [Level::O0, Level::O2] {
        let built = fixture(opt);
        let program = built.host_program().unwrap();
        let mut instance = program
            .instantiate(HostOptions::new(gate_imports(&built, Default::default())))
            .unwrap();
        let args = [Value::Int((-7).into())];
        let expected = instance
            .call("app/r2.expected_trace", &args)
            .unwrap()
            .into_value()
            .unwrap();
        let Value::String(expected) = expected else {
            panic!("String expected")
        };
        if let Some(previous) = cross_level.as_ref() {
            assert_eq!(&expected, previous);
        }
        cross_level = Some(expected.clone());
        for iteration in 0..200 {
            let outcome = block_on(instance.call_async("app/r2.trace", &args)).unwrap();
            assert!(
                matches!(outcome, Outcome::Ok(Value::String(ref trace)) if *trace == expected),
                "iteration {iteration}: {outcome:?}"
            );
        }
        expect_int(instance.call("app/r2.fake", &[]).unwrap(), 7);
        expect_int(
            block_on(instance.call_async("app/r2.pure_tasks", &[])).unwrap(),
            7,
        );
        assert!(matches!(
            instance
                .call("app/r2.cmp2", &[Value::Float(3.0), Value::Float(2.0)])
                .unwrap(),
            Outcome::Ok(Value::Float(1.0))
        ));
        let Outcome::Stop(stop) = instance.call("app/r2.sync_poll", &[]).unwrap() else {
            panic!("sync poll must stop")
        };
        assert!(
            stop.message
                .unwrap()
                .starts_with("panic: request: no active entry\n")
        );
    }
}
