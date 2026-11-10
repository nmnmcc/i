Handle {}
H = Handle
Id(x) = x
Wrap { Wrap(Int) }

// Types the host can take (§10.3), also through aliases.
@external("a", "ok1")
ok1: (Int, Float, String, BitArray, Nil, Bool) -> Nil
@external("a", "ok2")
ok2: (h: H, n: Id(Int)) -> Id(Handle)

// Not a function type.
@external("a", "f5")
f5: Int

// `_` and type variables.
@external("a", "f2")
f2: (_) -> Int
@external("a", "f3")
f3: (a) -> a
@external("a", "f7")
f7: (List(_)) -> Int

// Types the host cannot take.
@external("a", "f1")
f1: (List(Int)) -> Int
@external("a", "f6")
f6: ((Int) -> Int) -> Result(Int, Nil)
@external("a", "f8")
f8: (Wrap, UtfCodepoint, Never) -> Id(List(Int))
@external("a", "f9")
f9: (Int | String, (Int, Int)) -> Int

// No errors and no environment.
@external("a", "f4")
f4: () -> Int ? String
@external("a", "f10")
f10: () -> Int ? Int ! Assert
