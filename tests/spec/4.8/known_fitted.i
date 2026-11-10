// spec: 4.8
// expect: pass

// Decision 7: rule 13 uses whole scalar bounds and fresh noncovariant holes.
Box(A) { Box(value: A) }
f = (s: String) {
	x = "a" = s
	t = (x, 0)
	t.0
}
g = (s: String) {
	x = "a" = s
	Box(x).value
}
h = (xs: List(String)) { xs |> { [x = "a", ..] -> (x, 0).0  _ -> "" } }
kind = { "a" -> 1  "b" -> 2 }
apply(A, B) = (x: A, f: (A) -> B) { (f, f(x)) }

@test
fitted = () {
	assert(f("a") == "a")
	assert(g("a") == "a")
	assert(h(["a"]) == "a")
	assert((kind, 0).1 == 0)
	p = apply("a", { "a" -> 1  "b" -> 2 })
	assert(p.1 == 1)
}
