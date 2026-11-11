// spec: 9.9
// expect: pass
// runtime.md 6.3: task.keep saves every service except routing. The returned
// callback uses its invoking task's Scheduler and keeps services on third resume.
(
	i/task
	i/control
	i/request
	i/core
)

Name {
	Name(value: String)
}

read_name = (name ! Name) { name.value }

@main
main = () {
	! Name = Name("saved")
	callback = task.keep((x: Int) {
		task.yield()
		task.yield()
		task.yield()
		(read_name(), task.path(), x)
	})
	task.scope((s) {
		t = task.go(s, () {
			! Name = Name("caller")
			answer = callback(7)
			assert(read_name() == "caller")
			answer
		})
		assert(task.join(t) == ("saved", [1], 7))
	})
	assert(read_name() == "saved")
	pure = task.keep((_: Nil) { 1 })
	assert(core.portable(pure))
	control.boundary((b) {
		! Name = Name("saved")
		! Marker = Marker(b)
		task.scope((s) {
			t = task.go(s, () {
				task.yield()
				saved = task.keep((_: Nil) { 1 })
				core.not(core.portable(saved))
			})
			assert(core.portable(t))
			assert(task.join(t))
		})
	})
}

Marker {
	Marker(Boundary(Nil, Never, task.Scheduler | Assert | request.Root))
}
