// typed macro: T4
(
	m = spec/macros
)
f = (x: (a = Int, b = String)) { m.m(x).a }
main = () { assert(f((a = 42, b = "x")) == 42) }
