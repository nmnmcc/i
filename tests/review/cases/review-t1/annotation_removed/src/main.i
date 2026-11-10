Cell(A) { Cell(get: () -> A, put: (A) -> Nil) }
mk(A) = (x: A) { Cell(() { x }, (_: A) { Nil }) }
f = () { c = mk(0) c.put(1) c.get() }
@main
main = () { assert(f()==0) Nil }
