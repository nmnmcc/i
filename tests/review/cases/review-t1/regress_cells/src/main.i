Cell(A) = (get = () -> A, put = (A) -> ())
c = Cell(get = () { 0 }, put = (x : Int) { () })
f = () { c.put(1) }
