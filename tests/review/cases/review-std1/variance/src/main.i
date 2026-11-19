Marker(n) = ((n) -> ())
Invariant(n) = ((n) -> n)
Thunk(n) = (() -> (), (n) -> ())
a = () -> Marker(n) { Marker((_ : n) { () }) }
b = () -> Invariant(n) { Invariant((x : n) { x }) }
c = () -> Thunk(n) { Thunk(() -> () { () }, (_ : n) { () }) }
d : () -> Thunk(n) = () { Thunk(() -> () { () }, (_ : n) { () }) }
Sink(a) = (put = (a) -> ())
coerce = (x : Thunk(Sink(Int))) -> Thunk(Sink(Never)) { x }
// old-syntax-check: E0101,E0101,E0101,E0101
