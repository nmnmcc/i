// spec: 3, 7.1
// expect: compile-error E0101
// old-syntax-check: E0101
// A top-level function in Annot reads Inter for its result too.
f : (Int) -> Int | String = (x) { x }
