(
	main
)
State(A) = (get = () -> A)
cur = <S>(start : S) <- (state : State(S)) -> S { state.get() }
main = () {
	n = cur(0) <- (State(get = () { 5 }))
	assert(n + 1 == 6)
}
