Consumer(A) = ((A) -> Nil)
Sink(E, N) = (() <- (Fail(E), ..N) -> Nil, (N) -> Nil)
bad = (s : Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }
