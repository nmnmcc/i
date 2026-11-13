wrap = <A, N>(f: () <- N -> A) {
	g = () { f() }
	g
}
