Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N), svc: S(N)) {
	! S(N) = svc
	body()
}


main = () {
	! Dep = Dep(7)
	assert(wrap(() { use() }, S(() { get() })) == 7)
}
