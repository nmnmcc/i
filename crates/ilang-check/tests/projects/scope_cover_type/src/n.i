(
	(S) = app/m
)

// A local name covers a module name (§2.3), in types too.
f = (m : Int) {
	y : m.T = m
	y
}

g = (m) {
	{
		h = () <- (s : m.S) { 1 }
		h
	} <- (S(get = () { 1 }))
}

// Not covered before the local is bound: the module.
ok : m.T = 1

ok_fn = (x : m.T) {
	y : m.T = x
	m = 1
	z : m.T = m
	z
}
