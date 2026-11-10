// Rule 14 and §4.7: a binding with a direct value is generalized; any
// other binding is not (the value restriction).
id = (x) { x }
both = (id(1), id("a"))
ids = id(id)
use_ids = () { (ids(1), ids("a")) }

// Local bindings follow the same rule.
local = () {
  f = (x) { x }
  (f(1), f("a"))
}
local_mono = () {
  g = id(id)
  (g(1), g("a"))
}

// Parameters are never generalized (§4.7).
param = (f) { (f(1), f("a")) }

// Inside its own value, a name is not general.
fact = (n) {
  case n {
    0 -> 1
    _ -> n * fact(n - 1)
  }
}
loop = (x) { loop(x) }

// A group is typed together and generalized together.
(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
f = (x) { g(x) }
g = (x) { f(x) }
uses = (even(1), f(1), f("a"))
