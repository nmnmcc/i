// typed macro: 7.2
(m = spec/macros)
main = () { assert(m.m(b: 2, a: 1) == 3) assert({ 1 |> m.m(b: 2) } == 3) }
