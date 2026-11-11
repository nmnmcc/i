// spec: 9.9
// expect: pass
// runtime.md 6.5: nested loops retain fresh identity and route yields outward;
// inner scope bodies still execute in their own opener's absolute path.
(
	i/task
)


main = () {
	assert(task.run(() {
		task.scope((s) {
			child = task.go(s, () {
				task.run(() {
					task.yield()
					task.path()
				})
			})
			task.join(child)
		})
	}) == [1, 1, 1])
}
