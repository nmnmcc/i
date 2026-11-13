Cell(A) = (get = () -> A, set = (A) -> Nil)
cell = <A>(x : A) { Cell(get = () { x }, set = (y : A) { Nil }) }
put = <A>(c : Cell(A), x : A) { c.set(x) }
r = cell(1)
Read = r
later = () { put(r, "s") }
