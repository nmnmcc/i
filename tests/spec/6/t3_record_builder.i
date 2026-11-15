// spec: 4.9, 14
// expect: pass
(
	i/record
)

(
	main
)
set = <R, K, V>(r : R & Record, key : K, value : V) -> (k = V) {
	record.put(r, key, value)
}

main = () {
	x = set((name = "Ada", age = 1), "age", "old")
	assert(x.name == "Ada")
	assert(x.age == "old")
	Nil
}
