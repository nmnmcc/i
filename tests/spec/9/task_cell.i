// spec: 9.9
// expect: pass
(
	i/task
)

@main
main = () {
	task.run(() {
		state = task.cell(0)
		task.scope((s) {
			left = task.go(s, () { task.update(state, (n) { (n + 1, Nil) }) })
			right = task.go(s, () { task.update(state, (n) { (n + 1, Nil) }) })
			task.join(left)
			task.join(right)
		})
		assert(task.update(state, (n) { (n, n) }) == 2)
	})
}
