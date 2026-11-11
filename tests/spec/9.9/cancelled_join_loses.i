// spec: 9.9
// expect: pass
// runtime.md R4a/6.4: a cancelled join is ready, not a losing-leaf validation
// fault. A shielded final stops only when that join wins the ordered choice.
(
	i/task
	i/resource
	i/fail
)

@main
main = () {
	result = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(() {
					task.send(armed, Nil)
					task.sync(task.never())
				}, () {
					dropped = task.go(s, () { Nil })
					assert(
						fail.catch(() {
							task.sync(
								task.choose([
									task.always(Nil)
									task.join_event(dropped)
								])
							)
						}) ==
							Ok(Nil)
					)
				})
			})
			task.receive(armed)
			fail.raise("boom")
		})
	})
	assert(result == Error("boom"))
}
