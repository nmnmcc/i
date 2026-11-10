// Statements that are not the last and do nothing give W0502 (§6.10).
Pair {
	Pair(Int, Int)
}

f = (x: Int) { x }

warned = (x: Int, y: Float) {
	1
	"text"
	x
	f
	(x) { x }
	(x, [x, ..[]], Pair(x, -x))
	x + 1 == 2 && x > 0
	"a" <> "b"
	y <. 1.0
	Nil
}

// A call, `|>`, `case`, a block, a bit array, and a Float operator, which
// may stop (§11.3), are not warned about.
not_warned = (x: Int, y: Float) {
	f(x)
	x |> f
	x |> {
		_ -> 1
	}
	{
		z = 1
		z
	}
	<<x>>
	y +. 1.0
	-.y
	Nil
}
