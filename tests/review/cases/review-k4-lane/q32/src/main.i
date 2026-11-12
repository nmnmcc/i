S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) -> Int ! N | S(N) { s.f() }

fw(N) = () -> Int ! S(N) | N { use() }


main = () {
	! S(Never) = S(f: () { 1 })
	assert(fw() == 1)
}
