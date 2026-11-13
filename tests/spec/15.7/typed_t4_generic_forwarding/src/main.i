// typed macro: T4
(
	m = spec/macros
)
f = (x) { m.m(x) }
main = () {
	assert(f(1) == 1)
	assert(f("x") == "x")
}
