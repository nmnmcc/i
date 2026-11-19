// spec: 4.5, 9.2
// expect: compile-error E0515

// Rule 4: a caller providing only Yield(Int) cannot run a function that
// needs Yield(Int | String).
Yield(A) = (emit = (A) -> ())
Narrow = () <- (Yield(Int)) -> ()
Wide = () <- (Yield(Int | String)) -> ()

bad = (f : Wide) -> Narrow { f }
