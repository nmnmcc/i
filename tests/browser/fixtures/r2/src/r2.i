//// R2 host probes shared by Rust, Node and browser tests.
//// Expectations follow runtime.md §§5–6 and D5/D10, not generated results.
(
	i/task
	i/request
	i/control
	i/fail
	i/core
	i/random
)

(
	Gates
	Control
	unlisted_answer
	go_progress
	overlap
	cancel_owned
	cancel_borrowed
	race_read
	nested_interest
	owned_idle
	unrelated_idle
	raw_only
	pure_tasks
	cmp
	cmp2
	fake
	sync_poll
	trace
	expected_trace
)

@host("r2:gates/v1", "request")
Gates {
	Gates(wait: (id: Int) -> Request(Int))
}

@host("r2:control/v1")
Control {
	Control(release: (id: Int) -> Nil, check: (phase: Int) -> Nil)
}

// §6.4: both starts precede release; polls cannot starve the runnable releaser.
overlap = (gates ! Gates, control ! Control) {
	task.scope((s) {
		a = task.go(s, () { request.await_owned(gates.wait(1)) })
		b = task.go(s, () { request.await_owned(gates.wait(2)) })
		task.go(s, () { control.release(0) })
		task.join(a) + task.join(b)
	})
}

// §5.1: an unlisted completion is stored while the original wait continues.
unlisted_answer = (gates ! Gates, control ! Control) {
	a = gates.wait(1)
	b = gates.wait(2)
	control.release(2)
	task.scope((s) {
		first = task.go(s, () { request.await_owned(a) })
		task.go(s, () {
			ready_wait(b)
			control.release(1)
		})
		value = task.join(first)
		assert(request.state(b) == request.Ready(2))
		value + request.await_owned(b)
	})
}

// §6.4: go keeps the parent turn but polls an existing IO interest.
go_progress = (gates ! Gates, control ! Control) {
	task.scope((s) {
		waiter = task.go(s, () { request.await_owned(gates.wait(10)) })
		task.yield()
		control.release(10)
		task.go(s, () { Nil })
		control.check(5)
		task.join(waiter)
	})
}

cancel_probe = (owned: Bool, gates ! Gates, control ! Control) {
	r = gates.wait(1)
	answer = fail.catch(() {
		task.scope((s) {
			armed: task.Channel(Nil) = task.channel()
			task.go(s, () {
				task.send(armed, Nil)
				{ owned } |> {
					True  -> request.await_owned(r)
					False -> request.await(r)
				}
			})
			task.go(s, () {
				task.receive(armed)
				fail.raise("cancel")
			})
			task.sync(task.never())
		})
	})
	assert(answer == Error("cancel"))
	{ owned } |> {
		True -> {
			assert(request.state(r) == request.Cancelled)
			control.check(1)
		}
		False -> {
			assert(request.state(r) == request.Waiting)
			control.check(2)
		}
	}
	request.cancel(request.ticket(r))
	control.check(1)
}

cancel_owned = () { cancel_probe(True) }

cancel_borrowed = () { cancel_probe(False) }

ready_wait(A) = (r: Request(A), await ! request.Await) {
	await.wait(request.ticket(r))
}

// §5.4 race-then-read: the losing waiter never owns the producer.
race_read = (gates ! Gates, control ! Control) {
	a = gates.wait(1)
	b = gates.wait(2)
	task.scope((s) {
		task.go(s, () { control.release(2) })
		winner = task.race(() {
			ready_wait(a)
			1
		}, () {
			ready_wait(b)
			2
		})
		assert(winner == 2)
		control.check(3)
		value = request.await_owned(b)
		assert(value == 2)
		assert(request.state(a) == request.Waiting)
		value
	})
}

forever = () -> Never ! task.Scheduler {
	task.yield()
	forever()
}

// §12.8: nested waiter interests reach the root despite a CPU-yielding sibling.
nested_interest = (gates ! Gates) {
	task.race(
		() {
			task.run(() {
				task.race(() { request.await_owned(gates.wait(9)) }, () {
					bounded_cpu(32)
				})
			})
		}
		forever
	)
}

// A missing nested progress-interest route fails after turns, without a timer
// or an indefinitely hanging test process. A delivered answer gets a FIFO turn.
bounded_cpu = (turns: Int) -> Never ! task.Scheduler {
	turns |> {
		0 -> panic("nested IO progress was not forwarded")
		_ -> {
			task.yield()
			bounded_cpu(turns - 1)
		}
	}
}

// §6.4: every Owner affiliated with this run's Root counts as work.
owned_idle = (gates ! Gates) {
	! request.Owner = request.open_owner()
	gates.wait(9)
	task.sync(task.never())
	Nil
}

// §6.4: a producer in a different Root cannot keep this run alive.
unrelated_idle = (gates ! Gates) {
	unrelated = request.open_root()
	! request.Root = unrelated
	! request.Owner = request.open_owner()
	gates.wait(9)
	task.sync(task.never())
	Nil
}

// §12.8: raw work alone does not authorize runnable progress polls.
raw_only = (gates ! Gates, control ! Control) {
	gates.wait(9)
	task.yield()
	task.yield()
	control.check(4)
	7
}

pure_tasks = () {
	task.both(() {
		task.yield()
		2
	}, () {
		task.yield()
		5
	}) |> {
		(a, b) -> a + b
	}
}

cmp = (a: Float, b: Float) { a -. b }

cmp2 = (a: Float, b: Float) { cmp(a, b) }

fake = () { request.await_owned(request.ready(7)) }

sync_poll = () { request.poll() }

one = () { core.inspect((task.path(), random.word(task.random()))) }

trace = (seed: Int) {
	result = fail.catch(() {
		task.test(random.from_seed(seed), [[1], [1, 2], [1, 1]], () {
			task.scope((s) {
				a = task.go(s, () {
					task.yield()
					one()
				})
				b = task.go(s, () {
					task.yield()
					one()
				})
				task.join(a) <> "/" <> task.join(b)
			})
		})
	})
	result |> {
		Ok(value) -> value
		Error(_)  -> panic("invalid replay script")
	}
}

expected_one = (seed: Int, p: List(Int)) {
	generator = fail.catch(() {
		random.at_path(random.from_seed(seed), p)
	}) |> {
		Ok(g)    -> g
		Error(_) -> panic("invalid expected path")
	}
	core.inspect((p, random.word(generator)))
}

expected_trace = (seed: Int) {
	expected_one(seed, [1, 1]) <> "/" <> expected_one(seed, [1, 2])
}
