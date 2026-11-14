// spec: 3, 4.3, 4.7
// expect: pass
(
	testlib = (test) = std/test
	std/list
	std/string
	std/int
)

id = (x) { x }

twice = (f : id, n : Int) { f(f(n)) }

port : Int = 8080

Port = port

Pair(A, B) = { (A, B) }

parts : (Int, String) = (1, "a")

Pair(Number, Text) = parts

Composite(A) = { (A, port) }

Codec(D, E) = (decode = (String) -> D, encode = (E) -> String)

make = <D, E>(d : (String) -> D, e : (E) -> String) {
	Codec(decode = d, encode = e)
}

size = id(
	make(
		(s : String) -> Int { string.length(s) },
		(n : Int) -> String { int.to_string(n) },
	),
)

Codec(Size, _) = size

test("annotation_instance", () { assert(twice(id, 3) == 3) })

test("local_sharing", () {
	f = (xs : List(Int)) {
		acc : xs = []
		acc
	}
	assert(f([1]) == [])
})

test("inferred_shape", () {
	copy = (s : String) {
		x = "a" = s
		y : x = "b"
		y
	}
	assert(copy("a") == "b")
	assert(
		list.map(["a"], (x) {
			y : x = "b"
			y == "b"
		}) ==
			[True],
	)
})

test("closed_aliases", () {
	f = (n : Number, s : Text, p : Port) { (n, s, p) }
	assert(f(2, "b", 8081) == (2, "b", 8081))
	g = (value : Composite(String)) { value }
	assert(g(("x", 8081)) == ("x", 8081))
})

test("codec_projection_and_later_encode", () {
	result : Size = size.decode("abcd")
	assert(result == 4)
	assert(size.encode(12) == "12")
})

test_type = (v : { Int | String }) { v |> (n : Port) { n + 1 } | (_) { 0 } }

test("completed_alias_type_tests", () {
	assert(test_type(2) == 3)
	assert(test_type("x") == 0)
})

Box(A) = (A)

test("matched_holder_and_original_subject", () {
	copy = (p : Box(Int)) {
		Box(a) = id(p)
		y : a = "s"
		y == "s"
	}
	assert(copy(Box(1)))
	original = (x) {
		x |>
			(_ : Int) {
				y : x = "s"
				y == "s"
			}
			| (_) { True }
	}
	assert(original(1))
})
