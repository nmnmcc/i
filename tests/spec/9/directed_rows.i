// spec: 4.5, 9.2, 9.3
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/failure
)

check = (got: String, want: String) {
	assert(got == want)
}

show = (x) { core.inspect(x) }

// Rule A: a need is met by a form that is a subtype of it as a value, and
// two forms of one service in a row are one entry.

need_int = () -> Nil ! Fail(Int) { failure.raise(1) }
need_string = () -> Nil ! Fail(String) { failure.raise("s") }

both = (f ! Fail(Int | String)) { need_int() }

either = (n: Int) {
	n |> {
		0 -> need_int()
		_ -> need_string()
	}
}

Yield(A) {
	Yield(emit: (value: A) -> Nil)
}

collect(A, N) = (body: () -> Nil ! Yield(A) | N) -> List(A) ! N {
	control.boundary((b) {
		! Yield(A) = Yield((x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		body()
		[]
	})
}

emit(A) = (x: A, y ! Yield(A)) { y.emit(x) }

test("a wider Fail meets a narrower need", () {
	r: Result(Nil, Int | String) = failure.catch(() { both() })
	check(show(r), "Error(1)")
})

test("two needs of Fail are one entry", () {
	a: Result(Nil, Int | String) = failure.catch(() { either(0) })
	b: Result(Nil, Int | String) = failure.catch(() { either(1) })
	check(show(a), "Error(1)")
	check(show(b), "Error(\"s\")")
})

test("a generator of two types", () {
	xs: List(Int | String) = collect(() {
		emit(1)
		emit("two")
	})
	check(show(xs), "[1, \"two\"]")
})

test("errors of a nested catch", () {
	r = failure.catch(() {
		inner = failure.catch(() { need_int() })
		check(show(inner), "Error(1)")
		need_string()
	})
	check(show(r), "Error(\"s\")")
})
