// spec: 9.9
// expect: pass
// The caller's need is Assert + Runner, not Scheduler. The CLI regression
// checks the printed type of this exact program.
(
	i/task
	i/control
	i/core
)
event = task.always(1)
main = () {
	task.run(() { assert(task.sync(event) == 1) })
	control.boundary((b) {
		e = task.map(task.always(Nil), (_) { b })
		assert(core.not(core.portable(e)))
	})
}
