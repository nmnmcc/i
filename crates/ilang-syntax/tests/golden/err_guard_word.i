// old-syntax-check: E0111,E0101,E0101,E0101
a = v |> { x if c -> 1 }
b = v |> { 0 -> 1  x if c -> 2 }
c = case v { x ? c -> 1 }
