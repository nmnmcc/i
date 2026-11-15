// spec: 4.8, 6.12
// expect: compile-error E0501
id = <A>(x : A) { x }
f = () { id |> (g | g) { (g(1) + 1, g("a") <> "b") } }
