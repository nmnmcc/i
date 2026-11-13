(
	testlib = (test) = std/test
	i/fail
)

Dep = (value = Int)

get = () <- (d: Dep) { d.value }

S(A, E, N) = ((A) <- (Fail(E), ..N) -> A, (N) -> Nil)

use = <A, E, N>(x: A) <- (s: S(A, E, N)) {
	S(f, _) = s
	f(x)
}

run = <A, N>(body: () <- N -> A) { body() }

forward = <A, E, N>(x: A) <- (s: S(A, E, N)) { run(() { use(x) }) }

left = <N>(b: Bool) <- (s: S(Int, Never, N)) {
	b |> (True) { right(False) } | (False) { use(1) }
}

right = <N>(b: Bool) <- (s: S(Int, Never, N)) {
	b |> (True) { left(False) } | (False) { use(1) }
}

test("integer", () {
	_syn3_0: S(Int, Never, Dep) = S((x: Int) { x + get() }, (_: Dep) { Nil })
	forward(1) |> (8) { Nil } | (other) { panic("wrong result") } <- (
		Dep(value = 7),
		_syn3_0,
	)
})

test("string", () {
	_syn3_1: S(String, Never, Never) = S(
		(x: String) { x <> "!" },
		(_: Never) { Nil },
	)
	forward("a") |> ("a!") { Nil } | (other) { panic("wrong result") } <- (
		_syn3_1,
	)
})

test("error", () {
	_syn3_2: S(Int, String, Never) = S(
		(_: Int) { fail.raise("failed") },
		(_: Never) { Nil },
	)
	fail.catch(() { forward(1) }) |>
		(Error("failed")) { Nil }
		| (other) { panic("wrong result") } <- (_syn3_2)
})

test("mutual", () {
	_syn3_3: S(Int, Never, Dep) = S((x: Int) { x + get() }, (_: Dep) { Nil })
	left(True) |> (8) { Nil } | (other) { panic("wrong result") } <- (
		Dep(value = 7),
		_syn3_3,
	)
})

make = (x: Int) {
	() {
		_syn3_4: S(Int, Never, Dep) = S(
			(n: Int) { n + get() },
			(_: Dep) { Nil },
		)
		run(() { use(1) }) <- (Dep(value = x), _syn3_4)
	}
}

test("installed_in_closure", () {
	a = make(7)
	b = make(9)
	(a(), b()) |> ((8, 10)) { Nil } | (other) { panic("wrong result") }
})
