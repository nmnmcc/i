// spec: 4.9
// expect: pass
(
	main
)
f = <K>(r : (k = Int), key : K) -> Int { 1 }
g = <K>(r : (k = Int), key : K) -> Int { f(r, key) }

main = () {
	assert(g((name = 9), "name") == 1)
	Nil
}
