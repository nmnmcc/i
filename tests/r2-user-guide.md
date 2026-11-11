Programs can now:

- Start children in a scope, join their results, and wait for all cleanup.
- Race computations and cancel the loser, including its final functions.
- Exchange values through unbuffered FIFO channels.
- Compose events with always, never, map, choose and sync.
- Use deterministic task paths and random streams, and detect deadlock.

These programs run on the Rust host at O0 and O2. The integration test checks
their complete outputs. They use the shipped entry services; no explicit run
wrapper or manifest provider plan is needed in R2.

Two children and a result:

```i
(
	i/task
	i/core
	(Io) = i/io
)


main = (io ! Io) {
	answer = task.scope((s) {
		a = task.go(s, () { 20 })
		b = task.go(s, () { 22 })
		task.join(a) + task.join(b)
	})
	io.print(core.inspect(answer))
}
```

Output:

```text
42
```

Cancellation of a race loser:

```i
(
	i/task
	i/resource
	i/core
	(Io) = i/io
)

forever = () -> Never ! task.Scheduler {
	task.yield()
	forever()
}


main = (io ! Io) {
	answer = task.race(() {
		task.yield()
		7
	}, () { resource.ensure(forever, () { io.print("loser cancelled") }) })
	io.print(core.inspect(answer))
}
```

Output:

```text
loser cancelled
7
```

A channel between two children:

```i
(
	i/task
	(Io) = i/io
)


main = (io ! Io) {
	task.scope((s) {
		c: task.Channel(String) = task.channel()
		task.go(s, () { task.send(c, "hello") })
		receiver = task.go(s, () { io.print(task.receive(c)) })
		task.join(receiver)
	})
}
```

Output:

```text
hello
```

In the channel program, remove the sender's go call. The receiver and its
joiner then block with no requests or runnable tasks. The separately executed
deadlock package stops with `panic: task: deadlock`. A blocked receiver is not
independent work (runtime.md §6.4).

The contracts behind the outputs are runtime.md §6.2: scope returns only after
its children are clean and race cancels its loser; §6.1: channel is an unbuffered
FIFO rendezvous; §6.4: no runnable task and an empty program waitable set means
deadlock. These expectations were written from those rules, not generated from
an observed run.
