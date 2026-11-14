Cell(A) = (get = () -> A, put = (A) -> Nil)
c = Cell(get = () { 0 }, put = (x : Int) { Nil })
f = () { c.put(1) }
