// spec: 4.8, 6.12
// expect: pass
(
	testlib = (test) = std/test
)

f = (0 | 1) { 2 }

select = (x: { Int | String }) { x |> (0 = n | n: String) { n } }

test("choices", () {
	assert(f(0) == 2)
	assert(f(1) == 2)
	assert(select(0) == 0)
	assert(select("s") == "s")
})
