//// Syntax three, stage 11 (docs/design/syntax-3.md §2.6.2 to §2.6.7):
//// selection by known types and by expected types, and direct aliases.
//// `i check --types` prints the lines of
//// `types.txt`, and `i run` the lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
)

Conn = (host = String, port = Int)

Some(A) = (A)
None = ()
Option(A) = Some(A) | None

show =
	(n: Int) -> String { core.inspect(n) }
	| (s: String) -> Int { 7 }
	| (a: Int, b: Int) -> Bool { a == b }
	| (host: String, port = p: Int) -> Conn { Conn(host = host, port = p) }

gcd = (a: Int, 0) -> Int { a } | (a: Int, b: Int) -> Int { gcd(b, a % b) }

/// `None` has a branch because the transition checker reads `Some(A)` and
/// `None` as the constructors of `Option(A)`, which a group must cover.
f = (Some(0)) { "zero" } | (Some(n)) { n } | (None) { "none" }

first = (x: Int) { "a" } | (x: Int | String) { 1 }

guarded = (x: Int ? x > 0) { "a" } | (x: Int | String) { 1 }

/// One branch with a type parameter: each shape group is generalized by
/// itself.
pick = <A>(x: A) { x } | (x: Int, y: Int) { y }

map = <A, B>(xs: List(A), g: (A) -> B) -> List(B) {
	xs |> ([]) { [] } | ([x, ..rest]) { [g(x), ..map(rest, g)] }
}

apply = <A, B>(g: (A) -> B, x: A) -> B { g(x) }

// Selection by the known types of the arguments.
s: String = show(1)

n: Int = show("a")

b: Bool = show(1, 2)

c: Conn = show("a.com", port = 80)

d: Int = gcd(12, 18)

e: Int = gcd(5, 0)

x: String | Int = f(Some(5))

a1: String = first(1)

a2 = guarded(1)

// A direct alias stays precise.
g = show

t: String = g(1)

// No known type: the whole group.
h = (x) { show(x) }

// Selection by an expected type.
shown = map([1, 2], show)

applied = apply(show, 1)

p1: String = pick("a")

p2: Int = pick(1, 2)

main = () <- (io: Io) {
	io.print(s)
	io.print(core.inspect(n))
	io.print(core.inspect(b))
	io.print(core.inspect(d))
	io.print(core.inspect(x))
	io.print(t)
	io.print(core.inspect(shown))
	io.print(applied)
	io.print(
		core.inspect(h("b")),
	)
	io.print(p1)
	io.print(core.inspect((c.port, e, a1, a2, p2)))
}
