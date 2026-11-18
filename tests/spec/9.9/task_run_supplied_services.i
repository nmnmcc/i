// spec: 9.9
// expect: pass
// run/test supply their callback's Scheduler; their caller supplies Runner.
(
	i/task
	rng = i/random
)

only_runner: (() <- (task.Scheduler) -> ()) <- (task.Runner) -> () = (
	body
) <- (runner: task.Runner) { task.run(body) }

only_runner_test: (rng.Generator, () <- (task.Scheduler) -> ()) <- (
	Fail(task.ScheduleError),
	task.Runner,
) -> () = (seed, body) <- (runner: task.Runner) { task.test(seed, [], body) }

main = () { () }
