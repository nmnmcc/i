// typed macro
(
	m = spec/macros
)
main = () {
	assert(
		m.m(m.m(42)) == 42,
	)
}
