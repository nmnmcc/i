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

wrap = (body, svc) {
	! S(_) = svc
	body()
}


main = () {
	! Dep = Dep(7)
	assert(wrap(() { use() }, S(() { get() })) == 7)
}
