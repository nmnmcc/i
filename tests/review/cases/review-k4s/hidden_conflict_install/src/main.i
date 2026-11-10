Cell(A) { Cell(read: () -> A, write: (A) -> Nil) }
int = (c ! Cell(Int)) { c.read() }
text = (c ! Cell(String)) { c.read() }
apply = (f) { f() }
both = (a, b) { () { a() b() } }
bad = () {
 ! Cell(Int) = Cell(read: () { 1 }, write: (_: Int) { Nil })
 both(() { int() }, () { text() })()
}
