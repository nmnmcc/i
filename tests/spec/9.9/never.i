// spec: 9.9
// expect: stop "panic: task: deadlock"
// runtime.md 6.4: choose([]) is never; a waiter is not independent work.
(
	i/task
)


main = () { task.sync(task.choose([])) }
