// spec: 2.5
// expect: stop "panic: y"
// at: spec/order_go:14:19
// at: spec/order_go:11:5
(
	i/core
)

// The example of §2.5: the order is `y`, `z`, `a`. The first binding whose
// needs are all worked out comes next, so `y` comes before `z`, although
// `a`, which needs `z`, is first in the source.
a = (1, z)

y = p("y")

z = p("z")

p = (s: String) { panic(s) }

@main
main = () { core.inspect(a) }
