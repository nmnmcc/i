S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap(N) = (svc: S(N)) -> Int ! N {
	! S(N) = svc
	use()
}


main = () { assert(wrap(S(f: () { 1 })) == 1) }
