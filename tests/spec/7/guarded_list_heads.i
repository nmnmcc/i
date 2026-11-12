// spec: 4.8, 7.4
// expect: pass

// Std-1b report §5: an unguarded list rest covers every later item place.
(
	testlib = (test) = std/test
)

probe = (xs: List(Int)) -> Nil {
	xs |>
		([]) { Nil }
		| ([37, a, b, ..tail] ? a >= 0 && b >= 0) { Nil }
		| ([37, .._]) { Nil }
		| ([x, ..tail]) { Nil }
}

test("all_paths", () {
	probe([])
	probe([37, 1, 2, 3])
	probe([37, -1, -2])
	probe([37])
	probe([0, 1, 2])
})
