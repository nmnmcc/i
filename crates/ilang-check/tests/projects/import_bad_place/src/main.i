(
	Service(..)
	Twice(..)
	Both(..)
	Hidden
	Alias
	Js
	value
)

@import("app:a/v1")
value = 1

@import("app:a/v1", "f", "now")
Service = (f = () -> Int)

@import("app:b/v1")
Hidden = (f = () -> Int)

@import("app:c/v1")
Alias = Int

@import("js")
Js = (f = () -> Int)

@import("ilang:core")
Core = (f = () -> Int)

@import("app:d/v1", "soon")
Soon = (f = () -> Int)

@import("app:e/v1")
one : () -> Int

@import("app:e/v1", "two")
two : () -> Int

@import("js", "two", "now")
three : () -> Int

@import("js", "globalThis.document")
document : () -> Int

@import()
four : () -> Int

@import("app:f/v1")
Twice = (f = () -> Int)

make_twice = 1

@import("app:g/v1")
@host("app:g/v1")
Both = (f = () -> Int)

@runtime("ilang:core", "not")
not : (Bool) -> Bool
