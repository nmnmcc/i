// spec: 9.9
// expect: stop "panic: task: shared value is gone"
// runtime.md R3: scope exit frees share storage even when its handle survives.
(
	i/task
)

main = () {
	shared = task.scope((s) { task.share(s, 7) })
	task.get(shared)
}
