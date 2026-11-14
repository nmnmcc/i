// spec: 9.9
// expect: pass
// run/test supply their callback's Scheduler; their caller supplies Runner.
(
	i/task
	rng = i/random
)

only_runner: (() <- (task.Scheduler) -> Nil) <- (task.Runner) -> Nil = (
	body
) <- (runner: task.Runner) { task.run(body) }

only_runner_test: (rng.Generator, () <- (task.Scheduler) -> Nil) <- (
	Fail(task.ScheduleError),
	task.Runner,
) -> Nil = (seed, body) <- (runner: task.Runner) { task.test(seed, [], body) }

main = () { Nil }
