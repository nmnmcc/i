RowPair(A, B) = A | B
S(A, B) = (() <- A -> Nil, () <- B -> Nil)
use = <A, B>() <- (s : S(A, B)) {
	S(f, g) = s
	f()
	g()
}
wrap = <A, B>() {
	local : () <- (Fail(Never), S(A, B), RowPair(A, B)) -> Nil = () { use() }
	local()
}
