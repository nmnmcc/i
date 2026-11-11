Dependency {
	Dependency(value: Int)
}

get = (d ! Dependency) { d.value }

Inner(N) {
	Inner(() -> Int ! N, (N) -> Nil)
}

read(N) = (s ! Inner(N)) {
	Inner(f, _) = s
	f()
}

Outer(N) {
	Outer(() -> Int ! N, (N) -> Nil)
}

use(N) = (s ! Outer(N)) -> Int ! N | Outer(N) {
	Outer(f, _) = s
	f()
}

run(A, N) = (body: () -> A ! N) { body() }

example = () { run(() { use() }) }


main = () { example() }
