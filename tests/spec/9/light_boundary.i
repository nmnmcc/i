// spec: 9.3, 9.4, 9.5
// expect: pass
(
	testlib = (test) = std/test
	i/control
	i/core
	i/failure
	i/resource
)

// Light boundaries (syntax-3 §2.10.7, O30): a boundary whose handle is only
// given to `control.escape`, as in `failure.catch`. Each one has its own
// identity, so an escape goes to its own boundary only; captures for another
// boundary go through it, and finals run on the way.

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

the_fail(E) = (f ! Fail(E)) { f }

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

// A light boundary written by hand: the handle is used inside a nested
// function literal, and also through `|>`.
first_over = (xs: List(Int), limit: Int) -> Int {
	control.boundary((b) {
		each = (ys: List(Int)) -> Int {
			ys |> {
				[] -> -1
				[y, ..rest] -> {
					{ y > limit } |> {
						True  -> b |> control.escape(y)
						False -> each(rest)
					}
				}
			}
		}
		each(xs)
	})
}

test("an escape to a hand-written light boundary", () {
	check(show(first_over([1, 5, 9, 2], 4)), "5")
	check(show(first_over([1, 2], 4)), "-1")
})

test("an inner catch of another E does not take the outer error", () {
	r: Result(Result(Nil, String), Int) = failure.catch(() {
		outer: Fail(Int) = the_fail()
		inner: Result(Nil, String) = failure.catch(() {
			outer.raise(7)
		})
		inner
	})
	check(show(r), "Error(7)")
})

test("an inner catch of the same E does not take the outer error", () {
	r: Result(Result(Nil, Int), Int) = failure.catch(() {
		outer: Fail(Int) = the_fail()
		failure.catch(() {
			outer.raise(1)
		})
	})
	check(show(r), "Error(1)")
	s: Result(Result(Nil, Int), Int) = failure.catch(() {
		failure.catch(() { failure.raise(2) })
	})
	check(show(s), "Ok(Error(2))")
})

test("a catch inside an ensure", () {
	r: Result(Nil, Int) = resource.ensure(() {
		failure.catch(() { failure.raise(1) })
	}, () { Nil })
	check(show(r), "Error(1)")
})

test("a catch inside the final function of an ensure", () {
	r = failure.catch(() {
		resource.ensure(() { failure.raise(1) }, () {
			inner: Result(Nil, Int) = failure.catch(() { failure.raise(2) })
			check(show(inner), "Error(2)")
		})
	})
	check(show(r), "Error(1)")
})

test("a generator inside a catch: captures go through it", () {
	r: Result(List(Int), Int) = failure.catch(() {
		collect(() {
			emit(1)
			emit(2)
		})
	})
	check(show(r), "Ok([1, 2])")
	s: Result(List(Int), Int) = failure.catch(() {
		collect(() {
			emit(1)
			failure.raise(3)
		})
	})
	check(show(s), "Error(3)")
})

test("a catch inside a generator is active again after resume", () {
	xs = collect(() {
		r: Result(Nil, Int) = failure.catch(() {
			emit(1)
			emit(2)
			failure.raise(9)
		})
		r |> {
			Ok(_)    -> emit(0)
			Error(e) -> emit(e)
		}
	})
	check(show(xs), "[1, 2, 9]")
})

test("many catches in a row", () {
	loop = (i: Int, acc: Int) -> Int {
		i |> {
			0 -> acc
			_ -> {
				r = failure.catch(() {
					{ i % 2 == 0 } |> {
						True  -> failure.raise(i)
						False -> i
					}
				}) |> {
					Ok(v)    -> v
					Error(e) -> e * 10
				}
				loop(i - 1, acc + r)
			}
		}
	}
	check(show(loop(4, 0)), "64")
})
