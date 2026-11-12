// spec: 9.9
// expect: pass
// runtime.md D7: the implicit root and local scopes/channels permit boundaries;
// local handles keep the maker's boundary as a portability witness.
(
	i/task
	i/control
	i/core
)

main = () {
	task.local_scope((s) {
		c = task.local_channel()
		task.go(s, () { control.boundary((b) { task.send(c, b) }) })
		t = task.go(s, () { task.receive(c) })
		assert(
			core.not(core.portable(c)),
		)
		assert(
			core.not(core.portable(t)),
		)
		assert(
			core.not(
				core.portable(task.join_event(t)),
			),
		)
		assert(
			core.not(
				core.portable(task.join(t)),
			),
		)
	})
}
