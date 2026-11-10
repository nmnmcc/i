S(N) {
	S(() -> Nil ! N)
}

use(N) = (s ! S(N)) {
	S(f) = s
	f()
}

example(N) = () {
	local = () { use() }
	local()
}

@main
main = () { example() }
