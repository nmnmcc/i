(
	testlib = (test) = std/test
	review/rat
)

test("atan_fifth_29", () {
	v = rat.atan_terms(rat.R(1, 5), 29)
	assert(v.n > 0)
})

test("atan_fifth_250", () {
	v = rat.atan_terms(rat.R(1, 5), 250)
	assert(v.n > 0)
})

test("atan_half_65", () {
	v = rat.atan_terms(rat.R(1, 2), 65)
	assert(v.n > 0)
})

test("sine_one_20", () {
	v = rat.sin_terms(rat.R(1, 1), 20)
	assert(v.n > 0)
})
