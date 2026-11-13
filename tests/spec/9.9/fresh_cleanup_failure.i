// spec: 9.9
// expect: pass
// output: cleanup-child-final
// output: outer-final
// runtime.md R4/R4a: a fresh cleanup scope obeys its own failure rules. Its
// cancellable body unwinds before observing a cancelled join, despite the
// older shield around the outer final.
(
	i/task
	i/resource
	i/fail
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

print = (text: String) <- (out: Out) { out.print(text) }

main = () {
	result = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(
					() {
						task.send(armed, Nil)
						task.sync(task.never())
					},
					() {
						inner = fail.catch(() {
							task.scope((cleanup) {
								ready: task.Channel(Nil) = task.channel()
								target = task.go(cleanup, () {
									resource.ensure(
										() {
											task.send(ready, Nil)
											task.sync(task.never())
										},
										() {
											task.yield()
											print("cleanup-child-final")
										},
									)
								})
								// An unstarted child has no installed final (§9.5).
								task.go(cleanup, () {
									task.receive(ready)
									fail.raise("cleanup")
								})
								task.join(target)
							})
						})
						assert(inner == Error("cleanup"))
						print("outer-final")
					},
				)
			})
			task.receive(armed)
			fail.raise("outer")
		})
	})
	assert(result == Error("outer"))
}
