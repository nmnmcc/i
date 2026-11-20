(
	testlib = (test) = i/test
	i/fail
	i/list
	i/resource
	i/semaphore
	i/task
)

// Gives the other tasks several scheduler turns.
settle = () <- (own : task.Scheduler) -> () {
	go = (n : Int) {
		{ n == 0 } |>
			(True) { () }
			& (False) {
				task.yield()
				go(n - 1)
			}
	}
	go(20)
}

// Appends `x` to the log in `r`.
note = (r : task.Ref(List(Int)), x : Int) <- (own : task.Scheduler) -> () {
	task.update(r, (xs : List(Int)) { (list.append(xs, [x]), ()) })
}

test("zero_permits_run_immediately_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(1)
			inside = semaphore.with_permit(s, () {
				// All permits are held, yet zero permits do not wait.
				semaphore.with_permits(s, 0, () { semaphore.available(s) })
			})
			assert(inside == 0)
			assert(semaphore.available(s) == 1)
		})
	})
	()
})

test("free_permits_run_body_and_come_back_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(3)
			assert(semaphore.available(s) == 3)
			assert(semaphore.with_permit(s, () { semaphore.available(s) }) == 2)
			assert(semaphore.available(s) == 3)
			assert(
				semaphore.with_permits(s, 3, () { semaphore.available(s) }) ==
					0,
			)
			assert(semaphore.available(s) == 3)
			r = fail.catch(() {
				semaphore.with_permits(s, 2, () {
					assert(semaphore.available(s) == 1)
					fail.raise("body")
				})
			})
			assert(r == Error("body"))
			assert(semaphore.available(s) == 3)
		})
	})
	()
})

test("acquire_waits_until_release_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(1)
			log = task.ref([])
			task.scope((g) {
				held : task.Channel(()) = task.channel()
				done : task.Channel(()) = task.channel()
				holder = task.go(g, () {
					semaphore.with_permit(s, () {
						task.send(held, ())
						task.receive(done)
					})
				})
				task.receive(held)
				waiter = task.go(g, () {
					semaphore.with_permit(s, () {
						note(log, 2)
						semaphore.available(s)
					})
				})
				settle()
				assert(task.get(log) == [])
				assert(semaphore.available(s) == 0)
				task.send(done, ())
				task.join(holder)
				assert(task.join(waiter) == 0)
				assert(task.get(log) == [2])
			})
			assert(semaphore.available(s) == 1)
		})
	})
	()
})

Entry = (now = Int, peak = Int)

// Counts the bodies inside the semaphore and remembers the most at once.
enter = (r : task.Ref(Entry)) <- (own : task.Scheduler) -> () {
	task.update(r, (e : Entry) {
		now = e.now + 1
		{ e.peak > now } |>
			(True) { (Entry(now = now, peak = e.peak), ()) }
			& (False) { (Entry(now = now, peak = now), ()) }
	})
}

leave = (r : task.Ref(Entry)) <- (own : task.Scheduler) -> () {
	task.update(r, (e : Entry) { (Entry(..e, now = e.now - 1), ()) })
}

test("capacity_holds_under_contention_test", () {
	got = task.run(() {
		resource.scoped(() {
			s = semaphore.new(2)
			r = task.ref(Entry(now = 0, peak = 0))
			body = () {
				enter(r)
				settle()
				leave(r)
			}
			task.scope((g) {
				a = task.go(g, () { semaphore.with_permit(s, body) })
				b = task.go(g, () { semaphore.with_permit(s, body) })
				c = task.go(g, () { semaphore.with_permit(s, body) })
				task.join(a)
				task.join(b)
				task.join(c)
			})
			e = task.get(r)
			(e.now, e.peak, semaphore.available(s))
		})
	})
	assert(got == (0, 2, 2))
	()
})

test("waiters_are_admitted_in_ticket_order_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(2)
			log = task.ref([])
			task.scope((g) {
				held : task.Channel(()) = task.channel()
				done : task.Channel(()) = task.channel()
				holder = task.go(g, () {
					semaphore.with_permit(s, () {
						task.send(held, ())
						task.receive(done)
					})
				})
				task.receive(held)
				// One permit is free, but the first waiter wants two; the
				// later waiters wanting one must queue behind it.
				first = task.go(g, () {
					semaphore.with_permits(s, 2, () { note(log, 1) })
				})
				settle()
				second = task.go(g, () {
					semaphore.with_permit(s, () { note(log, 2) })
				})
				settle()
				third = task.go(g, () {
					semaphore.with_permit(s, () { note(log, 3) })
				})
				settle()
				assert(task.get(log) == [])
				assert(semaphore.available(s) == 1)
				task.send(done, ())
				task.join(holder)
				task.join(first)
				task.join(second)
				task.join(third)
				assert(task.get(log) == [1, 2, 3])
			})
			assert(semaphore.available(s) == 2)
		})
	})
	()
})

test("cancelled_waiter_leaves_the_queue_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(1)
			log = task.ref([])
			task.scope((g) {
				held : task.Channel(()) = task.channel()
				done : task.Channel(()) = task.channel()
				holder = task.go(g, () {
					semaphore.with_permit(s, () {
						task.send(held, ())
						task.receive(done)
					})
				})
				task.receive(held)
				// The waiter queues first, then its scope fails and cancels it.
				r = fail.catch(() {
					task.scope((inner) {
						task.go(inner, () {
							semaphore.with_permit(s, () { note(log, 1) })
						})
						settle()
						fail.raise("stop")
					})
				})
				assert(r == Error("stop"))
				assert(semaphore.available(s) == 0)
				later = task.go(g, () {
					semaphore.with_permit(s, () { note(log, 2) })
				})
				settle()
				task.send(done, ())
				task.join(holder)
				settle()
				// The cancelled ticket does not block the later waiter.
				assert(task.get(log) == [2])
				task.join(later)
			})
			assert(task.get(log) == [2])
			assert(semaphore.available(s) == 1)
		})
	})
	()
})

test("race_loser_waiting_takes_no_permit_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(2)
			log = task.ref([])
			task.scope((g) {
				held : task.Channel(()) = task.channel()
				done : task.Channel(()) = task.channel()
				holder = task.go(g, () {
					semaphore.with_permits(s, 2, () {
						task.send(held, ())
						task.receive(done)
					})
				})
				task.receive(held)
				winner = task.race(
					() {
						semaphore.with_permit(s, () { note(log, 1) })
						1
					},
					() {
						settle()
						2
					},
				)
				assert(winner == 2)
				task.send(done, ())
				task.join(holder)
				assert(semaphore.available(s) == 2)
				semaphore.with_permits(s, 2, () { note(log, 3) })
			})
			assert(task.get(log) == [3])
			assert(semaphore.available(s) == 2)
		})
	})
	()
})

test("invalid_counts_raise_test", () {
	task.run(() {
		resource.scoped(() {
			zero = fail.catch(() {
				semaphore.new(0)
				()
			})
			assert(zero == Error(semaphore.InvalidPermits(wanted = 0)))
			negative = fail.catch(() {
				semaphore.new(-2)
				()
			})
			assert(negative == Error(semaphore.InvalidPermits(wanted = -2)))
			s = semaphore.new(1)
			ran = task.ref([])
			over = fail.catch(() {
				semaphore.with_permits(s, 2, () { note(ran, 1) })
			})
			assert(over == Error(semaphore.InvalidPermits(wanted = 2)))
			under = fail.catch(() {
				semaphore.with_permits(s, -1, () { note(ran, 1) })
			})
			assert(under == Error(semaphore.InvalidPermits(wanted = -1)))
			assert(task.get(ran) == [])
			assert(semaphore.available(s) == 1)
		})
	})
	()
})

test("raising_body_releases_once_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(2)
			outer = semaphore.with_permit(s, () {
				r = fail.catch(() {
					semaphore.with_permit(s, () {
						assert(semaphore.available(s) == 0)
						fail.raise("body")
					})
				})
				assert(r == Error("body"))
				// One release, not two: the outer permit is still held.
				semaphore.available(s)
			})
			assert(outer == 1)
			assert(semaphore.available(s) == 2)
		})
	})
	()
})

test("cancelled_holder_releases_once_test", () {
	task.run(() {
		resource.scoped(() {
			s = semaphore.new(3)
			r = fail.catch(() {
				task.scope((g) {
					held : task.Channel(()) = task.channel()
					task.go(g, () {
						semaphore.with_permits(s, 2, () {
							task.send(held, ())
							task.sync(task.never())
						})
					})
					task.receive(held)
					assert(semaphore.available(s) == 1)
					fail.raise("stop")
				})
			})
			assert(r == Error("stop"))
			assert(semaphore.available(s) == 3)
			// All three are usable again, so nothing was given back twice.
			assert(
				semaphore.with_permits(s, 3, () { semaphore.available(s) }) ==
					0,
			)
		})
	})
	()
})
