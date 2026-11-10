S(N) {
	S(() -> Nil ! N)
}

use(N) = (s ! S(N)) {
	S(f) = s
	f()
}

example(N) = () {
	local: () -> Nil ! N | S(N) = () { use() }
	local()
}

@main
main = () { example() }
