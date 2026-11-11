S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N), svc: S(N)) -> Int ! N {
	! S(N) = svc
	body()
}


main = () { assert(wrap(() { use() }, S(() { 1 })) == 1) }
