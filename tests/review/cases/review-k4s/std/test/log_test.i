(
	testlib = (test) = std/test
	i/control
	i/fail
	std/list
	std/log
	std/time
	(Some, None) = std/option
)

Event = Sent | Message

Sent {
	Sent(log.Entry)
}

Message {
	Message
}

Capture {
	Capture(record: (Event) -> Nil)
}

record = (event: Event, capture ! Capture) -> Nil ! Capture {
	capture.record(event)
}

// std-2 §7: finite, closed instantiations retain service and error types.
deliver: (log.Entry) -> Nil ^ String ! Capture | log.Sink(
	String
	Capture
) = log.write

query: (log.Level) -> Bool ! log.Sink(String, Capture) = log.enabled

defer_message: (
	log.Level
	() -> String ^ String ! Capture
) -> Nil ^ String ! Capture | log.Sink(String, Capture) = log.lazy

capture_sink = () -> log.Sink(String, Capture) {
	log.make((_: log.Level) { True }, (entry) { record(Sent(entry)) })
}

// A pure divergent predicate detects an unwanted call without adding effects.
unreachable = (level: log.Level) -> Bool { unreachable(level) }

entry = (level: log.Level) -> log.Entry { log.Entry(level, "hello", [], None) }

// L1/L2 and the sequential service law: entry fields and timestamps are data.

test("log_delivery", () {
	original = log.Entry(
		log.Info
		"hello"
		[("", ""), ("k", "1"), ("k", "2")]
		Some(time.from_unix_nanoseconds(-1))
	)
	producer = () { deliver(original) }
	trace = control.boundary((b) {
		! Capture = Capture((event) {
			control.suspend(b, (k) { [event, ..control.resume(k, Nil)] })
		})
		! log.Sink(String, Capture) = log.silent()
		producer()
		{
			! log.Sink(String, Capture) = capture_sink()
			assert(query(log.Info))
			producer()
			deliver(entry(log.Warning))
		}
		producer()
		[]
	})
	assert(trace == [Sent(original), Sent(entry(log.Warning))])
	Nil
})

// L3: rejection suppresses both callbacks; eager arguments still evaluate.

test("log_lazy", () {
	trace = control.boundary((b) {
		! Capture = Capture((event) {
			control.suspend(b, (k) { [event, ..control.resume(k, Nil)] })
		})
		! log.Sink(String, Capture) = log.silent()
		assert(query(log.Info) == False)
		defer_message(log.Info, () { fail.raise("must not run") })
		deliver({
			record(Message)
			entry(log.Info)
		})
		{
			! log.Sink(String, Capture) = capture_sink()
			defer_message(log.Info, () {
				record(Message)
				"built"
			})
			assert(fail.catch(() {
				defer_message(log.Info, () {
					record(Message)
					fail.raise("bad")
				})
			}) == Error("bad"))
		}
		[]
	})
	assert(
		trace ==
			[
				Message
				Message
				Sent(log.Entry(log.Info, "built", [], None))
				Message
			]
	)
	Nil
})

// L4 and both filter laws: thresholds follow L1's four explicitly ordered levels.

test("log_filters", () {
	levels = [log.Debug, log.Info, log.Warning, log.Failure]
	cases = [
		(log.Debug, levels)
		(log.Info, [log.Info, log.Warning, log.Failure])
		(log.Warning, [log.Warning, log.Failure])
		(log.Failure, [log.Failure])
	]
	list.each(cases, (case) {
		(threshold, expected) = case
		trace = control.boundary((b) {
			! Capture = Capture((event) {
				control.suspend(b, (k) { [event, ..control.resume(k, Nil)] })
			})
			! log.Sink(String, Capture) = log.minimum(capture_sink(), threshold)
			list.each(levels, (level) { deliver(entry(level)) })
			[]
		})
		assert(trace == list.map(expected, (level) { Sent(entry(level)) }))
		list.each(cases, (inner_case) {
			(inner, inner_levels) = inner_case
			trace = control.boundary((b) {
				! Capture = Capture((event) {
					control.suspend(b, (k) {
						[event, ..control.resume(k, Nil)]
					})
				})
				! log.Sink(String, Capture) = log.minimum(
					log.minimum(capture_sink(), inner)
					threshold
				)
				list.each(levels, (level) { deliver(entry(level)) })
				[]
			})
			// Intersection of the two explicitly enumerated accepted-level tables.
			assert(
				trace ==
					list.map(
						list.filter(expected, (level) {
							list.contains(inner_levels, level)
						})
						(level) { Sent(entry(level)) }
					)
			)
		})
	})
	trace = control.boundary((b) {
		! Capture = Capture((event) {
			control.suspend(b, (k) { [event, ..control.resume(k, Nil)] })
		})
		! log.Sink(String, Capture) = log.filter(
			capture_sink()
			(_: log.Level) { True }
		)
		list.each(levels, (level) { deliver(entry(level)) })
		[]
	})
	assert(trace == list.map(levels, (level) { Sent(entry(level)) }))
	// L4 requires new_accept before old_accept and short-circuiting.
	! log.Sink(String, Capture) = log.filter(
		log.make(unreachable, (_: log.Entry) -> Nil ! Capture { Nil })
		(_: log.Level) { False }
	)
	assert(query(log.Info) == False)
	Nil
})

DeliveryError {
	DeliveryError(String)
}

broken_deliver: (log.Entry) -> Nil ^ DeliveryError ! Capture | log.Sink(
	DeliveryError
	Capture
) = log.write

// L2/L5: a custom error is preserved, with no retry or swallowed failure.

test("log_errors", () {
	trace = control.boundary((b) {
		! Capture = Capture((event) {
			control.suspend(b, (k) { [event, ..control.resume(k, Nil)] })
		})
		! log.Sink(DeliveryError, Capture) = log.make(
			(_: log.Level) { True }
			(event) {
				record(Sent(event))
				fail.raise(DeliveryError(event.message))
			}
		)
		assert(
			fail.catch(() { broken_deliver(entry(log.Info)) }) ==
				Error(DeliveryError("hello"))
		)
		[]
	})
	assert(trace == [Sent(entry(log.Info))])
	Nil
})

// L6: fixed expected bytes derive from the format rule, not core.inspect.

test("log_format", () {
	assert(
		log.format(
			log.Entry(log.Info, "a\nb", [("k", "v"), ("k", "")], None)
		) ==
			"info \"a\\nb\" \"k\"=\"v\" \"k\"=\"\""
	)
	assert(
		log.format(
			log.Entry(
				log.Debug
				"\"\\\r\t"
				[("\n", "\u{0}\u{1b}\u{2028}\u{2029}")]
				Some(time.from_unix_nanoseconds(-1))
			)
		) ==
			"debug \"\\\"\\\\\\r\\t\" @-1 \"\\n\"=\"\\u{0}\\u{1b}\\u{2028}\\u{2029}\""
	)
	assert(
		log.format(
			log.Entry(
				log.Warning
				"é漢🙂"
				[("", "\u{7f}\u{80}\u{9f}\u{a0}")]
				Some(time.from_unix_nanoseconds(0))
			)
		) ==
			"warning \"é漢🙂\" @0 \"\"=\"\\u{7f}\\u{80}\\u{9f}\u{a0}\""
	)
	assert(log.format(entry(log.Failure)) == "failure \"hello\"")
	assert(log.format(log.Entry(log.Info, "", [], None)) == "info \"\"")
	Nil
})

// The compiler gate and negative service cases are executable checks, kept out
// of the ordinary passing test suite per std-2 §7. The harness appends each
// case to this owned file and restores it even when a check fails. K4 must make
// all five positive cases pass; until then E0505 records a waiting gate.
// Successful generic clients must also run at both optimization levels.
// Run from the lane with the exact compiler:
// direnv exec /home/agent/Programming/i python3 -c 'from pathlib import Path; exec("\n".join(x[4:] for x in Path("std/test/log_test.i").read_text().splitlines() if x.startswith("//| ")))'
//| import subprocess
//| from pathlib import Path
//| path = Path('std/test/log_test.i')
//| original = path.read_text()
//| positive = {
//|     'annotated logging body (std-2 section 7)': '''
//| GateSink(E, N) { GateSink((log.Level) -> Bool, (log.Entry) -> Nil ? E ! N, (N) -> Nil) }
//| gate_write(E, N) = (event: log.Entry, sink ! GateSink(E, N)) -> Nil ? E ! N | GateSink(E, N) {
//|   GateSink(accept, send, _) = sink
//|   accept(event.level) |> { True -> send(event) False -> Nil }
//| }
//| ''',
//|     'annotated body (review F1/spec 4.7)': '''
//| GateS(N) { GateS(() -> Nil ! N) }
//| gate_use(N) = (s ! GateS(N)) -> Nil ! N | GateS(N) { GateS(f) = s f() }
//| ''',
//|     'generic write client (std-2 section 7)': '''
//| @test
//| gate_client = () {
//|   ! log.Sink(String, Never) = log.make((_: log.Level) { True }, (e) { fail.raise(e.message) })
//|   assert(fail.catch(() { log.write(entry(log.Info)) }) == Error("hello"))
//|   Nil
//| }
//| ''',
//|     'generic lazy client (L3/section 7)': '''
//| @test
//| gate_lazy = () {
//|   ! log.Sink(String, Never) = log.silent()
//|   assert(fail.catch(() { log.lazy(log.Info, () -> String ? String { fail.raise("bad") }) }) == Ok(Nil))
//|   Nil
//| }
//| ''',
//|     'minimal invariant client (review F1)': '''
//| GateS(N) { GateS(() -> Nil ! N, (N) -> Nil) }
//| gate_use(N) = (s ! GateS(N)) { GateS(f, _) = s f() }
//| gate_run(N) = (body: () -> Nil ! N) { body() }
//| @test
//| gate_client = () {
//|   ! GateS(Never) = GateS(() { Nil }, (_: Never) { Nil })
//|   gate_run(() { gate_use() })
//| }
//| ''',
//| }
//| negative = {
//|     'invariant stored environment (L5)': ('''Consumer(A) { Consumer((A) -> Nil) }
//| bad = (s: log.Sink(Never, Consumer(Int))) -> log.Sink(Never, Consumer(Never)) { s }
//| ''', 'E0501'),
//|     'unequal service arguments (L5/spec 9.2)': ('''bad = (s ! log.Sink(String, Never)) -> Bool ! log.Sink(Int, Never) { True }''', 'E0515'),
//| }
//| baseline = subprocess.run(['target/release/i', 'check', '--project', 'std'], capture_output=True, text=True)
//| assert baseline.returncode == 0, baseline.stderr
//| try:
//|     for name, source in positive.items():
//|         path.write_text(original + '\n' + source + '\n')
//|         result = subprocess.run(['target/release/i', 'check', '--project', 'std'], capture_output=True, text=True)
//|         print(name, result.returncode, result.stderr, flush=True)
//|         assert result.returncode == 0 or (result.returncode == 1 and 'error[E0505]' in result.stderr)
//|         print('PASS' if result.returncode == 0 else 'WAITING FOR K4', flush=True)
//|         if result.returncode == 0 and '@test' in source:
//|             for level in ('0', '2'):
//|                 run = subprocess.run(['target/release/i', 'test', '--project', 'std', '--opt', level, 'gate_'], capture_output=True, text=True)
//|                 print(name, 'opt ' + level, run.returncode, run.stdout, run.stderr, flush=True)
//|                 assert run.returncode == 0 and '1 passed, 0 failed' in run.stdout
//|     for name, (source, code) in negative.items():
//|         path.write_text(original + '\n' + source + '\n')
//|         result = subprocess.run(['target/release/i', 'check', '--project', 'std'], capture_output=True, text=True)
//|         print(name, result.returncode, result.stderr, flush=True)
//|         assert result.returncode == 1 and 'error[' + code + ']' in result.stderr
//| finally:
//|     path.write_text(original)
