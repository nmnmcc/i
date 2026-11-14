// spec: 9.9
// expect: pass
// runtime.md 6.1: exact public higher-order schemes; the inferred display's
// intersections N & (D | (N - D)) impose no additional needs on the body.
(
	i/task
	i/random
	i/request
)

go : <A, E, N>(task.Scope(E), () <- (Fail(E), ..N) -> A) <- (
	task.Scheduler,
	..N,
) -> task.Task(A, E) = task.go

keep : <S, A, E, N>((S) <- (Fail(E), ..N) -> A) <- (task.Scheduler, ..N) -> (
	S,
) <- (Fail(E), task.Scheduler) -> A = task.keep

run : <A, E, N>(() <- (Fail(E), ..N) -> A) <- (
	Fail(E),
	task.Runner,
	..N,
) -> A = task.run

test : <A, E, N>(
	random.Generator,
	List(List(Int)),
	() <- (Fail(E), ..N) -> A,
) <- (Fail(E | task.ScheduleError), task.Runner, ..N) -> A = task.test

both : <A, B, E, N>(() <- (Fail(E), ..N) -> A, () <- (Fail(E), ..N) -> B) <- (
	Fail(E),
	task.Scheduler,
	..N,
) -> (A, B) = task.both

race : <A, E, N>(() <- (Fail(E), ..N) -> A, () <- (Fail(E), ..N) -> A) <- (
	Fail(E),
	task.Scheduler,
	..N,
) -> A = task.race

map_concurrently : <A, B, E, N>(List(A), (A) <- (Fail(E), ..N) -> B) <- (
	Fail(E),
	task.Scheduler,
	..N,
) -> List(B) = task.map_concurrently

main = () { Nil }
