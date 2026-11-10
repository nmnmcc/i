Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N, m: (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N)) -> Int ! N {
	! S(N) = S(() { 1 }, (_: N) { Nil })
	body()
}

@main
main = () {
	! Dep = Dep(7)
	assert(wrap(() { use() }) == 1)
}
