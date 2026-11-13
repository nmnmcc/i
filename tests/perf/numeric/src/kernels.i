(
	i/core
	math = numeric/math
)

(
	identity
	fast
	horner
	integer
	dot
	tuples
	closures
)
fast = (x : Float) -> Float { math.fast_sin(x) }
identity = (x : Float) -> Float { x }
horner = (x : Float) -> Float {
	a = 0.125 *. x +. 0.25
	b = a *. x +. 0.375
	c = b *. x +. 0.5
	d = c *. x +. 0.625
	e = d *. x +. 0.75
	f = e *. x +. 0.875
	f *. x +. 1.0
}
int_go = (n : Int, acc : Int) -> Int {
	n |> (0) { acc } | (_) { int_go(n - 1, acc + n) }
}
integer = (x : Float) -> Float {
	core.to_float(
		int_go(128, core.truncate(x)),
	)
}
xs = [
	0.125,
	0.25,
	0.375,
	0.5,
	0.625,
	0.75,
	0.875,
	1.0,
	0.125,
	0.25,
	0.375,
	0.5,
	0.625,
	0.75,
	0.875,
	1.0,
	0.125,
	0.25,
	0.375,
	0.5,
	0.625,
	0.75,
	0.875,
	1.0,
	0.125,
	0.25,
	0.375,
	0.5,
	0.625,
	0.75,
	0.875,
	1.0,
]
dot_go = (as : List(Float), bs : List(Float), sum : Float) -> Float {
	as |>
		([]) { sum }
		| ([a, ..at]) {
			bs |> ([]) { sum } | ([b, ..bt]) { dot_go(at, bt, sum +. a *. b) }
		}
}
dot = (x : Float) -> Float { dot_go(xs, xs, x) }
pair = (x : Float) -> (Float, Float) { (x +. 0.25, x *. 0.5) }
tuples = (x : Float) -> Float {
	(a, b) = pair(x)
	(c, d) = pair(a +. b)
	(e, f) = pair(c +. d)
	e +. f
}
fold = <A, B>(items : List(A), acc : B, f : (B, A) -> B) -> B {
	items |> ([]) { acc } | ([a, ..rest]) { fold(rest, f(acc, a), f) }
}
closures = (x : Float) -> Float {
	fold(xs, 0.0, (acc : Float, v : Float) -> Float { acc +. v *. x })
}
