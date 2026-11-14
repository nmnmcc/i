// spec: 4.2, 13
// expect: compile-error E0501
(
	core = i/core
)
f = (x : core.String) -> core.Int { x }
main = () { Nil }
