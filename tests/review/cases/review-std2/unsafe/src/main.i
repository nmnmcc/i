Consumer(A) { Consumer((A) -> Nil) }
Sink(E,N) { Sink(() -> Nil ? E ! N, (N) -> Nil) }
bad = (s: Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }
