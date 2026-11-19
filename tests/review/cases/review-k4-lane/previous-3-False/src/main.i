Dependency = (value = Int)

get = () <- (d : Dependency) { d.value }

Inner(N) = (() <- N -> Int, (N) -> ())

read = <N>() <- (s : Inner(N)) {
	Inner(f, _) = s
	f()
}

Outer(N) = (() <- N -> Int, (N) -> ())

use = <N>() <- (s : Outer(N)) -> Int {
	Outer(f, _) = s
	f()
}

run = <A, N>(body : () <- N -> A) { body() }

example = () { run(() { use() }) }

main = () { example() }
