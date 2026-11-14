Marker(n) = ((n) -> Nil)
Invariant(n) = ((n) -> n)
Thunk(n) = (() -> Nil, (n) -> Nil)
a = () -> Marker(n) { Marker((_ : n) { Nil }) }
b = () -> Invariant(n) { Invariant((x : n) { x }) }
c = () -> Thunk(n) { Thunk(() -> Nil { Nil }, (_ : n) { Nil }) }
d : () -> Thunk(n) = () { Thunk(() -> Nil { Nil }, (_ : n) { Nil }) }
Sink(a) = (put = (a) -> Nil)
coerce = (x : Thunk(Sink(Int))) -> Thunk(Sink(Never)) { x }
// old-syntax-check: E0101,E0101,E0101,E0101
