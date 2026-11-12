// typed macro: 7.2
(
	m = spec/macros
)
main = () {
	assert(m.m(1, 2) == 3)
	assert({ 1 |> m.m(2) } == 3)
}
