S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) { s.f() }

fw = () { use() }

@main
main = () {
	! S(Never) = S(() { 1 })
	assert(fw() == 1)
}
