// §4.5 rules 6 and 7: variance from the definitions.
Box(a) { Box(value: a) }
Sink(a) { Sink(put: (a) -> Nil) }
Cell(a) { Cell(get: () -> a, set: (a) -> Nil) }
Wrap(a) { Wrap(inner: Box(a)) }
Back(a) { Back(sink: Sink(Sink(a))) }

// Covariant: two forms are put together by union.
boxes = [Box(1), Box("a")]
// Contravariant: by intersection.
sinks = (s1: Sink(Int), s2: Sink(String)) { [s1, s2] }
// Through other definitions.
wraps = [Wrap(Box(1)), Wrap(Box("a"))]
backs = (b1: Back(Int), b2: Back(String)) { [b1, b2] }
// Subtyping by variance.
widen = (b: Box(Int)) -> Box(Int | String) { b }
narrow = (s: Sink(Int | String)) -> Sink(Int) { s }
// Invariant: the forms must be equal.
cells = (c1: Cell(Int), c2: Cell(String)) { [c1, c2] }
cell_ok = (c1: Cell(Int), c2: Cell(Int)) { [c1, c2] }
bad_widen = (c: Cell(Int)) -> Cell(Int | String) { c }
// List is covariant (rule 7).
lists = (a: List(Int)) -> List(Int | String) { a }
