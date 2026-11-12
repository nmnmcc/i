// spec: 4.8
// expect: pass

// Decision 7: literal parts at noncovariant places become fresh bound holes.
(
	testlib = (test) = std/test
)

Sink(A) {
	Sink(put: (A) -> Nil)
}

Cell(A) {
	Cell(get: () -> A, set: (A) -> Nil)
}

State(A) {
	State(get: () -> A)
}

cofinite = (s: String - "a") { 1 }

read = (c: Cell("a")) { c.get() }

stored = (st ! State("a")) { st.get() }

test("noncovariant", () {
	s = Sink(put: (m: "GET" | "POST") { Nil })
	s.put("GET")
	c = Cell(get: () { "a" }, set: (x: "a") { Nil })
	assert(read(c) == "a")
	assert((cofinite, 0).1 == 0)
	! State("a") = State(get: () { "a" })
	assert((stored, 0).1 == 0)
	assert(stored() == "a")
})
