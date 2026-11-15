// spec: 9.9
// expect: compile-error E0501
// runtime.md 6.1: Scope, Task, Channel and Shared parameters are invariant.
(
	i/task
)

widen = (x : task.Task(Nil, Int)) -> task.Task(Nil, Int | String) { x }
