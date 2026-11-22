// spec: 9.9
// expect: pass
// run/test supply their callback's Scheduler; their caller supplies Runner.
// Both are implicit services, so neither is written in these types.
(
	i/task
	rng = i/random
)

only_runner: (() -> ()) -> () = (body) { task.run(body) }

only_runner_test: (rng.Generator, () -> ()) <- (
	Fail(task.ScheduleError),
) -> () = (seed, body) { task.test(seed, [], body) }

main = () { () }
