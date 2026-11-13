S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
T(N) = (() <- N -> Nil, (N) -> Nil)
f = <N>() <- (s: S(N)) -> Nil { g() }
g = <N>() <- (t: T(N)) -> Nil { f() }
