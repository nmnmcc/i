S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N)) {
	! S(N) = S(() { 1 }, (_: N) { Nil })
	body()
}

@main
main = () { assert(wrap(() { use() }) == 1) }
