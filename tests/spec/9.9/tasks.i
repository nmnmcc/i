// spec: 9.9
// expect: pass
// runtime.md 6.1/6.4: bodies stay in the opener, joins yield, go does not;
// D10: all go calls and nested runs use one counter in that opener task.
(
	i/task
)


main = () {
	assert(task.path() == [])
	value = task.scope((s) {
		c: task.Channel(Int) = task.channel()
		task.go(s, () { task.send(c, 7) })
		receiver = task.go(s, () { task.receive(c) + 1 })
		assert(task.path() == [])
		task.join(receiver)
	})
	assert(value == 8)
	assert(task.both(() { 1 }, () { "two" }) == (1, "two"))
	assert(task.map_concurrently([1, 2, 3], (x) { x + 1 }) == [2, 3, 4])
	assert(task.race(() {
		task.yield()
		1
	}, () { 2 }) == 2)
	// The opener spawned 2 + 2 + 3 + 2 children above, so this run is index 10.
	assert(task.run(() { task.path() }) == [10])
}
