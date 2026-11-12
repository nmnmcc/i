@inline
a = () { Nil }

b = () { Nil }

@external("only")
c: (Int) -> Int

@host("app:h/v1", "sync")
H = (f = () -> Nil)

@deprecated
d = 1

@host("app:h/v1")
e = 1

T = ()

@deprecated("old")
@deprecated("older")
f = 1

g: (Int) -> Int
