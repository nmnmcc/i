// spec: 9.9
// expect: pass
// The caller needs Assert, the genuine Fail(Never) key, and Runner.
// run supplies Scheduler. The CLI regression pins this program's printed type.
(
	i/task
	i/control
	i/core
)
event = task.always(1)
main = () {
	task.run(() { assert(task.sync(event) == 1) })
	control.boundary((b) {
		e = task.map(task.always(()), (_) { b })
		assert(core.not(core.portable(e)))
	})
}
