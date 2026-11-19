Cell(A) = (() -> A, (A) -> ())
bad = (x) { bad(Cell(() { x }, (_) { () })) }
