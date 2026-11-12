f = (v: (Int) -> Int) { v |> (g: (Int) -> Int ? True) { g(41) } | (_) { 0 } }
