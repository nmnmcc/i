export(Scope, Task, Scheduler, Runner, go, local_scope)
import_dummy = Nil
Scope(e) = (witness = (e) -> e)
Task(a, e) = (value = (a) -> a, error = (e) -> e)
Scheduler = ()
Runner = ()
go = (s: Scope(e), body: () <- (Fail(e)) -> a) -> Task(a, e) {
	panic("type-only stub")
}
local_scope = (body: (Scope(e)) <- (Fail(e)) -> a) -> a {
	panic("type-only stub")
}
