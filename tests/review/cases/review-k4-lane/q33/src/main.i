S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) -> Int ! N | S(N) { s.f() }

fw(M) = () -> Int ! M | S(M) { use() }


main = () {
	! S(Never) = S(f: () { 1 })
	assert(fw() == 1)
}
