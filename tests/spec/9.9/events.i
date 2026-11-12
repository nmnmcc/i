// spec: 9.9
// expect: pass
// runtime.md 6.4: fresh identities, source-order choice, only winner maps;
// constructing and reusing pure events needs no Scheduler.
(
	i/task
	i/core
)

equal = (a, b) { a == b }

e = task.always(5)

unused = task.map(task.always(9), (_) { panic("losing map ran") })

main = () {
	assert(equal(e, e))
	assert(
		core.not(
			equal(e, task.always(5)),
		),
	)
	assert(
		task.sync(
			task.choose([task.always(3), task.always(4)]),
		) ==
			3,
	)
	assert(
		task.sync(task.choose([e, unused])) == 5,
	)
	assert(
		task.sync(
			task.map(e, (x) {
				task.yield()
				x + 2
			}),
		) ==
			7,
	)
	assert(task.sync(e) == 5)
	assert(task.sync(e) == 5)
}
