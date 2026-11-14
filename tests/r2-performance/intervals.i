// The interval excludes opening/retiring the enclosing scope. Two synchronous
// timer imports per interval are removed by the n=0 paired measurement.
@external("r2:meter/v1", "now")
now : () -> Float

interval = (body : () <- (Scheduler) -> Nil) -> Float {
	start = now()
	body()
	now() -. start
}

timed_empty = (n : Int) -> Float {
	task.scope((_ : Scope(Never)) { interval(() { spin(n) }) })
}

timed_spawn_join = (n : Int) -> Float {
	task.scope((s) { interval(() { spawns(s, n) }) })
}

timed_switch_pair = (n : Int) -> Float {
	task.scope((s) {
		child = task.go(s, () { yields(n) })
		interval(() {
			yields(n)
			task.join(child)
		})
	})
}

timed_channel_pair = (n : Int) -> Float {
	task.scope((s) {
		c : Channel(Nil) = task.channel()
		child = task.go(s, () { sends(c, n) })
		interval(() {
			receives(c, n)
			task.join(child)
		})
	})
}
