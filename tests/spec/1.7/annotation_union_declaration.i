// spec: 1.7, 2.5, 3, 10.3
// expect: compile-error E0318
// The declaration parses; its union result is not a foreign boundary type.
@external("test:math/v1", "add")
f : (Int) -> { Int | String }
