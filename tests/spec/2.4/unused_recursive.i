// spec: 2.4.1; references from unused bindings count.
// expect: warning W0504
(
	testlib = (test) = std/test
)

even = (n: Int) { n == 0 || odd(n - 1) }

odd = (n: Int) { n != 0 && even(n - 1) }

helper = () { assert(True) }

test("check", () { helper() })

unused = 1

_quiet = 1
