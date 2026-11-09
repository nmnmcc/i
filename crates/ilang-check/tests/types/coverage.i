// §7.4: the clauses of a `case` must cover every value of the type of the
// subject. A missing pattern is shown.
Shape { Circle(radius: Float) Square(side: Float) Point }
Flags { Flags(admin: Bool, active: Bool) }

covered = (s: Shape) {
  case s {
    Circle(..) -> 1
    Square(..) -> 2
    Point -> 3
  }
}
missing_ctor = (s: Shape) {
  case s {
    Circle(..) -> 1
    Square(..) -> 2
  }
}
missing_nested = (r: Result(Bool, Nil)) {
  case r {
    Ok(True) -> 1
    Error(_) -> 2
  }
}
missing_member = (v: Int | String) {
  case v {
    _: Int if True -> 1
    _: String -> 2
  }
}
missing_label = (f: Flags) {
  case f {
    Flags(admin = True, ..) -> 1
    Flags(active = True, ..) -> 2
  }
}
// Without annotations, the type comes from the patterns.
inferred = (r) {
  case r {
    Ok(1) -> 1
    Error(_) -> 2
  }
}
// A guard is not counted.
guarded = (b: Bool) {
  case b {
    True -> 1
    False if True -> 2
  }
}
// Literal patterns never cover their type.
ints = (n) {
  case n {
    0 -> 0
    1 -> 1
  }
}
floats = (x) {
  case x {
    0.0 -> 0
  }
}
strings = (s) {
  case s {
    "a" -> 0
    "b" -> 1
  }
}
prefixes = (s) {
  case s {
    "a" <> r -> r
    "b" -> "b"
  }
}
// `"" <> r` matches every string.
all_strings = (s) {
  case s {
    "a" <> r -> r
    "" <> r -> r
  }
}
bit_arrays = (b) {
  case b {
    <<x>> -> x
    <<>> -> 0
  }
}
all_bits = (b) {
  case b {
    <<rest:bits>> -> 1
  }
}
// Lists: the empty list, and a cons of an element and the rest.
no_empty = (xs) {
  case xs {
    [x, ..] -> x
  }
}
no_long = (xs) {
  case xs {
    [] -> 0
    [x] -> x
  }
}
lists = (xs) {
  case xs {
    [] -> 0
    [x] -> x
    [x, y, ..] -> x + y
  }
}
list_items = (xs: List(Bool)) {
  case xs {
    [] -> 0
    [True, ..] -> 1
  }
}
// Tuples.
tuples = (p: (Bool, Bool)) {
  case p {
    (True, _) -> 1
    (_, True) -> 2
  }
}
// Types with no values are covered.
only_ok = (r: Result(Int, Never)) {
  case r {
    Ok(x) -> x
  }
}
empty = case [] { [] -> 0 }
// The type test makes `a` an `a & Int`, which it covers.
any = (x: a) {
  case x {
    _: Int -> 1
  }
}
// A type that comes from a function that is not known.
unknown = (f) {
  case f() {
    True -> 1
    False -> 2
  }
}
// The values of the subject are those that come to it: `Error` has none.
no_error = () {
  case Ok(1) {
    Ok(x) -> x
  }
}
