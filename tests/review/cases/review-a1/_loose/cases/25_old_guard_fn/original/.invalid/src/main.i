// old-syntax-check: E0114,E0101
f = (v: (Int) -> Int) { case v { g: (Int) -> Int if True -> g(41) _ -> 0 } }
