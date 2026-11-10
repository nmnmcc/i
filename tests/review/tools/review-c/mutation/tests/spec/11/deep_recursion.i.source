// spec: 11.6, 11.7
// expect: pass
import (i/core)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

// Ordinary recursion that is not in tail position, 100000 calls deep and
// more.
depth = (n) { case n { 0 -> 0  _ -> 1 + depth(n - 1) } }

@test
deep = () {
  check(core.inspect(depth(100000)), "100000")
  check(core.inspect(depth(300000)), "300000")
}
