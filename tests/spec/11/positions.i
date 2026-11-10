// spec: 11.7
// expect: stop "panic: deep"
// at: spec/positions:13:45
// at: spec/positions:20:14
// at: spec/positions:29:7
// at: spec/positions:38:3

(
	i/core
)

// `é` counts as one column.
inner = (s: String) {
	x = core.byte_size("éé")
	{ x > 100 } |> {
		True  -> s
		False -> "é" <> panic("deep")
	}
}

// The call of `inner` is a tail call: it is not in the message.
middle = (s) { inner(s) }

outer = (s) {
	r = "é" <> middle(s)
	r
}

each = (xs, f) {
	xs |> {
		[] -> Nil
		[x, ..rest] -> {
			// A call through a value.
			f(x)
			// A tail call: `each` is not repeated in the message.
			each(rest, f)
		}
	}
}

@main
main = () {
	each(["a", "b"], (s) { outer(s) })
	Nil
}
