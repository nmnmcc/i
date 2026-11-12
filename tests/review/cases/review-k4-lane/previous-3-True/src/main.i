Dependency = (value = Int)

get = () <- (d: Dependency) { d.value }

Inner(N) = (() <- N -> Int, (N) -> Nil)

read = <N>() <- (s: Inner(N)) {
	Inner(f, _) = s
	f()
}

Outer(N) = (() <- N -> Int, (N) -> Nil)

use = <N>() <- (s: Outer(N)) {
	Outer(f, _) = s
	f()
}

run = <A, N>(body: () <- N -> A) { body() }

example = () { run(() { use() }) }

main = () { example() }
