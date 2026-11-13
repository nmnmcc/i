// spec: 4.8, 6.12, 7.3
// expect: pass
(
	testlib = (test) = std/test
)

f = (n: { 0 | 1 }) -> 2 { n |> (0 | 1) { 2 } }

test("finite_fitting_is_total", () {
	assert(f(0) == 2)
	assert(f(1) == 2)
})
