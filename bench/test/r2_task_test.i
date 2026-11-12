//// Scheduler costs with checked results, measured by scripts/bench.
//// The spawn case includes scope creation/retirement and a join; the channel
//// case includes both peers and their switches. These are aggregate costs.
(
	testlib = (test) = std/test
	i/task
)

rounds = 1_000

spawns = (n: Int, total: Int) -> Int {
	n |>
		(0) { total }
		| (_) {
			value = task.scope((s) { task.join(task.go(s, () { 1 })) })
			spawns(n - 1, total + value)
		}
}

switches = (n: Int, total: Int) -> Int {
	n |>
		(0) { total }
		| (_) {
			task.yield()
			switches(n - 1, total + 1)
		}
}

sends = (c: task.Channel(Int), n: Int) -> Nil {
	n |>
		(0) { Nil }
		| (_) {
			task.send(c, n)
			sends(c, n - 1)
		}
}

receives = (c: task.Channel(Int), n: Int, total: Int) -> Int {
	n |>
		(0) { total }
		| (_) {
			value = task.receive(c)
			receives(c, n - 1, total + value)
		}
}

test("r2_spawn_join_1000", () { assert(spawns(rounds, 0) == rounds) })

test("r2_switch_1000", () { assert(switches(rounds, 0) == rounds) })

test("r2_channel_1000", () {
	result = task.scope((s) {
		c: task.Channel(Int) = task.channel()
		task.go(s, () { sends(c, rounds) })
		receives(c, rounds, 0)
	})
	assert(result == 500_500)
})
