// spec: 9.9
// expect: pass
// A quiet source keeps an older live waiter while many losing leaves withdraw.
(
	i/task
)

loop = (
	a: task.Channel(Int),
	b: task.Channel(Int),
	c: task.Channel(Int),
	n: Int
) -> Nil {
	n |>
		(0) { Nil }
		| (_) {
			value = task.sync(
				task.choose(
					[
						task.receive_event(a),
						task.receive_event(b),
						task.receive_event(c),
					],
				),
			)
			assert(value == n)
			loop(a, b, c, n - 1)
		}
}

send = (c: task.Channel(Int), n: Int) -> Nil {
	n |>
		(0) { Nil }
		| (_) {
			task.send(c, n)
			send(c, n - 1)
		}
}

main = () {
	task.scope((s) {
		a: task.Channel(Int) = task.channel()
		b: task.Channel(Int) = task.channel()
		c: task.Channel(Int) = task.channel()
		older = task.go(s, () { assert(task.receive(b) == -1) })
		producer = task.go(s, () { send(a, 257) })
		loop(a, b, c, 257)
		task.send(b, -1)
		task.join(older)
		task.join(producer)
	})
}
