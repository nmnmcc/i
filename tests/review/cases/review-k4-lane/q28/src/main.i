S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f) = s
	f()
}

fw(M) = () -> Int ! M | S(M) { use() }


main = () {
	! S(Never) = S(f: () { 1 })
	assert(fw() == 1)
}
