(
	(Scheduler, Scope, Channel) = i/task
)

(
	empty
	spawn_join
	switch_pair
	channel_pair
)

spin = (n: Int) -> Nil {
	n |> {
		0 -> Nil
		_ -> spin(n - 1)
	}
}

spawns = (s: Scope(Never), n: Int) -> Nil ! Scheduler {
	n |> {
		0 -> Nil
		_ -> {
			task.join(task.go(s, () { Nil }))
			spawns(s, n - 1)
		}
	}
}

yields = (n: Int) -> Nil ! Scheduler {
	n |> {
		0 -> Nil
		_ -> {
			task.yield()
			yields(n - 1)
		}
	}
}

sends = (c: Channel(Nil), n: Int) -> Nil ! Scheduler {
	n |> {
		0 -> Nil
		_ -> {
			task.send(c, Nil)
			sends(c, n - 1)
		}
	}
}

receives = (c: Channel(Nil), n: Int) -> Nil ! Scheduler {
	n |> {
		0 -> Nil
		_ -> {
			task.receive(c)
			receives(c, n - 1)
		}
	}
}

empty = (n: Int) -> Int ! Scheduler {
	task.scope((_: Scope(Never)) { spin(n) })
	n
}

spawn_join = (n: Int) -> Int ! Scheduler {
	task.scope((s) { spawns(s, n) })
	n
}

switch_pair = (n: Int) -> Int ! Scheduler {
	task.scope((s) {
		child = task.go(s, () { yields(n) })
		yields(n)
		task.join(child)
	})
	n
}

channel_pair = (n: Int) -> Int ! Scheduler {
	task.scope((s) {
		c: Channel(Nil) = task.channel()
		child = task.go(s, () { sends(c, n) })
		receives(c, n)
		task.join(child)
	})
	n
}
