@inline
a = () { Nil }

@main("x")
b = () { Nil }

@external("only")
c: (Int) -> Int

@host("app:h/v1", "sync")
H {
	H(f: () -> Nil)
}

@deprecated
d = 1

@host("app:h/v1")
e = 1

@main
T {
	T
}

@deprecated("old")
@deprecated("older")
f = 1

g: (Int) -> Int
