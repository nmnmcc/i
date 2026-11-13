// spec: 7.3
// expect: pass

// literal-types §5.3: deferred fitted failure in its own effect frame.
(
	testlib = (test) = std/test
)

pair = (
	(1, 2),
	() {
		(a, b) = pair.0
		a
	},
)

use = (f: () -> Int) -> Int { f() }

test("total", () { assert(use(pair.1) == 1) })
