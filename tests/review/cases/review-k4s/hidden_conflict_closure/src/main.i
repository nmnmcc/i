Cell(A) { Cell(read: () -> A, write: (A) -> Nil) }
int = (c ! Cell(Int)) { c.read() }
text = (c ! Cell(String)) { c.read() }
apply = (f) { f() }
both = (a, b) { () { a() b() } }
bad = () { both(() { int() }, () { text() })() }
