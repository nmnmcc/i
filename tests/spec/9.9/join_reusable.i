// spec: 9.9
// expect: pass
// runtime.md R3/6.3: successful task results remain in the table until scope
// exit; joining is not consuming a request answer.
(
	i/task
)


main = () {
	task.scope((s) {
		t = task.go(s, () { 7 })
		assert(task.join(t) == 7)
		assert(task.join(t) == 7)
		another = task.go(s, () { task.join(t) })
		assert(task.join(another) == 7)
	})
}
