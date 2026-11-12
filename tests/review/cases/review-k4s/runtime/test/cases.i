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
	forward(1) |> (8) { Nil } | (other) { panic("wrong result") } <- (
		Dep(value = 7),
		S((x: Int) { x + get() }, (_: Dep) { Nil }),
	)
})

test("string", () {
	forward("a") |> ("a!") { Nil } | (other) { panic("wrong result") } <- (
		S((x: String) { x <> "!" }, (_: Never) { Nil }),
	)
})

test("error", () {
	fail.catch(() { forward(1) }) |>
		(Error("failed")) { Nil }
		| (other) { panic("wrong result") } <- (
		S((_: Int) { fail.raise("failed") }, (_: Never) { Nil }),
	)
})

test("mutual", () {
	left(True) |> (8) { Nil } | (other) { panic("wrong result") } <- (
		Dep(value = 7),
		S((x: Int) { x + get() }, (_: Dep) { Nil }),
	)
})

make = (x: Int) {
	() {
		run(() { use(1) }) <- (
			Dep(value = x),
			S((n: Int) { n + get() }, (_: Dep) { Nil }),
		)
	}
}

test("installed_in_closure", () {
	a = make(7)
	b = make(9)
	(a(), b()) |> ((8, 10)) { Nil } | (other) { panic("wrong result") }
})
