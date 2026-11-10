Cell(A) { Cell(get: () -> A, put: (A) -> Nil) }
c = Cell(() { 0 }, (x: Int) { Nil })
f = () { c.put(1) }
