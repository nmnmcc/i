// spec: 11.7
// expect: stop "panic: deep"
// at: spec/main:14:49
// at: spec/main:21:13
// at: spec/main:30:4
// at: spec/main:37:2
(
	i/core
)

// `é` counts as one column.
inner = (s: String) {
	x = core.byte_size("éé")
	{ x > 100 } |> (True) { s } | (False) { "é" <> panic("deep") }
}

// The call of `inner` is a tail call: it is not in the message.
middle = (s) { inner(s) }

outer = (s) {
	r = "é" <> middle(s)
	r
}

each = (xs, f) {
	xs |>
		([]) { Nil }
		| ([x, ..rest]) {
			// A call through a value.
			f(x)
			// A tail call: `each` is not repeated in the message.
			each(rest, f)
		}
}

main = () {
	each(["a", "b"], (s) { outer(s) })
	Nil
}
