//// Counting semaphores: first-come, first-served admission of task bodies
//// with a release that runs exactly once.
////
//// ```i
//// resource.scoped(() {
//// 	s = semaphore.new(2)
//// 	semaphore.with_permit(s, () { fetch_one() })
//// })
//// ```
////
//// Ownership. `new` keeps the semaphore's state in a `task.Ref`, so the
//// semaphore belongs to the task scope (or run) where it was made, and every
//// operation needs that run's `task.Scheduler`. The state is one immutable
//// `SemaphoreState`; every change is one pure, atomic `task.update`. There is
//// no other hidden state and no helper task.
////
//// FIFO tickets. A waiter first takes the next ticket and joins the end of
//// `waiting`. It is admitted only when its ticket is the head of the queue
//// and enough permits are free, and the check, the removal of the ticket and
//// the taking of the permits are one atomic update. A later, smaller request
//// never jumps an earlier one, even when there are enough permits for it. A
//// waiter that is not admitted calls `task.yield()` and tries again in the
//// next scheduler turn.
////
//// Release once. From the moment the ticket is taken, the wait and the body
//// run inside one `resource.ensure`. Its cleanup is a single atomic update:
//// if the ticket is still waiting, it is removed and no permit is touched;
//// otherwise the waiter was admitted and its permits are given back. So the
//// permits come back exactly once whether the body returns, raises or is
//// cancelled, and a waiter cancelled before admission takes nothing and stops
//// blocking the waiters behind it.
////
//// Stability: experimental. Since: next. Tracking: docs/design/std-v3.md.
(
	i/fail
	i/list
	i/resource
	i/task
)

(
	Semaphore
	SemaphoreState
	SemaphoreError(..)
	new
	with_permits
	with_permit
	available
)

/// A counting semaphore made by `new`. Its constructor is private.
Semaphore = (state = task.Ref(SemaphoreState))

/// The value kept in a semaphore's ref. `capacity` never changes; `permits`
/// is the number free now, from 0 to `capacity`; `waiting` holds the tickets
/// of the waiters not yet admitted, oldest first; `next` is the next ticket.
SemaphoreState = (
	capacity = Int,
	permits = Int,
	waiting = List(Int),
	next = Int,
)

/// `new` was given a count below 1.
InvalidPermits = (wanted = Int)

/// `with_permits` asked for fewer than 0 or more than `capacity` permits.
TooMany = (wanted = Int)

/// The input errors of `new` and `with_permits`.
SemaphoreError = InvalidPermits | TooMany

/// A semaphore with `permits` permits, all free. It belongs to the current
/// task scope.
/// Needs: `resource.Scope`, `task.Scheduler`.
/// Errors: `Fail(InvalidPermits(wanted = permits))` when `permits < 1`.
///
/// ```i
/// example = () {
/// 	task.run(() {
/// 		resource.scoped(() {
/// 			s = semaphore.new(3)
/// 			assert(semaphore.available(s) == 3)
/// 		})
/// 	})
/// }
/// ```
new = (permits : Int) <- (
	_scope : resource.Scope,
	own : task.Scheduler,
) -> Semaphore {
	{ permits < 1 } |>
		(True) { fail.raise(InvalidPermits(wanted = permits)) }
		& (False) { () }
	Semaphore(
		state = task.ref(
			SemaphoreState(
				capacity = permits,
				permits = permits,
				waiting = [],
				next = 0,
			),
		),
	)
}

/// Waits for `n` permits in FIFO order, runs `body` while holding them, gives
/// them back exactly once when `body` ends in any way, and gives `body`'s
/// value. `n == 0` runs `body` at once and takes nothing. Cancelling a call
/// that is still waiting removes it from the queue without taking a permit.
/// Needs: `body`'s needs and `task.Scheduler`.
/// Errors: `Fail(TooMany(wanted = n))` when `n < 0` or `n > capacity`, before
/// waiting; otherwise `body`'s errors, after the permits are back.
///
/// ```i
/// example = () {
/// 	task.run(() {
/// 		resource.scoped(() {
/// 			s = semaphore.new(3)
/// 			inside = semaphore.with_permits(s, 2, () { semaphore.available(s) })
/// 			assert(inside == 1)
/// 			assert(semaphore.available(s) == 3)
/// 		})
/// 	})
/// }
/// ```
with_permits = <A, N>(s : Semaphore, n : Int, body : () <- N -> A) <- (
	own : task.Scheduler,
) -> A {
	capacity = task.get(s.state).capacity
	{ n < 0 || n > capacity } |>
		(True) { fail.raise(TooMany(wanted = n)) }
		& (False) { () }
	{ n == 0 } |> (True) { body() } & (False) { hold(s, n, body) }
}

/// `with_permits(s, 1, body)`. One permit is always a valid request, so it
/// has no `SemaphoreError`.
/// Needs: `body`'s needs and `task.Scheduler`. Errors: `body`'s.
///
/// ```i
/// example = () {
/// 	task.run(() {
/// 		resource.scoped(() {
/// 			s = semaphore.new(1)
/// 			assert(semaphore.with_permit(s, () { semaphore.available(s) }) == 0)
/// 		})
/// 	})
/// }
/// ```
with_permit = <A, N>(s : Semaphore, body : () <- N -> A) <- (
	own : task.Scheduler,
) -> A { hold(s, 1, body) }

/// The number of permits free now. Only for observation: another task may
/// take them before this one acts.
/// Needs: `task.Scheduler`. Errors: none.
///
/// ```i
/// example = () {
/// 	task.run(() {
/// 		resource.scoped(() { assert(semaphore.available(semaphore.new(2)) == 2) })
/// 	})
/// }
/// ```
available = (s : Semaphore) <- (own : task.Scheduler) -> Int {
	task.get(s.state).permits
}

// Takes a ticket, then waits and runs `body` under one `ensure` whose
// cleanup either withdraws the ticket or gives the `n` permits back. Nothing
// asks the scheduler between the ticket update and entering `ensure`, so a
// cancel cannot come between them and leave the ticket behind.
hold = <A, N>(s : Semaphore, n : Int, body : () <- N -> A) <- (
	own : task.Scheduler,
) -> A {
	ticket = task.update(s.state, (st : SemaphoreState) {
		(
			SemaphoreState(
				..st,
				waiting = list.append(st.waiting, [st.next]),
				next = st.next + 1,
			),
			st.next,
		)
	})
	resource.ensure(
		() {
			admit(s, ticket, n)
			body()
		},
		() {
			task.update(s.state, (st : SemaphoreState) {
				(leave(st, ticket, n), ())
			})
		},
	)
}

// Polls once per scheduler turn until `ticket` is admitted.
admit = (s : Semaphore, ticket : Int, n : Int) <- (own : task.Scheduler) -> () {
	admitted = task.update(s.state, (st : SemaphoreState) {
		take(st, ticket, n)
	})
	{ admitted } |>
		(True) { () }
		& (False) {
			task.yield()
			admit(s, ticket, n)
		}
}

// Admits `ticket` when it heads the queue and `n` permits are free.
take = (st : SemaphoreState, ticket : Int, n : Int) -> (SemaphoreState, Bool) {
	st.waiting |>
		([head, ..rest]) {
			{ head == ticket && st.permits >= n } |>
				(True) {
					(
						SemaphoreState(
							..st,
							permits = st.permits - n,
							waiting = rest,
						),
						True,
					)
				}
				& (False) { (st, False) }
		}
		& ([]) { (st, False) }
}

// The cleanup step. A ticket still in the queue was never admitted: remove
// it and touch no permit. Otherwise the caller holds `n` permits; give them
// back. Admission removed the ticket in the same update that took the
// permits, so the two cases cannot overlap, and the count can only go above
// `capacity` through a bug here, which stops the task.
leave = (st : SemaphoreState, ticket : Int, n : Int) -> SemaphoreState {
	{ list.contains(st.waiting, ticket) } |>
		(True) {
			SemaphoreState(
				..st,
				waiting = list.filter(st.waiting, (t : Int) { t != ticket }),
			)
		}
		& (False) {
			permits = st.permits + n
			{ permits > st.capacity } |>
				(True) {
					panic("semaphore: released more permits than capacity")
				}
				& (False) { SemaphoreState(..st, permits = permits) }
		}
}
