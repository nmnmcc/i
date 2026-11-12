// spec: 9.9
// expect: pass
// output: blocked-final
// runtime.md R2/R4: cancelling a callback's body cannot return a fabricated
// answer to a blocked outer operation; its invoking child unwinds and drains.
(
	i/task
	i/control
	i/resource
	i/fail
)

@host("test:io/v1")
Out = (print = (String) -> Nil)

print = (text: String) <- (out: Out) { out.print(text) }

main = () {
	callback = control.keep_env((work: (() -> Nil, () -> Nil) -> Nil) {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			parked: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.receive(gate)
				task.yield()
				fail.raise("callback")
			})
			arm = control.keep_env((_: Nil) { task.send(gate, Nil) })
			park = control.keep_env((_: Nil) { task.send(parked, Nil) })
			work(() { arm(Nil) }, () { park(Nil) })
		})
	})
	assert(
		task.run(() {
			task.run(() {
				work = control.keep_env((callbacks: (() -> Nil, () -> Nil)) {
					(arm, park) = callbacks
					task.scope((inner) {
						child = task.go(inner, () {
							resource.ensure(
								() {
									arm()
									park()
									panic("blocked callback returned")
								},
								() {
									task.yield()
									print("blocked-final")
								},
							)
						})
						task.join(child)
					})
				})
				assert(
					fail.catch(() {
						callback((arm, park) { work((arm, park)) })
					}) ==
						Error("callback"),
				)
				42
			})
		}) ==
			42,
	)
}
