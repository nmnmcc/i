// spec: 10.1
// expect: compile-error E0208
// host-boundary §3.2, §8 case 5: the accessor `make_svc` is synthesized in
// the type's module; a user definition with that name is defined twice.
(
	Svc(..)
	main
)

@import("app:x/v1")
Svc = (ping = () -> Int)

make_svc = () { 1 }

main = () { () }
