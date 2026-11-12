// spec: 2.2
// expect: compile-error E0301
// test-macro-migrate: reject
(
	main
)

@test()
main = () { Nil }
