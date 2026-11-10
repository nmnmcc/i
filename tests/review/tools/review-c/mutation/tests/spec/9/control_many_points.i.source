// spec: 9.4, 6.12
// expect: pass
import (i/control, i/core)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

// Many calls that may suspend, in the clauses of `case`s with guards and
// in nested `case`s. A program that suspends at each of them and resumes at
// once gives the same values as one that does not suspend.
f = (s: (Int) -> Int, x: Int) {
  a0 = x
  a1 = case a0 % 3 { 0 -> s(a0 + 1)  _ -> a0 + 2 }
  a2 = case a1 % 4 {
    0 if a1 > 10 -> s(a1) + s(1)
    0 -> a1 + 1
    1 -> case s(a1) % 2 { 0 -> s(a1 + 3)  _ -> a1 + 5 }
    _ -> a1 + 7
  }
  a3 = case (a2, a1) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a4 = case a3 % 3 { 0 -> s(a3 + 1)  _ -> a3 + 2 }
  a5 = case a4 % 4 {
    0 if a4 > 10 -> s(a4) + s(1)
    0 -> a4 + 1
    1 -> case s(a4) % 2 { 0 -> s(a4 + 3)  _ -> a4 + 5 }
    _ -> a4 + 7
  }
  a6 = case (a5, a4) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a7 = case a6 % 3 { 0 -> s(a6 + 1)  _ -> a6 + 2 }
  a8 = case a7 % 4 {
    0 if a7 > 10 -> s(a7) + s(1)
    0 -> a7 + 1
    1 -> case s(a7) % 2 { 0 -> s(a7 + 3)  _ -> a7 + 5 }
    _ -> a7 + 7
  }
  a9 = case (a8, a7) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a10 = case a9 % 3 { 0 -> s(a9 + 1)  _ -> a9 + 2 }
  a11 = case a10 % 4 {
    0 if a10 > 10 -> s(a10) + s(1)
    0 -> a10 + 1
    1 -> case s(a10) % 2 { 0 -> s(a10 + 3)  _ -> a10 + 5 }
    _ -> a10 + 7
  }
  a12 = case (a11, a10) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a13 = case a12 % 3 { 0 -> s(a12 + 1)  _ -> a12 + 2 }
  a14 = case a13 % 4 {
    0 if a13 > 10 -> s(a13) + s(1)
    0 -> a13 + 1
    1 -> case s(a13) % 2 { 0 -> s(a13 + 3)  _ -> a13 + 5 }
    _ -> a13 + 7
  }
  a15 = case (a14, a13) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a16 = case a15 % 3 { 0 -> s(a15 + 1)  _ -> a15 + 2 }
  a17 = case a16 % 4 {
    0 if a16 > 10 -> s(a16) + s(1)
    0 -> a16 + 1
    1 -> case s(a16) % 2 { 0 -> s(a16 + 3)  _ -> a16 + 5 }
    _ -> a16 + 7
  }
  a18 = case (a17, a16) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a19 = case a18 % 3 { 0 -> s(a18 + 1)  _ -> a18 + 2 }
  a20 = case a19 % 4 {
    0 if a19 > 10 -> s(a19) + s(1)
    0 -> a19 + 1
    1 -> case s(a19) % 2 { 0 -> s(a19 + 3)  _ -> a19 + 5 }
    _ -> a19 + 7
  }
  a21 = case (a20, a19) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  a22 = case a21 % 3 { 0 -> s(a21 + 1)  _ -> a21 + 2 }
  a23 = case a22 % 4 {
    0 if a22 > 10 -> s(a22) + s(1)
    0 -> a22 + 1
    1 -> case s(a22) % 2 { 0 -> s(a22 + 3)  _ -> a22 + 5 }
    _ -> a22 + 7
  }
  a24 = case (a23, a22) {
    (p, q) if p > q + 5 -> s(p - q) + p
    (p, _) -> case p % 2 { 0 -> p + 1  _ -> s(p) + 2 }
  }
  [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23, a24]
}

suspending = (x) {
  control.boundary((b) {
    f((v) { control.suspend(b, (k) { control.resume(k, v) }) }, x)
  })
}

plain = (x) { f((v) { v }, x) }

@test
many_points = () {
  check(core.inspect(suspending(0)), core.inspect(plain(0)))
  check(core.inspect(suspending(1)), core.inspect(plain(1)))
  check(core.inspect(suspending(2)), core.inspect(plain(2)))
  check(core.inspect(suspending(17)), core.inspect(plain(17)))
}
