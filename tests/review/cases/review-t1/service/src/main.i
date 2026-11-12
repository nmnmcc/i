State(A) = (get = () -> A)
cur = <A>(seed: A) <- (st: State(A)) { st.get() }

main = () {
	{
		assert(cur(0) == 5)
		Nil
	} <- (State(get = () { 5 }))
}
