// spec: 9.9
// expect: stop "panic: task: join outside its scope"
// runtime.md R1/R3: returning a handle cannot extend its scope's lifetime.
(
	i/task
)

@main
main = () {
	t = task.scope((s) { task.go(s, () { 7 }) })
	task.join(t)
}
