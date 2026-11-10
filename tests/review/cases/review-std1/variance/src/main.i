Marker(n) { Marker((n) -> Nil) }
Invariant(n) { Invariant((n) -> n) }
Thunk(n) { Thunk(() -> Nil ! n, (n) -> Nil) }
a = () -> Marker(n) { Marker((_: n) { Nil }) }
b = () -> Invariant(n) { Invariant((x: n) { x }) }
c = () -> Thunk(n) { Thunk(() -> Nil ! n { Nil }, (_: n) { Nil }) }
d: () -> Thunk(n) = () { Thunk(() -> Nil ! n { Nil }, (_: n) { Nil }) }
Sink(a) { Sink(put: (a) -> Nil) }
coerce = (x: Thunk(Sink(Int))) -> Thunk(Sink(Never)) { x }
