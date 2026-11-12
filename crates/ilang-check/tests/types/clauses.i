// A1 §3.9 and §6.2: labels, direct values, fitting, and expected types.
User = (name = String)
Some(A) = (A)
None = ()
Option(A) = Some(A) | None
Done = ()
one = (n) { n }
many = (0) { 1 } | (_) { 2 }
blocks = [(0) { 1 } | (_) { 2 }, (1) { 0 } | (_) { 3 }]
loop = (0) { Done } | (n) { loop(n - 1) }
local = () {
	loop = (0) { Done } | (n) { loop(n - 1) }
	loop(5)
}
mutual = () {
	(even, odd) = (
		(0) { True } | (n) { odd(n - 1) },
		(0) { False } | (n) { even(n - 1) },
	)
	(even(4), odd(5))
}
get = (opt: Option(User)) { opt |> (Some(u)) { u.name } }
map = (opts: List(Option(User))) {
	list_map(opts, (Some(u)) { u.name } | (None) { "" })
}
list_map = <A, B>(xs: List(A), f: (A) -> B) {
	xs |> ([]) { [] } | ([x, ..rest]) { [f(x), ..list_map(rest, f)] }
}
expected = () {
	(u) { u.name } |> (f: (User) -> String) { f(User(name = "bob")) }
}
f = (x) { x + 1 }
subject_scope = () { (x) { f(x) * 2 } |> (f) { f(1) } }
empty = (x: Never) { core.never(x) }
