// spec: 4.5, 9.2, 9.3
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/fail
)

check = (got : String, want : String) { assert(got == want) }

show = (x) { core.inspect(x) }

// Rule A: a need is met by a form that is a subtype of it as a value, and
// two forms of one service in a row are one entry.
need_int = () -> Nil { fail.raise(1) }
need_string = () -> Nil { fail.raise("s") }

Syn3ServiceFail(E) = Fail(E)

both = () <- (f : Syn3ServiceFail(Int | String)) { need_int() }

either = (n : Int) { n |> (0) { need_int() } & (_) { need_string() } }

Yield(A) = (emit = (A) -> Nil)

collect = <A, N>(body : () <- (Fail(Never), Yield(A), ..N) -> Nil) -> List(A) {
	control.boundary((b) {
		{
			body()
			[]
		} <- (
			Yield(
				emit = (x : A) {
					control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
				},
			),
		)
	})
}

emit = <A>(x : A) <- (y : Yield(A)) { y.emit(x) }

test("a wider Fail meets a narrower need", () {
	r : Result(Nil, Int | String) = fail.catch(() { both() })
	check(show(r), "Error(1)")
})

test("two needs of Fail are one entry", () {
	a : Result(Nil, Int | String) = fail.catch(() { either(0) })
	b : Result(Nil, Int | String) = fail.catch(() { either(1) })
	check(show(a), "Error(1)")
	check(show(b), "Error(\"s\")")
})

test("a generator of two types", () {
	xs : List(Int | String) = collect(() {
		emit(1)
		emit("two")
	})
	check(show(xs), "[1, \"two\"]")
})

test("errors of a nested catch", () {
	r = fail.catch(() {
		inner = fail.catch(() { need_int() })
		check(show(inner), "Error(1)")
		need_string()
	})
	check(show(r), "Error(\"s\")")
})
