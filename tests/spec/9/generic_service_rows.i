// spec: 4.7, 4.8, 9.2, 9.3
// expect: pass
(
	testlib = (test) = std/test
	i/fail
)

S(N) = (() <- N -> Nil, (N) -> Nil)

use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}

annotated = <N>() <- (s: S(N)) -> Nil {
	S(f, _) = s
	f()
}

run = <A, N>(body: () <- N -> A) { body() }

// §4.8 rules 3–5 instantiate N with Never; rule 9 removes S(Never).
test("finite", () {
	{
		assert(run(() { use() }) == Nil)
		assert(run(() { annotated() }) == Nil)
	} <- (S(() { Nil }, (_: Never) { Nil }))
})

Dependency = (value = Int)

get = () <- (d: Dependency) { d.value }

Inner(N) = (() <- N -> Int, (N) -> Nil)

read = <N>() <- (s: Inner(N)) {
	Inner(f, _) = s
	f()
}

Outer(N) = (() <- N -> Int, (N) -> Nil)

read_outer = <N>() <- (s: Outer(N)) -> Int {
	Outer(f, _) = s
	f()
}

// §9.2: callbacks get services at the call site, also with nested rows.
test("nested", () {
	{
		assert(run(() { read_outer() }) == 7)
		assert(run(() { read_outer() }) == 11) <- (Dependency(value = 11))
	} <- (
		Inner(() { get() }, (_: Dependency) { Nil }),
		Outer(() { read() }, (_: { Dependency | Inner(Dependency) }) { Nil }),
		Dependency(value = 7),
	)
})

// Std-2 §7's logging example, including its original error and marker.
Entry = (message = String)

Sink(E, N) = ((Entry) <- (Fail(E), ..N) -> Nil, (N) -> Nil)

make = <E, N>(send: (Entry) <- (Fail(E), ..N) -> Nil) -> Sink(E, N) {
	Sink(send, (_: N) { Nil })
}

write = <E, N>(entry: Entry) <- (sink: Sink(E, N)) {
	Sink(send, _) = sink
	send(entry)
}

write_annotated = <E, N>(entry: Entry) <- (sink: Sink(E, N)) -> Nil {
	Sink(send, _) = sink
	send(entry)
}

// §9.3 fail.catch reifies the exact String raised by send. §9.2 supplies
// the installed sink to calls through functions made before installation.
test("logging", () {
	producer = () { write(Entry(message = "seen")) }
	{
		assert(fail.catch(producer) == Error("seen"))
		assert(
			fail.catch(() { write_annotated(Entry(message = "annotated")) }) ==
				Error("annotated"),
		)
	} <- (make((entry) { fail.raise(entry.message) }))
})
