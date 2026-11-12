S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap = (body, svc) {
	! S(_) = svc
	body()
}


main = () { assert(wrap(() { use() }, S(f: () { 1 }, m: (_: Never) { Nil })) == 1) }
