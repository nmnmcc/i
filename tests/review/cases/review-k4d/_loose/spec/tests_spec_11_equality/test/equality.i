// spec: 11.4
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

yes = (b: Bool) { check(core.inspect(b), "True") }

no = (b: Bool) { check(core.inspect(b), "False") }

User = (name = String, age = Int)

Some(A) = (A)
None = ()
Option(A) = Some(A) | None

top = (x) { x }

alias = top

make = () { (x) { x } }

test("values", () {
	yes(1 == 1)
	no(1 == 2)
	yes(100000000000000000000 == 100000000000000000000)
	no(100000000000000000000 == 100000000000000000001)
	yes(1 != 2)
	yes(1.5 == 1.5)
	yes(0.0 == -.0.0)
	no(0.1 +. 0.2 == 0.3)
	yes("é" == "é")
	no("a" == "A")
	yes(<<1:u1>> == <<1:u1>>)
	no(<<1:u1>> == <<1:u2>>)
	yes(<<1, 2>> == <<1, 2>>)
	yes(core.codepoints("a") == core.codepoints("a"))
})

test("structures", () {
	yes((1, "a") == (1, "a"))
	no((1, "a") == (1, "b"))
	yes([1, 2, 3] == [1, 2, 3])
	no([1, 2] == [1, 2, 3])
	no([1] == [])
	yes([] == [])
	yes(User(name = "a", age = 1) == User(name = "a", age = 1))
	no(User(name = "a", age = 1) == User(name = "a", age = 2))
	yes(Some([1]) == Some([1]))
	no(Some(1) == None)
	yes(None == None)
	no(Ok(1) == Error(1))
	yes(Ok(Nil) == Ok(Nil))
	yes(True == True)
	no(True == False)
	yes(Nil == Nil)
})

mixed = (x: Int | String | Nil) { x == Nil }

test("different_types", () {
	yes(mixed(Nil))
	no(mixed(1))
	no(mixed(""))
})

test("functions", () {
	// A top-level function is one value (§2.5).
	yes(top == top)
	yes(alias == top)
	f = (x) { x + 1 }
	yes(f == f)
	g = f
	yes(g == f)
	// Two evaluations of one function expression are two values.
	no(make() == make())
	h = (x) { x + 1 }
	no(f == h)
	// A constructor, a built-in function, and a library function are each one value.
	yes(Some == Some)
	yes(User == User)
	yes(Ok == Ok)
	yes(panic == panic)
	yes(core.inspect == core.inspect)
	no(core.not == core.inspect)
	yes([top, f] == [top, f])
	no((f, 1) == (h, 1))
})
