Cell(A) = (get = () -> A, set = (A) -> ())
cell = <A>(x : A) { Cell(get = () { x }, set = (y : A) { () }) }
put = <A>(c : Cell(A), x : A) { c.set(x) }
r = cell(1)
Read = r
later = () { put(r, "s") }
