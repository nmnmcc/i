// spec: 4.8, 6.12, 7.1
// expect: compile-error E0501
Box(A) = (A)
f = (r: Box(Int | String) | Int) { r |> (0 | _: Box(Int)) { Nil } }
