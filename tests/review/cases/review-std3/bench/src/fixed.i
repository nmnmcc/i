(
	i/core
)

(
	I(..)
	cosine
)
I = (lo = Int, hi = Int)
scale = 340282366920938463463374607431768211456
floor = (a : Int, b : Int) -> Int {
	a / b - { { a < 0 && a % b != 0 } |> (True) { 1 } | (False) { 0 } }
}
ceil = (a : Int, b : Int) -> Int { -floor(-a, b) }
min = (a : Int, b : Int) -> Int { { a < b } |> (True) { a } | (False) { b } }
max = (a : Int, b : Int) -> Int { { a > b } |> (True) { a } | (False) { b } }
add = (a : I, b : I) -> I { I(lo = a.lo + b.lo, hi = a.hi + b.hi) }
neg = (a : I) -> I { I(lo = -a.hi, hi = -a.lo) }
mul = (a : I, b : I) -> I {
	aa = a.lo * b.lo
	ab = a.lo * b.hi
	ba = a.hi * b.lo
	bb = a.hi * b.hi
	I(
		lo = floor(
			min(min(aa, ab), min(ba, bb)),
			scale,
		),
		hi = ceil(
			max(max(aa, ab), max(ba, bb)),
			scale,
		),
	)
}
div = (a : I, b : Int) -> I { I(lo = floor(a.lo, b), hi = ceil(a.hi, b)) }
cosine = (r : I) -> I {
	square = mul(r, r)
	go = (k : Int, term : I, sum : I) -> I {
		{ k == 20 } |>
			(True) { I(lo = sum.lo - 1, hi = sum.hi + 1) }
			| (False) {
				go(
					k + 1,
					div(
						neg(mul(term, square)),
						{ 2 * k + 1 } * { 2 * k + 2 },
					),
					add(sum, term),
				)
			}
	}
	go(0, I(lo = scale, hi = scale), I(lo = 0, hi = 0))
}
