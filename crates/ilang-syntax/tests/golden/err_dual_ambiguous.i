a = { (x) -> Int { x } }
b = { (x) -> (y) { y } }
c = v |> { (a, b) -> (x) { x + a } }(1)
