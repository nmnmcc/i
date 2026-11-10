f = () {
	a = a
	b = [b]
	c = g((n) { c(n) })
	d = (1, () { d })
	unknown_fn(1)
}

g = (h) { h }

uses_param = (x, y) { x + y + z }

clause = (v) {
	v |> {
		(p, q) ? p > q -> p
		(p, _)         -> q
	}
}

t: Unknown = 1

u: m.T = 1
