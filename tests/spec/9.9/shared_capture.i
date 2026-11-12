// spec: 9.9, 11.9
// expect: pass
// Shared data must not be expanded along every path during go or portable.
(
	i/task
	i/control
	i/core
)

Tip(A) = (A)

Node(A) = (Tree(A), Tree(A))
Tree(A) = Tip(A) | Node(A)
make = <A>(value: A, depth: Int) -> Tree(A) {
	depth |>
		(0) { Tip(value) }
		| (_) {
			child = make(value, depth - 1)
			Node(child, child)
		}
}

spawn = (depth: Int) {
	value = make(1, depth)
	task.scope((s) {
		child = task.go(s, () { value |> (Tip(_)) { 0 } | (Node(_, _)) { 1 } })
		assert(task.join(child) == 1)
		shared = task.share(s, value)
		task.get(shared) |> (Tip(_)) { assert(False) } | (Node(_, _)) { Nil }
	})
	assert(core.portable((value, value, [value, value])))
	assert(core.portable(value))
}

main = () {
	spawn(28)
	spawn(60)
	control.boundary((b) {
		value = make(b, 60)
		assert(
			core.not(core.portable(value)),
		)
		assert(
			core.not(core.portable(value)),
		)
	})
}
