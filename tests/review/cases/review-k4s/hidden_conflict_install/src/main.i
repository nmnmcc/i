Cell(A) = (read = () -> A, write = (A) -> Nil)
int = () <- (c: Cell(Int)) { c.read() }
text = () <- (c: Cell(String)) { c.read() }
apply = (f) { f() }
both = (a, b) {
	() {
		a()
		b()
	}
}
bad = () {
	both(() { int() }, () { text() })() <- (
		Cell(read = () { 1 }, write = (_: Int) { Nil }),
	)
}
