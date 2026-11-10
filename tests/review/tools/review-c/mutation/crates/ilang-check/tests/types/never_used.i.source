// §6.12 and §7.4: a clause that can never be used gives a warning.
after_all = (b: Bool) {
  case b {
    True -> 1
    False -> 2
    _ -> 3
  }
}
duplicate = (n) {
  case n {
    1 -> 1
    1 -> 2
    _ -> 3
  }
}
after_catch_all = (x) {
  case x {
    _ -> 1
    0 -> 2
  }
}
// A clause with a guard may be never used, but it never makes a later one
// never used.
guarded_after = (b: Bool) {
  case b {
    _ -> 1
    True if True -> 2
  }
}
guard_not_counted = (b: Bool) {
  case b {
    True if True -> 1
    True -> 2
    False -> 3
  }
}
prefixes = (s) {
  case s {
    "a" <> _ -> 1
    "ab" -> 2
    "ab" <> _ -> 3
    _ -> 4
  }
}
// The type of the subject has no `String`.
no_values = (v: Int) {
  case v {
    _: Int -> 1
    _: String -> 2
  }
}
choices = (r) {
  case r {
    Ok(_) | Error(_) -> 1
    Ok(1) -> 2
  }
}
nested = (r: Result(Bool, Nil)) {
  case r {
    Ok(True) -> 1
    Ok(False) -> 2
    Error(_) -> 3
    Ok(_) -> 4
  }
}
lists = (xs) {
  case xs {
    [] -> 0
    [_, ..] -> 1
    [x] -> x
  }
}
// Bit array patterns are never known to match the values of another one.
bits = (b) {
  case b {
    <<x>> -> x
    <<x>> -> x
    _ -> 0
  }
}
// A catch-all in a clause that is never used has no values (rule 12).
dead = (x) {
  case x {
    _ -> 1
    y -> y
  }
}
