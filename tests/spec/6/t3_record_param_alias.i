// spec: 4.9
// expect: pass
(
	i/record
)

(
	main
)
f = <K>(r : (k = Int), key : K = other) -> Int {
	record.get(r, key) + record.get(r, other)
}

main = () {
	assert(f((a = 3), "a") == 6)
	Nil
}
