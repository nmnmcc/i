// spec: 3, 7.1
// expect: compile-error E0101
// Alternatives are not allowed inside a data-pattern item.
Box = (value = Int | String)
f = (Box(value = x : Int | String)) { x }
