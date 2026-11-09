// §4.7: the shown type is made simpler, and keeps its meaning.
import (i/fail)

// Arguments that do not change.
Ref(a) { Ref(get: () -> a, set: (a) -> Nil) }
read = (r: Ref(a)) { r.get() }
either = (r1: Ref(a), r2: Ref(b), c: Bool) {
  case c {
    True -> r1
    False -> r2
  }
}
make_int = (x) { Ref(() { x }, (y) { y + 1  Nil }) }
make_any = (x) { Ref(() { x }, (y) { Nil }) }

// Written variables on one side only are taken out.
one_side = (x: a) -> Int { x }
errors_only = () -> Int ? e { fail.raise(1) }
empty_list = () {
  y: List(a) = []
  y
}
// Intersections of unions.
inter_unions = (x: a & { Int | String } & { Int | Nil }) { x }
// Only the type constructors of `H` are shown.
rest = (v) {
  case v {
    [] -> 0
    [_, ..] -> 1
    Ok(_) -> 2
    Error(_) -> 3
    other -> other
  }
}

// A binding that is not generalized: each type variable is the union of
// the types that come to it.
id = (x) { x }
mono = id((x) { x })
use_int = () { mono(1) }
use_string = () { mono("a") }

// After a problem, the forms of an environment are shown as they are.
State(s) { State(get: () -> s) }
state_int = (st ! State(Int)) { 1 }
state_str = (st ! State(String)) { 1 }
both = () {
  state_int()
  state_str()
}
