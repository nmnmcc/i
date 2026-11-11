export (Scope, Task, Scheduler, Runner, go, local_scope)
import_dummy = Nil
Scope(e) { Scope(witness: (e) -> e) }
Task(a,e) { Task(value: (a) -> a, error: (e) -> e) }
Scheduler { Scheduler }
Runner { Runner }
go = (s: Scope(e), body: () -> a ^ e ! n) -> Task(a,e) ! Scheduler | n - Scheduler - Runner { panic("type-only stub") }
local_scope = (body: (Scope(e)) -> a ^ e ! n) -> a ^ e ! n | Scheduler { panic("type-only stub") }
