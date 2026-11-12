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

wrap = (body, svc) {
	! S(_) = svc
	body()
}


main = () {
	! Dep = Dep(value: 7)
	assert(wrap(() { use() }, S(f: () { get() }, m: (_: Dep) { Nil })) == 7)
}
