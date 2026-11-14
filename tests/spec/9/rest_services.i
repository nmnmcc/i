// spec: 4.8, 9.2, 9.4
// expect: pass
(
	i/control
	i/core
)

// Ordinary Fail and boundary code, independent of the concurrent i/fail port.
raise = <E>(error: E) <- (handler: Fail(E)) -> Never { handler.raise(error) }
catch: <A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E) = (body) {
	control.boundary((b) {
		Ok(body()) <- (
			Fail(raise = (error) { control.escape(b, Error(error)) }),
		)
	})
}

// Exercise the shared library implementation of syntax-3 §2.10.8.
keep = control.keep

Name = (text = String)
Db = (value = Int)
name = () <- (n: Name) { n.text }
read = () <- (db: Db) { db.value }

capture = <A, N>(f: () <- N -> A) <- (..env: N) {
	saved = env
	() { f() <- saved }
}

override = <N>(f: () <- (..N, Db) -> Int, db: Db) <- (..env: N) {
	() { f() <- (db, ..env) }
}

override_spread = <N>(f: () <- (..N, Db) -> Int, db: Db) <- (..env: N) {
	() { f() <- (..env, ..(db)) }
}

capture_inferred = <A, M, N>(f: () <- M -> A) <- (..env: N) {
	() { f() <- env }
}

capture_named = <N>() <- (..env: N) { () { name() <- env } }

services = () { (Name(text = "data"), Db(value = 31)) }

main = () {
	inferred = capture_inferred(name) <- (Name(text = "inferred"))
	named = capture_named() <- (Name(text = "named"))
	assert({ inferred() <- (Name(text = "caller")) } == "inferred")
	assert({ named() <- (Name(text = "caller")) } == "named")
	kept = capture(name) <- (Name(text = "saved"))
	assert({ kept() <- (Name(text = "caller")) } == "saved")
	disjoint = override_spread(read, Db(value = 44))
	assert(disjoint() == 44)
	replaced = override(read, Db(value = 42))
	assert({ replaced() <- (Db(value = 0)) } == 42)
	assert({ name() == "data" } <- services())
	assert({ read() <- (..services(), Db(value = 43)) } == 43)
	portable = catch(() { keep((x: Int) { x }) })
	portable |>
		(Ok(f)) { assert(core.portable(f)) }
		| (_) { panic("keep raised at creation") }
	raised = catch(() { keep((_: Nil) { raise("later") }) })
	raised |>
		(Ok(f)) { assert(catch(() { f(Nil) }) == Error("later")) }
		| (_) { panic("keep raised at creation") }
}
