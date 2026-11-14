// spec: 4.7, 9.4
// expect: compile-error E0225

// A row may be captured or provided, but cannot become ordinary data.
f = <N>() <- (..env: N) { (env) }
