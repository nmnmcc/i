// spec: 9.9
// expect: pass
// runtime §6.2: child handles retain results until scope retirement.
// Exercise balanced insertion, active deletion, FIFO and repeated late joins.
(
	i/task
)

spawn = (s: task.Scope(Never), n: Int) -> List(task.Task(Int, Never)) {
	n |>
		(0) { [] }
		| (_) {
			child = task.go(s, () {
				task.yield()
				n
			})
			[child, ..spawn(s, n - 1)]
		}
}

join_all = (tasks: List(task.Task(Int, Never)), sum: Int) -> Int {
	tasks |>
		([]) { sum }
		| ([child, ..rest]) {
			value = task.join(child)
			assert(task.join(child) == value)
			join_all(rest, sum + value)
		}
}

main = () {
	value = task.scope((s) { join_all(spawn(s, 513), 0) })
	assert(value == 131_841)
}
