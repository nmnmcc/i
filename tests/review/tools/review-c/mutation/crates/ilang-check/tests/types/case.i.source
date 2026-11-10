// Rule 11: case by places.
// No catch-all: the value must be one the patterns take.
digit = (x) {
  case x {
    0 -> "zero"
    1 -> "one"
  }
}
// A catch-all at the top place: any value may come.
digit_or = (x) {
  case x {
    0 -> "zero"
    _ -> "other"
  }
}
// Constructors share their type arguments at one place.
either = (r) {
  case r {
    Ok(v) -> v
    Error(e) -> e
  }
}
// A catch-all below a constructor: slack only there.
inner = (r) {
  case r {
    Ok(1) -> "one"
    Ok(_) -> "other"
    Error(e) -> e
  }
}
// Tuples and lists.
swap = (t) {
  case t {
    (a, b) -> (b, a)
  }
}
sum = (xs) {
  case xs {
    [] -> 0
    [x, ..rest] -> x + sum(rest)
  }
}
// A guard is a Bool, and a guarded catch-all gives no slack.
sign = (n) {
  case n {
    0 -> 0
    m if m > 0 -> 1
    _ -> -1
  }
}
guarded_only = (x) {
  case x {
    1 -> 1
    y if y > 2 -> y
  }
}
// Type tests.
describe = (v) {
  case v {
    _: Int -> "int"
    s: String -> s
  }
}
// Choices bind the same names.
choice = (r) {
  case r {
    Ok(v) | Error(v) -> v
  }
}
// The clause expressions share one type.
mixed = (b) {
  case b {
    True -> 1
    False -> "no"
  }
}
// Named patterns.
named = (r) {
  case r {
    whole = Ok(_) -> whole
    Error(_) -> Ok(0)
  }
}
// A type variable in a type test is a compile error.
bad = (v) {
  case v {
    x: a -> x
  }
}
// A catch-all next to a pattern whose parts are tested.
first_one = (p) {
  case p {
    (1, x) -> x
    _ -> 0
  }
}
