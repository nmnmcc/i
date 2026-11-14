// spec: 4.9
// expect: pass
(
	i/record
)

(
	main
)
pick = <K>(a : K, b : K) -> (k = Int) {
	r : (k = Int) = record.put((), a, 1)
	record.put(r, b, 2)
}

main = () {
	assert(
		record.get(pick("a", "a"), "a") == 2,
	)
	Nil
}
