// spec: 9.9
// expect: pass
// runtime.md 6.1: exact public higher-order schemes; the inferred display's
// intersections N & (D | (N - D)) impose no additional needs on the body.
(
	i/task
	i/random
	i/request
)

go(A, E, N): (task.Scope(E), () -> A ^ E ! N) -> task.Task(
	A
	E
) ! task.Scheduler | N - task.Scheduler - task.Runner - request.Await = task.go

keep(S, A, E, N): ((S) -> A ^ E ! N) -> { (
	S
) -> A ^ E ! task.Scheduler } ! task.Scheduler | N - task.Scheduler - task.Runner - request.Await = task.keep

run(A, E, N): (
	() -> A ^ E ! N
) -> A ^ E ! task.Runner | N - task.Scheduler - task.Runner - request.Await - request.Owner - request.Root = task.run

test(A, E, N): (
	random.Generator
	List(List(Int))
	() -> A ^ E ! N
) -> A ^ E | task.ScheduleError ! task.Runner | N - task.Scheduler - task.Runner - request.Await - request.Owner - request.Root = task.test

both(A, B, E, N): (() -> A ^ E ! N, () -> B ^ E ! N) -> (
	A
	B
) ^ E ! task.Scheduler | N - task.Scheduler - task.Runner - request.Await = task.both

race(A, E, N): (
	() -> A ^ E ! N
	() -> A ^ E ! N
) -> A ^ E ! task.Scheduler | N - task.Scheduler - task.Runner - request.Await = task.race

map_concurrently(A, B, E, N): (List(A), (A) -> B ^ E ! N) -> List(
	B
) ^ E ! task.Scheduler | N - task.Scheduler - task.Runner - request.Await = task.map_concurrently


main = () { Nil }
