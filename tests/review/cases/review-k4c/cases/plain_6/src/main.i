wrap = <A, N>(f : () <- N -> A) {
	g : () <- N -> A = () { f() }
	g
}
