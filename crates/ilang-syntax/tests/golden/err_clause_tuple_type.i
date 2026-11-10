// old-syntax-check: E0114,E0101
// §7.1: `p: (Int, Int) -> e` is read as a function type.
f = p |> { p: (Int, Int) -> e }
