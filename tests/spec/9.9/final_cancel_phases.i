// spec: 9.5, 9.9
// expect: pass
// output: start
// output: inner
// output: end
// output: Error("parent")
// output: start
// output: inner
// output: end
// output: Error("parent")
// output: start
// output: inner
// output: end
// output: Error("parent")
// output: start
// output: inner
// output: end
// output: Error("parent")
// output: start
// output: inner
// output: end
// output: Error("parent")
// output: start
// output: inner
// output: end
// output: Error("parent")
// Before/during/after cleanup, with normal and typed-error body exits.
(
	i/task
	i/resource
	i/fail
	i/core
)

@host("test:io/v1")
Out {
	Out(print: (String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }

probe = (phase: Int, typed: Bool) {
	result = fail.catch(() {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			never: task.Channel(Nil) = task.channel()
			started: task.Channel(Nil) = task.channel()
			child = task.go(s, () {
				fail.catch(() {
					resource.ensure(() {
						phase |> {
							0 -> {
								task.send(started, Nil)
								task.receive(never)
							}
							_ -> Nil
						}
						typed |> {
							True  -> fail.raise("child")
							False -> Nil
						}
					}, () {
						print("start")
						phase |> {
							1 -> task.send(gate, Nil)
							_ -> Nil
						}
						resource.ensure(() { task.yield() }, () {
							task.yield()
							print("inner")
						})
						task.yield()
						print("end")
					})
				})
				Nil
			})
			// §9.9: "Timing may choose ... which failures precede
			// cancellation"; the child says it started before the parent fails.
			phase |> {
				0 -> task.receive(started)
				1 -> task.receive(gate)
				_ -> task.join(child)
			}
			fail.raise("parent")
		})
	})
	print(core.inspect(result))
}


main = () {
	probe(0, False)
	probe(1, False)
	probe(2, False)
	probe(0, True)
	probe(1, True)
	probe(2, True)
}
