Inner(A, E, N) = ((A) <- (Fail(E), ..N) -> A, (N) -> Nil)

Outer(A, E, N) = (Inner(A, E, N))

use = <A, E, N>(x : A) <- (s : Outer(A, E, N)) {
	Outer(Inner(f, _)) = s
	f(x)
}

run = <A, N>(body : () <- N -> A) { body() }

example = () { run(() { use(1) }) }

main = () { example() }
