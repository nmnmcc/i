Cell(A) = (get = () -> A, set = (A) -> ())
cell = <A>(x : A) { Cell(get = () { x }, set = (y : A) { () }) }
put = <A>(c : Cell(A), x : A) { c.set(x) }
(a, b) = (cell(1), cell("s"))
First = a
Second = b
later = () { put(b, 1) }
