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
Out = (print = (String) -> ())

print = (text : String) <- (out : Out) { out.print(text) }

// Decision 2: install the real key required by a kept pure callback.
without_fail = <A, N>(body : () <- (Fail(Never), ..N) -> A) -> A {
	body() <- (Fail(raise = (_ : Never) { panic("fixture: impossible error") }))
}

main = () {
	callback = control.keep((work : (() -> (), () -> ()) -> ()) {
		task.scope((s) {
			gate : task.Channel(()) = task.channel()
			parked : task.Channel(()) = task.channel()
			task.go(s, () {
				task.receive(gate)
				task.yield()
				fail.raise("callback")
			})
			arm = control.keep((_ : ()) { task.send(gate, ()) })
			park = control.keep((_ : ()) { task.send(parked, ()) })
			work(
				() { without_fail(() { arm(()) }) },
				() { without_fail(() { park(()) }) },
			)
		})
	})
	assert(
		task.run(() {
			task.run(() {
				work = control.keep((callbacks : (() -> (), () -> ())) {
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
						callback((arm, park) {
							without_fail(() { work((arm, park)) })
						})
					}) ==
						Error("callback"),
				)
				42
			})
		}) ==
			42,
	)
}
