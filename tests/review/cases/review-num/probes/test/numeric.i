(
	testlib = (test) = std/test
	i/core
	i/control
	i/resource
)

check = (b: Bool) { b |> (True) { Nil } | (False) { panic("wrong") } }

id = <A>(a: A) -> A { a }

check_bits = (x: Float, n: Int) { check(core.float_to_bits(x) == n) }

test("zeros", () {
	check_bits(id(-.0.0), 9223372036854775808)
	check_bits(-.0.0 +. -.0.0, 9223372036854775808)
	check_bits(-.0.0 +. 0.0, 0)
	check_bits(3.0 /. -.0.0, 0)
	check_bits(core.sqrt(-.0.0), 9223372036854775808)
	check_bits(core.ceiling(-.0.25), 9223372036854775808)
	check(core.hash(-.0.0) == core.hash(0.0))
	check(
		core.inspect(id(-.0.0)) == "-0.0",
	)
})

test("rounding", () {
	x = core.float_from_bits(3)
	check_bits(x *. 0.5, 2)
	check_bits(-.core.float_from_bits(1) *. 0.5, 9223372036854775808)
	x = 10000000000000000.0
	check({ x +. 1.0 } -. x == 0.0)
})

test("integers", () {
	check(id(1073741823) + 1 == 1073741824)
	check(-1073741824 / -1 == 1073741824)
	check(-9223372036854775808 / -1 == 9223372036854775808)
	check(-9223372036854775808 % -1 == 0)
	check(-7 / 3 == -2)
	check(-7 % 3 == -1)
	check(7 / 0 == 0)
	check(7 % 0 == 0)
	check(core.shift_right(-3, 1) == -2)
})

End = (Float, Int, Float)
More = (Continuation(Float, ResultStep, Never, Never))
ResultStep = End | More

work = (b, x: Float, n: Int) -> ResultStep {
	a = control.suspend(b, (k) { More(k) })
	y = a +. 0.25
	c = control.suspend(b, (k) { More(k) })
	End(x, n + 1, y +. c)
}

advance = (s: ResultStep, x: Float) -> ResultStep {
	s |> (More(k)) { control.resume(k, x) } | (other) { other }
}

test("lazy_two_resumes", () {
	s = control.boundary((b) { work(b, -.0.0, 9223372036854775808) })
	t = advance(s, 1.0)
	u = advance(t, 2.0)
	u |>
		(End(x, n, y)) {
			check_bits(x, 9223372036854775808)
			check(n == 9223372036854775809)
			check(y == 3.25)
		}
		| (_) { panic("not done") }
})

make = (x: Float) { () { x } }

Holder = (f = () -> Float, x = Float)

test("identity_and_capture", () {
	f = make(-.0.0)
	g = make(-.0.0)
	p = Holder(f = f, x = 1.0)
	q = Holder(f = f, x = 1.0)
	r = Holder(f = g, x = 1.0)
	check(id(p) == id(q))
	check(id(p) != id(r))
	check_bits(p.f(), 9223372036854775808)
})

relay = (f: (Float) -> Float, n: Int, x: Float) -> Float {
	{ n == 0 } |> (True) { f(x) } | (False) { relay(f, n - 1, x) }
}

test("tail_million", () {
	check_bits(relay(id, 1000000, -.0.0), 9223372036854775808)
})

Bias = (Float)

read = () <- (b: Bias) { b |> (Bias(v)) { v } }

test("retained_environment", () {
	{
		r = control.boundary((b) {
			{
				x = -.0.0
				y = control.suspend(b, (k) {
					control.resume(k, 2.0) <- (Bias(1.0))
				})
				(x, y +. read())
			} <- (Bias(0.5))
		})
		check_bits(r.0, 9223372036854775808)
		check(r.1 == 2.5)
	} <- (Bias(0.25))
})

test("cancel_final_values", () {
	x = -.0.0
	n = 9223372036854775808
	control.boundary((b) {
		resource.ensure(
			() {
				control.suspend(b, (k) { control.cancel(k) })
				panic("cancel continued")
			},
			() {
				check_bits(x, 9223372036854775808)
				check(n + 1 == 9223372036854775809)
			},
		)
	})
	Nil
})
