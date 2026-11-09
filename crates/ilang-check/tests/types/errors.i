import (i/fail)

// The kinds of type errors, each with its code.
id = (x) { x }
ids = id(id)
// The value restriction: `ids` has one type for all its uses.
n: Int = ids(1)
s: String = ids("a")

// E0501: mismatch, with the place of the other side.
add = (x: Int) { x + 1 }
bad_arg = () { add("a") }
bad_op = () { 1 + "a" }
bad_guard = (x) {
  case x {
    y if 1 -> y
    _ -> 2
  }
}
// E0502: the number of parameters.
arity = () { add(1, 2) }
arity_var = (f) { (f(1), f(1, 2)) }
// E0504: a value that could go to two type variables.
ambiguous = (x: a | b) { x }
use_ambiguous = () { ambiguous(1) }
// E0508: fields need a known type for now.
field = (u) { u.name }
part = (t) { t.0 }
// E0512: a field that not every constructor has.
Shape { Circle(radius: Float) Square(side: Float) }
radius = (s: Shape) { s.radius }
// E0509: a top-level binding may not give errors or need an environment.
Clock { Clock(now: () -> Int) }
now = (c ! Clock) { c.now() }
top_env = now()
top_err = fail.raise(1)
(top_a, Ok(top_fail)) = (1, Ok(1))
// E0514: bit array formats.
bits = <<1:u8, 2:u3, 3:i16be, 4.0:f64be, "a":utf8, 5:u(4)>>
bad_bits = <<1:u16, 2:x8>>

