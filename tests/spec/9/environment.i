// spec: 9.2, 6.10, 6.11, 4.8
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

// The example of §9.2.
Clock = (now = () -> Int)

elapsed = (start : Int) <- (clock : Clock) { clock.now() - start }

fixed = () { elapsed(40) <- (Clock(now = () { 42 })) }

Name = (text = String)

name = () <- (n : Name) { n.text }

// Two services in one environment, and an environment parameter used
// with a parameter pattern.
both = ((a, b)) <- (clock : Clock, n : Name) { (a + b + clock.now(), n.text) }

// A closure made in one place gets its services from the place where it is
// called (§9.2), not from where it was made.
made_here = () { () { name() } <- (Name(text = "maker")) }

call_elsewhere = (f) { f() <- (Name(text = "caller")) }

// An environment statement changes the environment of the rest of its
// block only.
scoped = () {
	{
		inner = { name() <- (Name(text = "inner")) }
		(inner, name())
	} <- (Name(text = "outer"))
}

// A later environment statement for the same type constructor takes the
// place of an earlier one.
shadowed = () { name() <- (Name(text = "first"), Name(text = "second")) }

// The value of an environment statement uses the environment from outside
// the statement (§4.8 rule 9).
from_outside = () {
	name() <- (Name(text = name() <> "c")) <- (Name(text = name() <> "b")) <- (
		Name(text = "a"),
	)
}

// An environment statement does not change which calls are in tail
// position (§11.6). A million calls do not fit on the stack (32 MiB in the
// Wasmtime host).
count = (n, acc, name : Name) {
	n |> (0) { acc } | (_) { count(n - 1, acc + 1, name) } <- (name)
}

// The environment of a tail call through a value.
deep = (n) <- (k : Name) {
	n |>
		(0) { k.text }
		| (1) { apply(deep, 0) <- (Name(text = "last")) }
		| (_) { apply(deep, n - 1) <- (k) }
}

apply = (f, n) { f(n) }

// A service is a normal value: it may be stored and given.
stored = () <- (clock : Clock) { clock }

// A generic service.
State(S) = (get = () -> S)

read = <S>() <- (st : State(S)) { st.get() }

test("example", () {
	check(
		show(fixed()),
		"2",
	)
	{
		check(
			show(elapsed(1)),
			"99",
		)
		check(
			show(fixed()),
			"2",
		)
		check(
			show(elapsed(1)),
			"99",
		)
	} <- (Clock(now = () { 100 }))
})

test("parameters", () {
	{
		check(
			show(both((2, 3))),
			"(6, \"n\")",
		)
		c = stored()
		check(
			show(c.now()),
			"1",
		)
	} <- (Clock(now = () { 1 }), Name(text = "n"))
})

test("dynamic_scoping", () {
	f = made_here()
	check(call_elsewhere(f), "caller")
	{
		check(f(), "here")
		check(call_elsewhere(() { name() }), "caller")
	} <- (Name(text = "here"))
})

test("blocks", () {
	check(
		show(scoped()),
		"(\"inner\", \"outer\")",
	)
	check(shadowed(), "second")
	check(from_outside(), "abc")
})

test("tail_calls", () {
	check(
		show(count(1000000, 0, Name(text = "count"))),
		"1000000",
	)
	{
		check(deep(100000), "last")
		check(deep(0), "start")
	} <- (Name(text = "start"))
})

test("generic", () {
	check(
		show(read() + 1),
		"8",
	) <- (State(get = () { 7 }))
})
