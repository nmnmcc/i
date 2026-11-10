// spec: 4.7, 4.8, 9.2, 9.3
// expect: pass
(i/fail)

S(N) { S(() -> Nil ! N, (N) -> Nil) }

use(N) = (s ! S(N)) { S(f, _) = s f() }
annotated(N) = (s ! S(N)) -> Nil ! N | S(N) { S(f, _) = s f() }
run(A, E, N) = (body: () -> A ? E ! N) { body() }

// §4.8 rules 3–5 instantiate N with Never; rule 9 removes S(Never).
@test
finite = () {
	! S(Never) = S(() { Nil }, (_: Never) { Nil })
	assert(run(() { use() }) == Nil)
	assert(run(() { annotated() }) == Nil)
}

Dependency { Dependency(value: Int) }
get = (d ! Dependency) { d.value }
Inner(N) { Inner(() -> Int ! N, (N) -> Nil) }
read(N) = (s ! Inner(N)) { Inner(f, _) = s f() }
Outer(N) { Outer(() -> Int ! N, (N) -> Nil) }
read_outer(N) = (s ! Outer(N)) -> Int ! N | Outer(N) { Outer(f, _) = s f() }

// §9.2: callbacks get services at the call site, also with nested rows.
@test
nested = () {
	! Inner(Dependency) = Inner(() { get() }, (_: Dependency) { Nil })
	! Outer(Dependency | Inner(Dependency)) = Outer(() { read() }, (_: Dependency | Inner(Dependency)) { Nil })
	! Dependency = Dependency(7)
	assert(run(() { read_outer() }) == 7)
	! Dependency = Dependency(11)
	assert(run(() { read_outer() }) == 11)
}

// Std-2 §7's logging example, including its original error and marker.
Entry { Entry(message: String) }
Sink(E, N) { Sink((Entry) -> Nil ? E ! N, (N) -> Nil) }

make(E, N) = (send: (Entry) -> Nil ? E ! N) -> Sink(E, N) {
	Sink(send, (_: N) { Nil })
}

write(E, N) = (entry: Entry, sink ! Sink(E, N)) {
	Sink(send, _) = sink
	send(entry)
}

write_annotated(E, N) = (entry: Entry, sink ! Sink(E, N)) -> Nil ? E ! N | Sink(E, N) {
	Sink(send, _) = sink
	send(entry)
}

// §9.3 fail.catch reifies the exact String raised by send. §9.2 supplies
// the installed sink to calls through functions made before installation.
@test
logging = () {
	producer = () { write(Entry("seen")) }
	! Sink(String, Never) = make((entry) { fail.raise(entry.message) })
	assert(fail.catch(producer) == Error("seen"))
	assert(fail.catch(() { write_annotated(Entry("annotated")) }) == Error("annotated"))
}
