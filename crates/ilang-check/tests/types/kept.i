// A1 §3.11: reserved forms parse in full and have checker diagnostics.
value = (p: p.name) { p }
macro_type = (x: m(Int, 1, "s")) { x }
f(1)
@custom
x = 1
@a.main
x2 = 2

x3 = () { Nil }
@external("env" <> "", "f")
ext: () -> Int
@external(module: "env", function: "f")
ext2: () -> Int
@external("env", "f")
bound = 1
no_external(A): (A) -> String
Schema(User, _) = user
Alias(Int) = Int
User { User }
