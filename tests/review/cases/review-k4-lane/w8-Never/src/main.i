S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (svc) {
	! S(_) = svc
	use()
}


main = () { assert(wrap(S(f: () { 1 })) == 1) }
