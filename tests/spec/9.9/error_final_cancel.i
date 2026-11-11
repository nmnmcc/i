// spec: 9.5, 9.9
// expect: pass
// output: final-start
// output: final-end
// output: Error("parent")
// multicore §5: an already-unwinding task is not cancelled again.
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


main = () {
	r = fail.catch(() {
		task.scope((s) {
			gate: task.Channel(Nil) = task.channel()
			task.go(s, () {
				resource.ensure(() { fail.raise("child") }, () {
					print("final-start")
					task.send(gate, Nil)
					task.yield()
					print("final-end")
				})
			})
			task.receive(gate)
			fail.raise("parent")
		})
	})
	print(core.inspect(r))
}
