S(A, B) = (() <- A -> Nil, () <- B -> Nil)
use = <A, B>() <- (s: S(A, B)) {
	S(f, g) = s
	f()
	g()
}
wrap = <A, B>() {
	local: () <- (S(A, B), ..A) -> Nil = () { use() }
	local()
}
