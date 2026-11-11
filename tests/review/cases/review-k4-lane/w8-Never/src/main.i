S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap = (svc) {
	! S(_) = svc
	use()
}

@main
main = () { assert(wrap(S(() { 1 })) == 1) }
