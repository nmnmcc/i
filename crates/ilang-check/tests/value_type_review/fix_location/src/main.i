Cell(A) = (get = () -> A, set = (A) -> Nil)
cell = <A>(x: A) { Cell(get = () { x }, set = (y: A) { Nil }) }
put = <A>(c: Cell(A), x: A) { c.set(x) }
(a, b) = (cell(1), cell("s"))
First = a
Second = b
later = () { put(b, 1) }
