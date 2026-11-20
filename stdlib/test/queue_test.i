(
	testlib = (test) = i/test
	i/fail
	(Option, Some, None) = i/option
	i/queue
	i/resource
	i/task
)

// Runs `body` on a fresh scheduler, inside one task scope and one resource
// scope, so that queues made in `body` belong to both.
in_queue_scope = <A, N>(
	body : (task.Group(Never)) <- (resource.Scope, task.Scheduler, ..N) -> A
) <- (runner : task.Runner, ..env : N) -> A {
	task.run(() { task.scope((s) { resource.scoped(() { body(s) }) }) })
}

// Makes a queue whose capacity is known to be valid.
must = <A, N>(make : () <- (Fail(queue.QueueError), ..N) -> A) <- (
	..env : N,
) -> A {
	fail.catch(make) |> (Ok(q)) { q } & (Error(_)) { panic("queue: invalid") }
}

closed = <A, N>(body : () <- (Fail(queue.QueueError), ..N) -> A) <- (
	..env : N,
) -> Bool {
	fail.catch(body) |> (Error(queue.Closed())) { True() } & (_) { False() }
}

test("queue_fifo_offer_then_take_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(String, Never) = queue.unbounded()
		empty = queue.size(q)
		accepted = must(() {
			[queue.offer(q, "a"), queue.offer(q, "b"), queue.offer(q, "c")]
		})
		full = queue.size(q)
		taken = [queue.take(q), queue.take(q), queue.take(q)]
		(empty, accepted, full, taken, queue.size(q), queue.poll(q))
	})
	assert(
		got ==
			(
				0,
				[True(), True(), True()],
				3,
				[Some("a"), Some("b"), Some("c")],
				0,
				None(),
			),
	)
	()
})

test("queue_constructors_reject_non_positive_capacity_test", () {
	got = in_queue_scope((_) {
		bounded = fail.catch(() {
			q : queue.Queue(Int, Never) = queue.bounded(0)
			q
		})
		dropping = fail.catch(() {
			q : queue.Queue(Int, Never) = queue.dropping(-1)
			q
		})
		sliding = fail.catch(() {
			q : queue.Queue(Int, Never) = queue.sliding(0)
			q
		})
		(
			bounded |> (Error(e)) { Some(e) } & (Ok(_)) { None() },
			dropping |> (Error(e)) { Some(e) } & (Ok(_)) { None() },
			sliding |> (Error(e)) { Some(e) } & (Ok(_)) { None() },
		)
	})
	assert(
		got ==
			(
				Some(queue.InvalidCapacity(capacity = 0)),
				Some(queue.InvalidCapacity(capacity = -1)),
				Some(queue.InvalidCapacity(capacity = 0)),
			),
	)
	()
})

test("queue_bounded_offer_waits_for_room_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, Never) = must(() { queue.bounded(1) })
		first = must(() { queue.offer(q, 1) })
		done = task.ref(False())
		producer = task.go(s, () {
			accepted = must(() { queue.offer(q, 2) })
			task.set(done, True())
			accepted
		})
		task.yield()
		task.yield()
		task.yield()
		blocked = (task.get(done), queue.size(q))
		taken = queue.take(q)
		accepted = task.join(producer)
		(first, blocked, taken, accepted, task.get(done), queue.take(q))
	})
	assert(got == (True(), (False(), 1), Some(1), True(), True(), Some(2)))
	()
})

test("queue_bounded_offers_wait_in_order_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, Never) = must(() { queue.bounded(1) })
		must(() { queue.offer(q, 1) })
		a = task.go(s, () { must(() { queue.offer(q, 2) }) })
		task.yield()
		b = task.go(s, () { must(() { queue.offer(q, 3) }) })
		task.yield()
		values = [queue.take(q), queue.take(q), queue.take(q)]
		(values, task.join(a), task.join(b), queue.size(q))
	})
	assert(got == ([Some(1), Some(2), Some(3)], True(), True(), 0))
	()
})

test("queue_waiting_takers_are_served_in_order_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(String, Never) = queue.unbounded()
		a = task.go(s, () { queue.take(q) })
		task.yield()
		b = task.go(s, () { queue.take(q) })
		task.yield()
		c = task.go(s, () { queue.take(q) })
		task.yield()
		// Two values for three waiting takers: the first two in line get
		// them, in the order they joined, and the third keeps waiting.
		must(() {
			queue.offer(q, "first")
			queue.offer(q, "second")
		})
		first = task.join(a)
		second = task.join(b)
		waiting = queue.size(q)
		must(() { queue.offer(q, "third") })
		(first, second, waiting, task.join(c), queue.size(q))
	})
	assert(got == (Some("first"), Some("second"), 0, Some("third"), 0))
	()
})

test("queue_poll_does_not_overtake_a_waiting_taker_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, Never) = queue.unbounded()
		// The taker gives up after a few turns, so a poll that stole its
		// value fails the assertion instead of hanging the run.
		taker = task.go(s, () { take_soon(q) })
		task.yield()
		task.yield()
		must(() { queue.offer(q, 4) })
		// The value is buffered, but the waiting taker is first in line.
		stolen = queue.poll(q)
		(stolen, task.join(taker), queue.size(q))
	})
	assert(got == (None(), Some(4), 0))
	()
})

test("queue_dropping_rejects_at_capacity_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, Never) = must(() { queue.dropping(2) })
		accepted = must(() {
			[queue.offer(q, 1), queue.offer(q, 2), queue.offer(q, 3)]
		})
		(accepted, queue.size(q), queue.take(q), queue.take(q), queue.poll(q))
	})
	assert(got == ([True(), True(), False()], 2, Some(1), Some(2), None()))
	()
})

test("queue_sliding_evicts_oldest_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, Never) = must(() { queue.sliding(2) })
		accepted = must(() {
			[queue.offer(q, 1), queue.offer(q, 2), queue.offer(q, 3)]
		})
		(accepted, queue.size(q), queue.take(q), queue.take(q), queue.poll(q))
	})
	assert(got == ([True(), True(), True()], 2, Some(2), Some(3), None()))
	()
})

test("queue_end_drains_then_none_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, String) = queue.unbounded()
		must(() {
			queue.offer(q, 1)
			queue.offer(q, 2)
		})
		queue.end(q)
		refused = closed(() { queue.offer(q, 3) })
		// The first terminal transition wins.
		queue.fail(q, "late")
		taken = fail.catch(() {
			[queue.take(q), queue.poll(q), queue.take(q), queue.take(q)]
		})
		(refused, taken, queue.size(q))
	})
	assert(got == (True(), Ok([Some(1), Some(2), None(), None()]), 0))
	()
})

test("queue_end_wakes_waiting_taker_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, Never) = queue.unbounded()
		taker = task.go(s, () { queue.take(q) })
		task.yield()
		task.yield()
		queue.end(q)
		task.join(taker)
	})
	assert(got == None())
	()
})

test("queue_poll_after_fail_keeps_the_waiting_takers_value_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, String) = queue.unbounded()
		taker = task.go(s, () { fail.catch(() { take_soon(q) }) })
		task.yield()
		task.yield()
		must(() { queue.offer(q, 1) })
		queue.fail(q, "boom")
		// The taker is still first in line, so the failure is not reported
		// to the poll before the value is drained.
		before = fail.catch(() { queue.poll(q) })
		taken = task.join(taker)
		after = fail.catch(() { queue.poll(q) })
		(before, taken, after)
	})
	assert(got == (Ok(None()), Ok(Some(1)), Error("boom")))
	()
})

test("queue_fail_drains_then_raises_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, String) = queue.unbounded()
		empty : queue.Queue(Int, String) = queue.unbounded()
		waiting = task.go(s, () { fail.catch(() { queue.take(empty) }) })
		task.yield()
		must(() { queue.offer(q, 1) })
		queue.fail(q, "boom")
		queue.fail(empty, "empty")
		queue.end(q)
		refused = closed(() { queue.offer(q, 2) })
		first = fail.catch(() { queue.take(q) })
		second = fail.catch(() { queue.take(q) })
		third = fail.catch(() { queue.take(q) })
		polled = fail.catch(() { queue.poll(q) })
		(refused, first, second, third, polled, task.join(waiting))
	})
	assert(
		got ==
			(
				True(),
				Ok(Some(1)),
				Error("boom"),
				Error("boom"),
				Error("boom"),
				Error("empty"),
			),
	)
	()
})

test("queue_shutdown_discards_and_wakes_waiters_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, String) = queue.unbounded()
		must(() {
			queue.offer(q, 1)
			queue.offer(q, 2)
		})
		empty : queue.Queue(Int, String) = queue.unbounded()
		a = task.go(s, () { fail.catch(() { queue.take(empty) }) })
		b = task.go(s, () { fail.catch(() { queue.take(empty) }) })
		full : queue.Queue(Int, Never) = must(() { queue.bounded(1) })
		must(() { queue.offer(full, 1) })
		offering = task.go(s, () { closed(() { queue.offer(full, 2) }) })
		task.yield()
		task.yield()
		queue.end(q)
		queue.shutdown(q)
		queue.shutdown(empty)
		queue.shutdown(full)
		(
			queue.size(q),
			fail.catch(() { queue.take(q) }),
			fail.catch(() { queue.poll(q) }),
			closed(() { queue.offer(q, 3) }),
			task.join(a),
			task.join(b),
			task.join(offering),
			queue.size(full),
		)
	})
	assert(
		got ==
			(
				0,
				Ok(None()),
				Ok(None()),
				True(),
				Ok(None()),
				Ok(None()),
				True(),
				0,
			),
	)
	()
})

test("queue_resource_scope_exit_shuts_down_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, Never) = resource.scoped(() {
			made : queue.Queue(Int, Never) = queue.unbounded()
			must(() { queue.offer(made, 1) })
			made
		})
		(queue.size(q), queue.take(q), closed(() { queue.offer(q, 2) }))
	})
	assert(got == (0, None(), True()))
	()
})

// Takes from `q`, or gives `None` when nothing arrives within a few turns,
// so that a lost wake-up fails the test instead of hanging it.
take_soon = <A, E>(q : queue.Queue(A, E)) <- (
	own : task.Scheduler,
	error : Fail(E),
) -> Option(A) {
	task.race(
		() { queue.take(q) },
		() {
			task.yield()
			task.yield()
			task.yield()
			task.yield()
			None()
		},
	)
}

offer_soon = <A, E>(q : queue.Queue(A, E), value : A) <- (
	own : task.Scheduler,
	error : Fail(queue.QueueError),
) -> Option(Bool) {
	task.race(
		() { Some(queue.offer(q, value)) },
		() {
			task.yield()
			task.yield()
			task.yield()
			task.yield()
			None()
		},
	)
}

test("queue_cancelled_take_leaves_the_line_test", () {
	got = in_queue_scope((s) {
		q : queue.Queue(Int, Never) = queue.unbounded()
		stopped = fail.catch(() {
			task.scope((inner) {
				task.go(inner, () { queue.take(q) })
				task.yield()
				task.yield()
				fail.raise("stop")
			})
		})
		must(() { queue.offer(q, 7) })
		after = take_soon(q)
		later = task.go(s, () { take_soon(q) })
		task.yield()
		must(() { queue.offer(q, 8) })
		(stopped, after, task.join(later), queue.size(q))
	})
	assert(got == (Error("stop"), Some(7), Some(8), 0))
	()
})

test("queue_cancelled_offer_leaves_the_line_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, Never) = must(() { queue.bounded(1) })
		must(() { queue.offer(q, 1) })
		stopped = fail.catch(() {
			task.scope((inner) {
				task.go(inner, () { must(() { queue.offer(q, 2) }) })
				task.yield()
				task.yield()
				fail.raise("stop")
			})
		})
		first = queue.take(q)
		// The cancelled offer neither inserted 2 nor kept its place in line.
		offered = must(() { offer_soon(q, 3) })
		(stopped, first, offered, take_soon(q), queue.poll(q))
	})
	assert(got == (Error("stop"), Some(1), Some(True()), Some(3), None()))
	()
})

test("queue_losing_race_take_consumes_nothing_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, Never) = queue.unbounded()
		// The take branch of the race is cancelled while it waits: it must
		// not hold a ticket, and it must not consume the next value.
		lost = take_soon(q)
		lost_then = take_soon(q)
		must(() { queue.offer(q, 9) })
		after = queue.take(q)
		(lost, lost_then, after, queue.size(q))
	})
	assert(got == (None(), None(), Some(9), 0))
	()
})

test("queue_poll_takes_at_most_one_test", () {
	got = in_queue_scope((_) {
		q : queue.Queue(Int, Never) = queue.unbounded()
		empty = queue.poll(q)
		must(() {
			queue.offer(q, 1)
			queue.offer(q, 2)
		})
		first = queue.poll(q)
		left = queue.size(q)
		(empty, first, left, queue.poll(q), queue.poll(q), queue.size(q))
	})
	assert(got == (None(), Some(1), 1, Some(2), None(), 0))
	()
})

// The example in the module introduction.
test("queue_module_example_test", () {
	got = task.run(() {
		task.scope((s) {
			resource.scoped(() {
				q : queue.Queue(Int, Never) = fail.catch(() {
					queue.bounded(2)
				}) |>
					(Ok(q)) { q }
					& (Error(_)) { panic("capacity") }
				producer = task.go(s, () {
					fail.catch(() {
						queue.offer(q, 1)
						queue.offer(q, 2)
						queue.offer(q, 3)
					})
					queue.end(q)
				})
				first = queue.take(q)
				second = queue.take(q)
				third = queue.take(q)
				task.join(producer)
				[first, second, third, queue.take(q)]
			})
		})
	})
	assert(got == [Some(1), Some(2), Some(3), None()])
	()
})
