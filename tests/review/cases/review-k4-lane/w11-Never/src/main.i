S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N), svc: S(N)) -> Int ! N {
	! S(N) = svc
	body()
}

@main
main = () { assert(wrap(() { use() }, S(() { 1 }, (_: Never) { Nil })) == 1) }
