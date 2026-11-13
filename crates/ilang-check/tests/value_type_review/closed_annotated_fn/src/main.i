f = (n : Int) -> Int { n + 1 }
F = f
g : F = (n : Int) -> Int { n }
v = g(1)
