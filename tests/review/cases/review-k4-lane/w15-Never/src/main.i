S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (svc: S(N)) {
	! S(N) = svc
	use()
}

@main
main = () { assert(wrap(S(() { 1 }, (_: Never) { Nil })) == 1) }
