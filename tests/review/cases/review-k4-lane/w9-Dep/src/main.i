Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N)) -> Int ! N {
	! S(N) = S(f: () { 1 }, m: (_: N) { Nil })
	body()
}


main = () {
	! Dep = Dep(value: 7)
	assert(wrap(() { use() }) == 1)
}
