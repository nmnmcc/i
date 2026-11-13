// typed macro: 7.5
(
	m = spec/macros
)
main = () {
	f = m.m(42)
	assert(f() == 42)
}
