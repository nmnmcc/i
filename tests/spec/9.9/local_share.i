// spec: 9.9
// expect: pass
// runtime.md 6.1/D7: shares are portable key-bearing handles even in a local
// scope, while their values must independently pass the ordinary portable walk.
(
	i/task
	i/core
)

main = () {
	task.local_scope((s) {
		value = task.share(s, "shared")
		assert(core.portable(value))
		assert(task.get(value) == "shared")
		task.go(s, () { assert(task.get(value) == "shared") })
		Nil
	})
}
