RowPair(A, B) = A | B
S(A, B) = (() <- A -> (), () <- B -> ())
use = <A, B>() <- (s : S(A, B)) {
	S(f, g) = s
	f()
	g()
}
wrap = <A, B>() {
	local : () <- (Fail(Never), S(A, B), RowPair(A, B)) -> () = () { use() }
	local()
}
