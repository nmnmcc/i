Consumer(A) = ((A) -> ())
Sink(E, N) = (() <- (Fail(E), ..N) -> (), (N) -> ())
bad = (s : Sink(Never, Consumer(Int))) -> Sink(Never, Consumer(Never)) { s }
