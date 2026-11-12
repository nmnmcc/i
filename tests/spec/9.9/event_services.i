// spec: 9.9
// expect: pass
// runtime.md 6.4: maps compose in order, use the syncing task's services,
// and may suspend after the one message commit; construction itself is pure.
(
	i/task
)

Name {
	Name(text: String)
}

read = (name ! Name) { name.text }

make = () {
	task.map(task.map(task.always(1), (n) {
		task.yield()
		value = (n + 1, read())
		value
	}), (pair) {
		task.yield()
		pair
	})
}


main = () {
	event = make()
	! Name = Name(text: "caller")
	assert(task.sync(event) == (2, "caller"))
	! Name = Name(text: "other")
	assert(task.sync(event) == (2, "other"))
	assert(task.sync(task.choose([task.always(3), task.choose([])])) == 3)
}
