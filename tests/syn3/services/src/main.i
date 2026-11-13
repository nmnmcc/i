//// Syntax three, stage 8 (docs/design/syntax-3.md §2.10): services in the
//// head, needs in types, `Fail` in a need, a row variable, and provides on a
//// block and on an expression. Read with `I_SYNTAX=3`; `i run` prints the
//// lines of `expected.txt`.
(
	(Io) = i/io
	fail = i/fail
	core = i/core
)

User = (name = String, age = Int)

NotFound = (id = String)

Db = (find = (String) <- (Fail(NotFound)) -> User)

Log = (write = (String) -> Nil)

Base = (value = Int)

find_user = (id: String) <- (db: Db, log: Log) -> User {
	log.write(id)
	db.find(id)
}

map = <A, B, N>(xs: List(A), f: (A) <- N -> B) -> List(B) {
	xs |> ([]) { [] } | ([x, ..rest]) { [f(x), ..map(rest, f)] }
}

/// The closed form of `catch` over the library function: the body needs
/// `Fail(E)` and whatever `N` stands for.
attempt = <A, E, N>(body: () <- (..N, Fail(E)) -> A) -> Result(A, E) {
	fail.catch(body)
}

must = (id: String) -> String {
	id |> ("") { fail.raise(NotFound(id = "empty")) } | (_) { id }
}

offset = (x: Int) <- (base: Base) -> Int { base.value + x }

show = (r: Result(String, NotFound)) -> String {
	r |>
		(Ok(name)) { "ok \(name)" }
		| (Error(NotFound(id = id))) { "not found \(id)" }
}

sum = (xs: List(Int)) -> Int {
	xs |> ([]) { 0 } | ([x, ..rest]) { x + sum(rest) }
}

fake_db = Db(
	find = (id) {
		id |>
			("1") { User(name = "ada", age = 36) }
			| (_) { fail.raise(NotFound(id = id)) }
	},
)

quiet_log = Log(write = (text) { Nil })

lookup = (id: String) -> Result(String, NotFound) {
	attempt(() {
		{
			u = find_user(must(id))
			u.name
		} <- (fake_db, quiet_log)
	})
}

main = () <- (io: Io) {
	io.print(
		show(lookup("1")),
	)
	io.print(
		show(lookup("2")),
	)
	io.print(
		show(lookup("")),
	)
	// A provide on an arbitrary expression: `<-` is the weakest level.
	n = 1 + offset(2) <- (Base(value = 10))
	io.print("n \(core.inspect(
	sum(map([n, 2], (x) { x * 2 })),
))")
	// A provide on one call, inside a function that is mapped.
	shifted = map([1, 2], (x) { offset(x) }) <- (Base(value = 100))
	io.print("shifted \(core.inspect(sum(shifted)))")
}
