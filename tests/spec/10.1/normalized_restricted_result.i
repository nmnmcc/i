// spec: 4.2, 4.4, 10.1, 10.3
// expect: compile-error E0318

// This normalizes to 1, not all of Int (§10.3).
One = Int - {Int - 1}
@external("test:math/v1", "add")
add: (Int, Int) -> One
