Cell(A) = (read = () -> A, write = (A) -> Nil)
int = () <- (c : Cell(Int)) { c.read() }
text = () <- (c : Cell(String)) { c.read() }
apply = (f) { f() }
both = (a, b) {
	() {
		a()
		b()
	}
}
bad = () {
	_syn3_0 : Cell(Int) = Cell(read = () { 1 }, write = (_ : Int) { Nil })
	both(() { int() }, () { text() })() <- (_syn3_0)
}
