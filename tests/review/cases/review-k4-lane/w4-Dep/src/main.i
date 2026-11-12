Dep {
	Dep(value: Int)
}

get = (d ! Dep) { d.value }

S(N) {
	S(f: () -> Int ! N)
}

use(N) = (s ! S(N)) {
	S(f = f, ..) = s
	f()
}

wrap(N) = (body: () -> Int ! N | S(N), svc: S(N)) {
	! S(N) = svc
	body()
}


main = () {
	! Dep = Dep(value: 7)
	assert(wrap(() { use() }, S(f: () { get() })) == 7)
}
