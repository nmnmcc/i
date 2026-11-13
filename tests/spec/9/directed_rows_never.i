// spec: 4.5, 9.2
// expect: compile-error E0515

// Rule A: a provider `Yield(Never)` does not meet a need `Yield(Int)`.
Yield(A) = (emit = (A) -> Nil)

emit = <A>(x: A) <- (y: Yield(A)) { y.emit(x) }

f = () { emit(1) <- (Yield(emit = (x) { x })) }
