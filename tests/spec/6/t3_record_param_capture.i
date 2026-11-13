// spec: 4.9
// expect: pass
(
	i/record
)

(
	main
)
f = <K>(r: (k = Int), key: K) -> (k = Int) {
	h = () { r }
	h()
}

main = () {
	assert(
		record.get(f((name = 9), "name"), "name") == 9,
	)
	Nil
}
