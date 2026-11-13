// spec: 4.5, 9.2
// expect: pass

// Rule 4: the caller's Yield(Int | String) is a value subtype of the
// Yield(Int) that the function needs.
Yield(A) = (emit = (A) -> Nil)

Narrow = () <- (Yield(Int)) -> Nil

Wide = () <- (Yield(Int | String)) -> Nil

fit = (f: Narrow) -> Wide { f }

need_int = () <- (y: Yield(Int)) { y.emit(1) }

main = () { fit(need_int)() <- (Yield(emit = (x: Int | String) { Nil })) }
