S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f : () <- N -> A) { f() }
T(N) = (() <- N -> Nil, (N) -> Nil)
f : <N>() <- (Fail(Never), ..N) -> Nil = () <- (s : S(N)) -> Nil { g() }
g : <N>() <- (Fail(Never), ..N) -> Nil = () <- (t : T(N)) -> Nil { f() }
