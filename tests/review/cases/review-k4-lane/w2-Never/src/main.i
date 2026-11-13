S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N)) {
	! S(N) = S(f: () { 1 })
	body()
}


main = () { assert(wrap(() { use() }) == 1) }
