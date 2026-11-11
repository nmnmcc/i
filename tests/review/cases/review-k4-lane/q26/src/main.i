S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f) = s
	f()
}

fw(N) = () -> Int ! N | S(N) { use() }


main = () {
	! S(Never) = S(() { 1 })
	assert(fw() == 1)
}
