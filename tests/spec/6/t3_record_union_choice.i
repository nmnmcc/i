// spec: 4.9.3
// expect: compile-error E0504
select(A, B) = (x) -> (a: A) | (a: B) { x }
bad = select((a: 1))
