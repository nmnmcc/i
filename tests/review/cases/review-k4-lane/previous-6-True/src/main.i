S(E, N) {
	S(() -> Nil ? E ! N)
}

use(E, N) = (s ! S(E, N)) {
	S(f) = s
	f()
}

run(E, N) = (body: () -> Nil ? E ! N) { body() }

example(E, N) = () { run(() { use() }) }

@main
main = () { example() }
