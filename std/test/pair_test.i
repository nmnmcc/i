(
	testlib = (test) = std/test
	i/core
	pair = std/pair
)

test("first_int_test", () {
	assert(pair.first((1, 2)) == 1)
	Nil
})

test("first_string_test", () {
	assert(pair.first(("abc", [])) == "abc")
	Nil
})

test("second_int_test", () {
	assert(pair.second((1, 2)) == 2)
	Nil
})

test("second_list_test", () {
	assert(pair.second(("abc", [])) == [])
	Nil
})

test("swap_test", () {
	assert(pair.swap((1, "2")) == ("2", 1))
	Nil
})

test("map_first_basic_test", () {
	inc = (a) { a + 1 }
	assert(pair.map_first((1, 2), inc) == (2, 2))
	Nil
})

test("map_first_larger_test", () {
	inc = (a) { a + 1 }
	assert(pair.map_first((8, 2), inc) == (9, 2))
	Nil
})

test("map_first_zero_test", () {
	inc = (a) { a + 1 }
	assert(pair.map_first((0, -2), inc) == (1, -2))
	Nil
})

test("map_first_negative_test", () {
	inc = (a) { a + 1 }
	assert(pair.map_first((-10, 20), inc) == (-9, 20))
	Nil
})

test("map_second_basic_test", () {
	dec = (a) { a - 1 }
	assert(pair.map_second((1, 2), dec) == (1, 1))
	Nil
})

test("map_second_larger_test", () {
	dec = (a) { a - 1 }
	assert(pair.map_second((8, 2), dec) == (8, 1))
	Nil
})

test("map_second_negative_test", () {
	dec = (a) { a - 1 }
	assert(pair.map_second((0, -2), dec) == (0, -3))
	Nil
})

test("map_second_mixed_test", () {
	dec = (a) { a - 1 }
	assert(pair.map_second((-10, 20), dec) == (-10, 19))
	Nil
})

test("new_test", () {
	assert(pair.new(1, 2) == (1, 2))
	Nil
})
