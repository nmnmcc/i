// Design counterexamples; not run.
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
		assert(
			core.not(core.portable(e)),
		)
	})
}
