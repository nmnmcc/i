// spec: 9.9
// expect: pass
// output: body
// output: child
// output: after
// runtime.md R3: a normal exit waits for every child, even without join.
(
	i/task
)

@host("test:io/v1")
Out {
	Out(print: (text: String) -> Nil)
}

print = (text: String, out ! Out) { out.print(text) }

@main
main = () {
	assert(task.scope((s) {
		task.go(s, () {
			task.yield()
			print("child")
		})
		print("body")
		42
	}) == 42)
	print("after")
}
