// spec: 6.4, 6.5, 6.7, 4.3
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

User = (name = String, age = Int)

Circle = (radius = Float)
Rect = (width = Float, height = Float)
Square = (width = Float)
Shape = Circle | Rect | Square

Pair(A, B) = (A, B)

Point = (Int, Int, label = String)

test("tuples", () {
	t = (1, "two", (3.0, [4]))
	check(show(t), "(1, \"two\", (3.0, [4]))")
	check(show(t.0), "1")
	check(show(t.1), "\"two\"")
	check(show(t.2.0), "3.0")
	check(show(t.2.1), "[4]")
	check(show(()), "()")
	check(show((1)), "(1)")
	check(show((7)), "(7)")
})

test("lists", () {
	xs = [1, 2, 3]
	check(show(xs), "[1, 2, 3]")
	check(show([0, ..xs]), "[0, 1, 2, 3]")
	check(show([-2, -1, ..[0, ..xs]]), "[-2, -1, 0, 1, 2, 3]")
	check(show([]), "[]")
	check(show([[], [1]]), "[[], [1]]")
	check(show([1, "a"]), "[1, \"a\"]")
})

test("custom", () {
	u = User(name = "ann", age = 31)
	check(show(u), "User(\"ann\", 31)")
	check(u.name, "ann")
	check(show(u.age), "31")
	older = User(..u, age = u.age + 1)
	check(show(older), "User(\"ann\", 32)")
	check(show(u), "User(\"ann\", 31)")
	renamed = User(..u, name = "bob", age = 1)
	check(show(renamed), "User(\"bob\", 1)")
	check(show(Pair(1, "x")), "Pair(1, \"x\")")
	p = Point(1, 2, label = "p")
	check(p.label, "p")
	check(show(Point(..p, label = "q")), "Point(1, 2, \"q\")")
})

Wide = (width = Float, depth = Float)
Thin = (width = Float)
Box = Wide | Thin

test("fields_on_unions", () {
	boxes = [Wide(width = 2.0, depth = 3.0), Thin(width = 4.0)]
	boxes |>
		([r, s]) {
			// `width` is field 0 of both constructors.
			check(show(r.width), "2.0")
			check(show(s.width), "4.0")
		}
		| (_) { panic("no") }
})

Small = (size = Int)
Big = (name = String, size = Int)
Sized = Small | Big

sizes = (xs: List(Sized)) -> List(Int) {
	xs |>
		([]) { [] }
		// `size` is field 0 of `Small` and field 1 of `Big`.
		| ([x, ..rest]) { [x.size, ..sizes(rest)] }
}

test("fields_by_constructor", () {
	check(
		show(
			sizes(
				[Small(size = 1), Big(name = "b", size = 2), Small(size = 3)],
			),
		),
		"[1, 2, 3]",
	)
})

test("known_constructor", () {
	s = Rect(width = 1.0, height = 2.0)
	s |>
		(Rect(..)) { check(show(Rect(..s, height = 9.0)), "Rect(1.0, 9.0)") }
		| (_) { panic("no") }
	Circle(radius = 1.5) |>
		(Circle(..) = c) { check(show(c.radius), "1.5") }
		| (_) { panic("no") }
})
