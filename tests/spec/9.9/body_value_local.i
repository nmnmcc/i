// spec: 9.9
// expect: pass
// runtime.md D7: a portable scope's own successful value is not a child result.
(
	i/task
	i/control
	i/core
)

main = () {
	control.boundary((b) {
		result = task.scope((_) { b })
		assert(
			core.not(core.portable(result)),
		)
	})
}
