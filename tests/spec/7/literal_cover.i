// spec: 7.4, 4.8
// expect: pass

// §5.3/§5.5: written finite item sets cover shared places without narrowing them.
(
	testlib = (test) = std/test
)

Pair(A) = (A, A)

list = (xs : List("a" | "b")) {
	xs |> ([]) { 0 } | (["a", ..]) { 1 } | (["b", ..]) { 2 }
}

pair = (p : Pair("a" | "b")) {
	p |>
		(Pair("a", "a")) { 1 }
		| (Pair("a", "b")) { 2 }
		| (Pair("b", "a")) { 3 }
		| (Pair("b", "b")) { 4 }
}

complement = (s : String) { s |> ("a") { 0 } | (x : String - "a") { 1 } }

test("closed_sets", () {
	assert(list([]) == 0)
	assert(list(["a", "b"]) == 1)
	assert(list(["b", "a"]) == 2)
	assert(pair(Pair("a", "a")) == 1)
	assert(pair(Pair("a", "b")) == 2)
	assert(pair(Pair("b", "a")) == 3)
	assert(pair(Pair("b", "b")) == 4)
	assert(complement("a") == 0)
	assert(complement("b") == 1)
})
