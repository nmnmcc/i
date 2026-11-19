(
	(Scheduler, Scope, Channel) = i/task
)

(
	empty
	spawn_join
	switch_pair
	channel_pair
)

spin = (n : Int) -> () { n |> (0) { () } & (_) { spin(n - 1) } }

spawns = (s : Scope(Never), n : Int) -> () {
	n |>
		(0) { () }
		& (_) {
			task.join(task.go(s, () { () }))
			spawns(s, n - 1)
		}
}

yields = (n : Int) -> () {
	n |>
		(0) { () }
		& (_) {
			task.yield()
			yields(n - 1)
		}
}

sends = (c : Channel(()), n : Int) -> () {
	n |>
		(0) { () }
		& (_) {
			task.send(c, ())
			sends(c, n - 1)
		}
}

receives = (c : Channel(()), n : Int) -> () {
	n |>
		(0) { () }
		& (_) {
			task.receive(c)
			receives(c, n - 1)
		}
}

empty = (n : Int) -> Int {
	task.scope((_ : Scope(Never)) { spin(n) })
	n
}

spawn_join = (n : Int) -> Int {
	task.scope((s) { spawns(s, n) })
	n
}

switch_pair = (n : Int) -> Int {
	task.scope((s) {
		child = task.go(s, () { yields(n) })
		yields(n)
		task.join(child)
	})
	n
}

channel_pair = (n : Int) -> Int {
	task.scope((s) {
		c : Channel(()) = task.channel()
		child = task.go(s, () { sends(c, n) })
		receives(c, n)
		task.join(child)
	})
	n
}
