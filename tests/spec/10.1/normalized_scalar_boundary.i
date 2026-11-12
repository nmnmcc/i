// spec: 4.2, 4.4, 10.1, 10.3
// expect: pass

// Equivalent scalar sets use the same Int ABI in both directions (§10.3).
One = { 1 | 2 } - 2
Two = Int & 2
Whole = Int - 0 | 0
@external("test:math/v1", "add")
add: (One, Two) -> Whole

main = () { assert(add(1, 2) == 3) }
