// spec: 7.4
// expect: compile-error E0516
Red = ()

Green = ()
Color = Red | Green
name = (c : Color) { c |> (Red) { "red" } | (Green ? False) { "green" } }
