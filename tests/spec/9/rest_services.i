// spec: 4.8, 9.2, 9.4
// expect: pass
(
    i/fail
)

// syntax-3 §2.10.8: keep N and use Fail from the later caller.
keep = <S, A, E, N>(f: (S) <- (..N, Fail(E)) -> A) <- (..env: N) -> (S) <- (Fail(E)) -> A {
    (x) { f(x) <- env }
}

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

services = () { (Name(text = "data"), Db(value = 31)) }

main = () {
    kept = capture(name) <- (Name(text = "saved"))
    assert({ kept() <- (Name(text = "caller")) } == "saved")
    replaced = override(read, Db(value = 42))
    assert({ replaced() <- (Db(value = 0)) } == 42)
    assert({ name() == "data" } <- services())
    assert({ read() <- (..services(), Db(value = 43)) } == 43)
    raised = fail.catch(() {
        keep((_: Nil) { fail.raise("later") })
    })
    raised |> (Ok(f)) {
        assert(fail.catch(() { f(Nil) }) == Error("later"))
    } | (_) { panic("keep raised at creation") }
}
