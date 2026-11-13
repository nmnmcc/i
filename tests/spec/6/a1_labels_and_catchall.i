// spec: 4.8, 6.12, 7.1
// expect: pass
(
	main
)
identity = (n) { n }
classify = (0) { "zero" } | (_) { "other" }
guarded = (value: (Int) -> Int) {
	value |> (f: (Int) -> Int ? True) { f(3) } | (_) { 0 }
}

main = () {
	assert(identity("value") == "value")
	assert(identity(12) == 12)
	assert(classify(0) == "zero")
	assert(classify("value") == "other")
	assert(guarded((n) { n + 1 }) == 4)
	Nil
}
