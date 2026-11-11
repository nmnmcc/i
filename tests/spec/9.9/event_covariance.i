// spec: 9.9
// expect: pass
// runtime.md 6.1: Event's answer, error and environment parameters are covariant.
(
	i/task
)

widen = (
	e: task.Event(Int, Never, Never)
) -> task.Event(Int | String, String, task.Scheduler) { e }


main = () { assert(task.sync(widen(task.always(7))) == 7) }
