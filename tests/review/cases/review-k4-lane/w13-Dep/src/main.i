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

wrap = (body, svc) {
	! S(_) = svc
	body()
}


main = () {
	! Dep = Dep(7)
	assert(wrap(() { use() }, S(() { get() }, (_: Dep) { Nil })) == 7)
}
