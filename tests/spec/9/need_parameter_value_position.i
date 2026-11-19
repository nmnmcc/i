// spec: 4.7, 9.4
// expect: pass
// Decision 1 (EF01/EF09) removes exclusive ordinary/row classification.
Holder(R, N) = (boundary = Boundary(R, N))
forward = <R, N>(b : Boundary(R, N), body : () <- N -> R) <- (..env : N) {
	body() <- env
}
unused_value = <N>(x : N) <- (..env : N) { () }
main = () { () }
