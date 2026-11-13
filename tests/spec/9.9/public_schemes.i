// spec: 9.9
// expect: pass
// runtime.md 6.1: exact public higher-order schemes; the inferred display's
// intersections N & (D | (N - D)) impose no additional needs on the body.
(
	i/task
	i/random
	i/request
)

go: <A, E, N>(task.Group(E), () <- (Fail(E), ..N) -> A) <- (
	task.Scheduler,
	..N,
) -> task.Task(A, E) = task.go

keep: <S, A, E, N>(
	(S) <- (..N, Fail(E), task.Scheduler, task.Runner, request.Await) -> A,
) <- (task.Scheduler, ..N) -> (S) <- (Fail(E), task.Scheduler) -> A = task.keep

run: <A, N>(() <- N -> A) <- (task.Runner, ..N) -> A = task.run

test: <A, E, N>(
	random.Generator,
	List(List(Int)),
	() <- (Fail(E), ..N) -> A,
) <- (Fail(E | task.ScheduleError), task.Runner, ..N) -> A = task.test

both: <A, B, N>(() <- N -> A, () <- N -> B) <- (task.Scheduler, ..N) -> (
	A,
	B,
) = task.both

race: <A, N>(() <- N -> A, () <- N -> A) <- (
	task.Scheduler,
	..N,
) -> A = task.race

map_concurrently: <A, B, N>(List(A), (A) <- N -> B) <- (
	task.Scheduler,
	..N,
) -> List(B) = task.map_concurrently

main = () { Nil }
