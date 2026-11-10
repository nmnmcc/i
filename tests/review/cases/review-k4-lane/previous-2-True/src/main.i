Inner(A, E, N) {
	Inner((A) -> A ? E ! N, (N) -> Nil)
}

Outer(A, E, N) {
	Outer(Inner(A, E, N))
}

use(A, E, N) = (x: A, s ! Outer(A, E, N)) {
	Outer(Inner(f, _)) = s
	f(x)
}

run(A, E, N) = (body: () -> A ? E ! N) { body() }

example = () { run(() { use(1) }) }

@main
main = () { example() }
