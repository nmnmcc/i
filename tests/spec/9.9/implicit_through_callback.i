// spec: 9.9
// expect: pass
// A function that uses the implicit Scheduler, passed on as a value, still
// makes the entry need it: through an inferred callback row, and through a
// written row that forwards the caller's environment (`..env : N`).
(
	i/resource
	i/task
)

f = () { task.scope((g) { 1 }) }

apply = (k) { k() }

forward = <N>(k : () <- N -> Int) <- (..env : N) -> Int { k() }

check = (got : Int, want : Int) {
	{ got == want } |>
		(True()) { () }
		& (False()) { panic("wrong value") }
}

main = () {
	check(apply(f), 1)
	check(forward(f) + resource.scoped(f), 2)
}
