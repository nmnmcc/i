S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N), svc: S(N)) {
	! S(N) = svc
	body()
}


main = () { assert(wrap(() { use() }, S(f: () { 1 })) == 1) }
