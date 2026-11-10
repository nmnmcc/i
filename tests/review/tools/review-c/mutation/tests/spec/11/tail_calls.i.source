// spec: 11.6
// expect: pass
import (i/core)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

// A clause of a `case` in tail position.
count = (n, acc) {
  case n {
    0 -> acc
    _ -> count(n - 1, acc + 1)
  }
}

// Mutual recursion between top-level functions.
ping = (n) { case n { 0 -> "ping"  _ -> pong(n - 1) } }
pong = (n) { case n { 0 -> "pong"  _ -> ping(n - 1) } }

// The last expression of a block in tail position, after a binding.
loop_block = (n, acc) {
  next = acc + 2
  case n == 0 {
    True -> acc
    False -> {
      m = n - 1
      loop_block(m, next)
    }
  }
}

// A call through a value: `call_ref` in tail position.
apply = (f, n) { f(n) }
via = (n) {
  case n {
    0 -> 0
    _ -> apply(via, n - 1)
  }
}

// `|>` in tail position is a call.
piped = (n) { case n { 0 -> "done"  _ -> n - 1 |> piped } }

@test
deep = () {
  check(show(count(1000000, 0)), "1000000")
  check(ping(1000001), "pong")
  check(show(loop_block(1000000, 0)), "2000000")
  check(show(via(1000000)), "0")
  check(piped(1000000), "done")
}

@test
local_functions = () {
  step = 3
  go = (n, acc) {
    case n {
      0 -> acc
      _ -> go(n - 1, acc + step)
    }
  }
  check(show(go(1000000, 0)), "3000000")
  (a, b) = ((n) { case n { 0 -> "a"  _ -> b(n - 1) } }, (n) { case n { 0 -> "b"  _ -> a(n - 1) } })
  check(a(1000000), "a")
}
