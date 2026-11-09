// spec: 6.10, 9.6
// expect: compile-error E0701
// A binding that may fail needs the `Assert` service, which comes later.

@test
binding = () {
  r: Result(Int, String) = Ok(1)
  Ok(n) = r
  case n { 1 -> Nil  _ -> panic("no") }
}
