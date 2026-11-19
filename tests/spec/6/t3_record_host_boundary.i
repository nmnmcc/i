// spec: 4.9, 9.7, 10.3
// expect: compile-error E0320
(
	testlib = (test) = i/test
)

@host("test:records/v1")
Records = (get = () -> (name = String))

test("check", () { Nil })
