// spec: 4.2, 4.4, 9.7, 10.3
// expect: compile-error E0320

// Adding an included value does not fill the missing zero (§10.3).
@host("test:kv/v1")
Kv = (check = (Int) <- (Fail(Nil | Int - 0 | 1 | String)) -> Int)
