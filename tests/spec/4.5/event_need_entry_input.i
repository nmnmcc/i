// spec: 4.5, 9.9
// expect: compile-error E0501
(
	i/task
)

Read(A) = (read = () -> A)
Pending(A) = (event = task.Event(Int, Never, Read(A)))
up = (x: Pending(Int)) -> Pending(Int | String) { x }
main = () { Nil }
