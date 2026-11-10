// spec: 9.2, 6.10, 6.11, 4.8
// expect: pass
(
	i/core
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

// The example of §9.2.
Clock {
	Clock(now: () -> Int)
}

elapsed = (start: Int, clock ! Clock) { clock.now() - start }

fixed = () {
	! Clock = Clock(now: () { 42 })
	elapsed(40)
}

Name {
	Name(text: String)
}

name = (n ! Name) { n.text }

// Two services in one environment, and an environment parameter used
// with a parameter pattern.
both = ((a, b), clock ! Clock, n ! Name) { (a + b + clock.now(), n.text) }

// A closure made in one place gets its services from the place where it is
// called (§9.2), not from where it was made.
made_here = () {
	! Name = Name("maker")
	() { name() }
}

call_elsewhere = (f) {
	! Name = Name("caller")
	f()
}

// An environment statement changes the environment of the rest of its
// block only.
scoped = () {
	! Name = Name("outer")
	inner = {
		! Name = Name("inner")
		name()
	}
	(inner, name())
}

// A later environment statement for the same type constructor takes the
// place of an earlier one.
shadowed = () {
	! Name = Name("first")
	! Name = Name("second")
	name()
}

// The value of an environment statement uses the environment from outside
// the statement (§4.8 rule 9).
from_outside = () {
	! Name = Name("a")
	! Name = Name(name() <> "b")
	! Name = Name(name() <> "c")
	name()
}

// An environment statement does not change which calls are in tail
// position (§11.6). A million calls do not fit on the stack (32 MiB in the
// Wasmtime host).
count = (n, acc, name) {
	! Name = name
	n |> {
		0 -> acc
		_ -> count(n - 1, acc + 1, name)
	}
}

// The environment of a tail call through a value.
deep = (n, k ! Name) {
	n |> {
		0 -> k.text
		1 -> {
			! Name = Name("last")
			apply(deep, 0)
		}
		_ -> {
			! Name = k
			apply(deep, n - 1)
		}
	}
}

apply = (f, n) { f(n) }

// A service is a normal value: it may be stored and given.
stored = (clock ! Clock) { clock }

// A generic service.
State(S) {
	State(get: () -> S)
}

read(S) = (st ! State(S)) { st.get() }

@test
example = () {
	check(show(fixed()), "2")
	! Clock = Clock(now: () { 100 })
	check(show(elapsed(1)), "99")
	check(show(fixed()), "2")
	check(show(elapsed(1)), "99")
}

@test
parameters = () {
	! Clock = Clock(now: () { 1 })
	! Name = Name("n")
	check(show(both((2, 3))), "(6, \"n\")")
	c = stored()
	check(show(c.now()), "1")
}

@test
dynamic_scoping = () {
	f = made_here()
	check(call_elsewhere(f), "caller")
	! Name = Name("here")
	check(f(), "here")
	check(call_elsewhere(() { name() }), "caller")
}

@test
blocks = () {
	check(show(scoped()), "(\"inner\", \"outer\")")
	check(shadowed(), "second")
	check(from_outside(), "abc")
}

@test
tail_calls = () {
	check(show(count(1000000, 0, Name("count"))), "1000000")
	! Name = Name("start")
	check(deep(100000), "last")
	check(deep(0), "start")
}

@test
generic = () {
	! State(Int) = State(() { 7 })
	check(show(read() + 1), "8")
}
