// A parameter used at many calls, each with a type constructor that does
// not change in its parameters (§4.5): its upper bounds are checked with
// one another without a constraint for each two of them.
import (i/control)

s = (b, v) { control.suspend(b, (k) { control.resume(k, v) }) }

f = (b, x) {
  a0 = x
  a1 = case a0 % 3 { 0 -> s(b, a0 + 1)  _ -> a0 + 2 }
  a2 = case a1 % 3 { 0 -> s(b, a1 + 1)  _ -> a1 + 2 }
  a3 = case a2 % 3 { 0 -> s(b, a2 + 1)  _ -> a2 + 2 }
  a4 = case a3 % 3 { 0 -> s(b, a3 + 1)  _ -> a3 + 2 }
  a5 = case a4 % 3 { 0 -> s(b, a4 + 1)  _ -> a4 + 2 }
  a6 = case a5 % 3 { 0 -> s(b, a5 + 1)  _ -> a5 + 2 }
  a6
}
main = () { control.boundary((b) { f(b, 0) }) }

// Two upper bounds with one type constructor whose arguments cannot be
// equal: the problem is found once, where the second is given.
take_int = (b: Boundary(Int, Never, Never)) { Nil }
take_string = (b: Boundary(String, Never, Never)) { Nil }
g = (b) {
  take_int(b)
  take_string(b)
  take_int(b)
  take_int(b)
}
