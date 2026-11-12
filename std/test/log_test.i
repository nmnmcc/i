(
	testlib = (test) = std/test
	i/control
	i/fail
	i/stdio
	i/request
	std/list
	std/log
	std/time
	(Some, None) = std/option
)

Event {
	Sent(log.Entry)
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

entry = (level: log.Level) -> log.Entry { log.Entry(level: level, message: "hello", fields: [], at: None) }

// L1/L2 and the sequential service law: entry fields and timestamps are data.

test("log_delivery", () {
	original = log.Entry(
		level: log.Info
		message: "hello"
		fields: [("", ""), ("k", "1"), ("k", "2")]
		at: Some(time.from_unix_nanoseconds(-1))
	)
	producer = () { deliver(original) }
	trace = control.boundary((b) {
		! Capture = Capture(record: (event) {
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
		! Capture = Capture(record: (event) {
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
				Sent(log.Entry(level: log.Info, message: "built", fields: [], at: None))
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
			! Capture = Capture(record: (event) {
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
				! Capture = Capture(record: (event) {
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
		! Capture = Capture(record: (event) {
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
		! Capture = Capture(record: (event) {
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
			log.Entry(level: log.Info, message: "a\nb", fields: [("k", "v"), ("k", "")], at: None)
		) ==
			"info \"a\\nb\" \"k\"=\"v\" \"k\"=\"\""
	)
	assert(
		log.format(
			log.Entry(
				level: log.Debug
				message: "\"\\\r\t"
				fields: [("\n", "\u{0}\u{1b}\u{2028}\u{2029}")]
				at: Some(time.from_unix_nanoseconds(-1))
			)
		) ==
			"debug \"\\\"\\\\\\r\\t\" @-1 \"\\n\"=\"\\u{0}\\u{1b}\\u{2028}\\u{2029}\""
	)
	assert(
		log.format(
			log.Entry(
				level: log.Warning
				message: "é漢🙂"
				fields: [("", "\u{7f}\u{80}\u{9f}\u{a0}")]
				at: Some(time.from_unix_nanoseconds(0))
			)
		) ==
			"warning \"é漢🙂\" @0 \"\"=\"\\u{7f}\\u{80}\\u{9f}\u{a0}\""
	)
	assert(log.format(entry(log.Failure)) == "failure \"hello\"")
	assert(log.format(log.Entry(level: log.Info, message: "", fields: [], at: None)) == "info \"\"")
	Nil
})

GateSink(E, N) {
	GateSink((log.Level) -> Bool, (log.Entry) -> Nil ^ E ! N, (N) -> Nil)
}

gate_write(E, N) = (
	event: log.Entry
	sink ! GateSink(E, N)
) -> Nil ^ E ! N | GateSink(E, N) {
	GateSink(accept, send, _) = sink
	accept(event.level) |> {
		True  -> send(event)
		False -> Nil
	}
}

GateS(N) {
	GateS(() -> Nil ! N, (N) -> Nil)
}

gate_use(N) = (s ! GateS(N)) {
	GateS(f, _) = s
	f()
}

gate_run(N) = (body: () -> Nil ! N) { body() }

test("gate_annotated_write", () {
	! GateSink(String, Never) = GateSink(
		(_: log.Level) { True }
		(e) { fail.raise(e.message) }
		(_: Never) { Nil }
	)
	assert(fail.catch(() { gate_write(entry(log.Info)) }) == Error("hello"))
	Nil
})

test("gate_annotated_service", () {
	! GateS(Never) = GateS(() { Nil }, (_: Never) { Nil })
	gate_use()
})

test("gate_generic_write", () {
	! log.Sink(String, Never) = log.make((_: log.Level) { True }, (e) {
		fail.raise(e.message)
	})
	assert(fail.catch(() { log.write(entry(log.Info)) }) == Error("hello"))
	Nil
})

test("gate_generic_lazy", () {
	! log.Sink(String, Never) = log.silent()
	assert(
		fail.catch(() {
			log.lazy(log.Info, () -> String ^ String { fail.raise("bad") })
		}) ==
			Ok(Nil)
	)
	Nil
})

test("gate_invariant_client", () {
	! GateS(Never) = GateS(() { Nil }, (_: Never) { Nil })
	gate_run(() { gate_use() })
})

test("stderr_sink_reports_failures", () {
	! stdio.Stdio = stdio.Stdio(
		read: (_) { request.ready(Ok(<<>>)) }
		write: (_) { request.ready(Ok(Nil)) }
		write_error: (_) { request.ready(Error("failed")) }
		flush: () { request.ready(Ok(Nil)) }
		flush_error: () { request.ready(Ok(Nil)) }
	)
	! log.Sink(
		stdio.StdioError
		stdio.Stdio | request.Await | request.Owner
	) = log.stderr()
	assert(
		fail.catch(() { log.write(entry(log.Info)) }) ==
			Error(stdio.StdioError(operation: "write_error", code: "failed"))
	)
	Nil
})
