// spec: 4.9
// expect: pass
(
	main
)
f = <K>(r : (k = Int), key : K) -> Int { 1 }

main = () {
	assert(f((name = 9), "name") == 1)
	Nil
}
