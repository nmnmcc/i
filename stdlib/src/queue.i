//// First-in, first-out handoff between tasks, bounded or unbounded.
////
//// Make a queue inside a `resource.scoped` body that runs inside a task
//// scope, give it to the tasks that share the work, `offer` values on one
//// side and `take` them on the other:
////
//// ```i
//// example = () {
//// 	got = task.run(() {
//// 		task.scope((s) {
//// 			resource.scoped(() {
//// 				q : queue.Queue(Int, Never) = fail.catch(() { queue.bounded(2) }) |>
//// 					(Ok(q)) { q }
//// 					& (Error(_)) { panic("capacity") }
//// 				producer = task.go(s, () {
//// 					fail.catch(() {
//// 						queue.offer(q, 1)
//// 						queue.offer(q, 2)
//// 						queue.offer(q, 3)
//// 					})
//// 					queue.end(q)
//// 				})
//// 				first = queue.take(q)
//// 				second = queue.take(q)
//// 				third = queue.take(q)
//// 				task.join(producer)
//// 				[first, second, third, queue.take(q)]
//// 			})
//// 		})
//// 	})
//// 	assert(got == [Some(1), Some(2), Some(3), None])
//// }
//// ```
////
//// State. A queue is one immutable `QueueState` value kept in a `task.Ref`.
//// The ref belongs to the task scope that is current when the queue is made,
//// and the queue is attached to the current `resource.Scope`: when that
//// resource scope ends, the queue is shut down. Make the queue inside its
//// task scope, so that the resource scope ends first. Every operation is one
//// or more pure `task.update` calls; there is no other hidden state. Stored
//// values must be portable, like every `task.Ref` value.
////
//// Order. Waiting takes and waiting offers each hold a ticket, given out in
//// call order. Only the first ticket of its line may act, and its check and
//// its change happen in one atomic update, so values leave in the order they
//// arrived and waiting takers are served in the order they came. `poll`
//// respects that line too: while a taker waits it gives `None` and leaves
//// the value for the taker. A waiter checks once per scheduler turn and
//// yields with `task.yield()` between checks. Only `bounded` makes an
//// `offer` wait; `dropping`, `sliding` and `unbounded` offers never wait.
////
//// Terminal state. A queue is open until `end`, `fail` or `shutdown`. `end`
//// and `fail` act only on an open queue, so the first of them wins. Both
//// refuse new offers with `Fail(Closed)` and let takers drain the buffered
//// values; after the last value, `take` and `poll` give `None` (after `end`)
//// or raise the error (after `fail`), every time. `shutdown` always acts,
//// also after `end` or `fail`: it discards the buffer, waiting and later
//// takes give `None`, and waiting and later offers raise `Fail(Closed)`.
////
//// Cancellation. A waiting `take` or `offer` that is cancelled removes its
//// ticket on the way out, so the next waiter becomes first. A take removes
//// its value and its ticket in the same update, and an offer inserts its
//// value and removes its ticket in the same update, so a cancel never loses
//// or duplicates a value.
////
//// `size` is for observation only: do not check it and then take, since
//// another task can act in between. Use `poll` for a take that does not wait.
////
//// Stability: experimental. Tracking: docs/design/std-v3.md §3.5.
(
	failure = i/fail
	i/core
	(Option, Some, None) = i/option
	i/resource
	sequence = (Sequence) = i/sequence
	i/task
)

(
	Queue
	QueueError(..)
	bounded
	dropping
	sliding
	unbounded
	offer
	take
	poll
	size
	end
	fail
	shutdown
)

/// A queue of `A` values that can end with an error `E`. Made by `bounded`,
/// `dropping`, `sliding` or `unbounded`; used through the functions of this
/// module.
Queue(A, E) = (state = task.Ref(QueueState(A, E)), strategy = Strategy)

/// What happens to an offer when a bounded queue is full.
Strategy = Backpressure | Dropping | Sliding

Backpressure = ()

Dropping = ()

Sliding = ()

/// The whole state of one queue. `capacity` is `None` for an unbounded
/// queue. `takers` and `offers` are the waiting tickets, first in line
/// first; `next` is the next ticket number.
QueueState(A, E) = (
	capacity = Option(Int),
	buffer = Sequence(A),
	takers = List(Int),
	offers = List(Int),
	next = Int,
	terminal = Terminal(E),
)

Terminal(E) = Open | Ended | Failed(E) | Shutdown

Open = ()

Ended = ()

Failed(E) = (E)

Shutdown = ()

/// An error from making a queue or from offering to it.
QueueError = InvalidCapacity | Closed

/// A capacity that is not positive was given to a constructor.
InvalidCapacity = (capacity = Int)

/// The queue has ended, failed or shut down, and takes no more offers.
Closed = ()

// Answers of the atomic steps.
OfferStep = OfferDone | OfferClosed | OfferWait

OfferDone = (Bool)

OfferClosed = ()

OfferWait = (Int)

TakeStep(A, E) = TakeDone(A) | TakeFailed(E) | TakeWait

TakeDone(A) = (Option(A))

TakeFailed(E) = (E)

TakeWait = (Int)

without = (tickets : List(Int), ticket : Int) -> List(Int) {
	tickets |>
		([]) { [] }
		& ([t, ..rest]) {
			{ t == ticket } |>
				(True) { rest }
				& (False) { [t, ..without(rest, ticket)] }
		}
}

append_ticket = (tickets : List(Int), ticket : Int) -> List(Int) {
	tickets |>
		([]) { [ticket] }
		& ([t, ..rest]) { [t, ..append_ticket(rest, ticket)] }
}

is_head = (tickets : List(Int), ticket : Int) -> Bool {
	tickets |> ([t, .._]) { t == ticket } & ([]) { False }
}

is_open = <E>(terminal : Terminal(E)) -> Bool {
	terminal |> (Open) { True } & (_) { False }
}

has_room = <A, E>(state : QueueState(A, E)) -> Bool {
	state.capacity |>
		(None) { True }
		& (Some(n)) { sequence.length(state.buffer) < n }
}

make = <A, E>(capacity : Option(Int), strategy : Strategy) <- (
	scope : resource.Scope,
	own : task.Scheduler,
) -> Queue(A, E) {
	resource.acquire(
		() {
			Queue(
				state = task.ref(
					QueueState(
						capacity = capacity,
						buffer = sequence.new(),
						takers = [],
						offers = [],
						next = 0,
						terminal = Open,
					),
				),
				strategy = strategy,
			)
		},
		(q : Queue(A, E)) { shutdown(q) },
	)
}

make_bounded = <A, E>(capacity : Int, strategy : Strategy) <- (
	scope : resource.Scope,
	own : task.Scheduler,
	error : Fail(QueueError),
) -> Queue(A, E) {
	{ capacity > 0 } |>
		(True) { make(Some(capacity), strategy) }
		& (False) { failure.raise(InvalidCapacity(capacity = capacity)) }
}

/// A queue that holds at most `capacity` values. When it is full, `offer`
/// waits until a take makes room, in offer order.
/// Needs: the current resource scope and task scheduler. The queue is shut
/// down when the resource scope ends.
/// Errors: `InvalidCapacity` when `capacity` is not positive.
/// Example: fail.catch(() { queue.bounded(0) }) == Error(queue.InvalidCapacity(capacity = 0)) inside a resource scope.
bounded = <A, E>(capacity : Int) <- (
	scope : resource.Scope,
	own : task.Scheduler,
	error : Fail(QueueError),
) -> Queue(A, E) { make_bounded(capacity, Backpressure) }

/// A queue that holds at most `capacity` values. When it is full, `offer`
/// gives `False` at once and keeps the queue as it was.
/// Needs: the current resource scope and task scheduler. The queue is shut
/// down when the resource scope ends.
/// Errors: `InvalidCapacity` when `capacity` is not positive.
/// Example: after offering 1 to `dropping(1)`, offering 2 gives False and take gives Some(1).
dropping = <A, E>(capacity : Int) <- (
	scope : resource.Scope,
	own : task.Scheduler,
	error : Fail(QueueError),
) -> Queue(A, E) { make_bounded(capacity, Dropping) }

/// A queue that holds at most `capacity` values. When it is full, `offer`
/// removes the oldest buffered value, adds the new one and gives `True`.
/// Needs: the current resource scope and task scheduler. The queue is shut
/// down when the resource scope ends.
/// Errors: `InvalidCapacity` when `capacity` is not positive.
/// Example: after offering 1 and 2 to `sliding(1)`, take gives Some(2).
sliding = <A, E>(capacity : Int) <- (
	scope : resource.Scope,
	own : task.Scheduler,
	error : Fail(QueueError),
) -> Queue(A, E) { make_bounded(capacity, Sliding) }

/// A queue with no capacity limit: `offer` never waits and always gives
/// `True` while the queue is open.
/// Needs: the current resource scope and task scheduler. The queue is shut
/// down when the resource scope ends.
/// Errors: none.
/// Example: after offering 1, 2 and 3 to `unbounded()`, size gives 3.
unbounded = <A, E>() <- (scope : resource.Scope, own : task.Scheduler) -> Queue(
	A,
	E,
) { make(None, Backpressure) }

// Inserts `value` into an open state that may act now. Gives the new state
// and the offer's answer.
insert = <A, E>(state : QueueState(A, E), strategy : Strategy, value : A) -> (
	QueueState(A, E),
	Bool,
) {
	{ has_room(state) } |>
		(True) {
			(
				QueueState(
					..state,
					buffer = sequence.push_back(state.buffer, value),
				),
				True,
			)
		}
		& (False) {
			strategy |>
				(Dropping) { (state, False) }
				& (Sliding) {
					rest = sequence.pop_front(state.buffer) |>
						(Some((_, rest))) { rest }
						& (None) { state.buffer }
					(
						QueueState(
							..state,
							buffer = sequence.push_back(rest, value),
						),
						True,
					)
				}
				& (Backpressure) { panic("queue: insert into a full queue") }
		}
}

offer_first = <A, E>(
	state : QueueState(A, E),
	strategy : Strategy,
	value : A
) -> (QueueState(A, E), OfferStep) {
	{ is_open(state.terminal) } |>
		(False) { (state, OfferClosed) }
		& (True) {
			waits = strategy |>
				(Backpressure) {
					state.offers != [] || core.not(has_room(state))
				}
				& (_) { False }
			waits |>
				(False) {
					(next, accepted) = insert(state, strategy, value)
					(next, OfferDone(accepted))
				}
				& (True) {
					ticket = state.next
					(
						QueueState(
							..state,
							offers = append_ticket(state.offers, ticket),
							next = ticket + 1,
						),
						OfferWait(ticket),
					)
				}
		}
}

offer_again = <A, E>(
	state : QueueState(A, E),
	strategy : Strategy,
	ticket : Int,
	value : A
) -> (QueueState(A, E), OfferStep) {
	{ is_open(state.terminal) } |>
		(False) {
			(
				QueueState(..state, offers = without(state.offers, ticket)),
				OfferClosed,
			)
		}
		& (True) {
			{ is_head(state.offers, ticket) && has_room(state) } |>
				(False) { (state, OfferWait(ticket)) }
				& (True) {
					(inserted, accepted) = insert(
						QueueState(
							..state,
							offers = without(state.offers, ticket),
						),
						strategy,
						value,
					)
					(inserted, OfferDone(accepted))
				}
		}
}

offer_wait = <A, E>(q : Queue(A, E), ticket : Int, value : A) <- (
	own : task.Scheduler,
) -> OfferStep {
	task.yield()
	task.update(q.state, (state : QueueState(A, E)) {
		offer_again(state, q.strategy, ticket, value)
	}) |>
		(OfferWait(_)) { offer_wait(q, ticket, value) }
		& (done) { done }
}

forget_offer = <A, E>(q : Queue(A, E), ticket : Int) <- (
	own : task.Scheduler,
) -> () {
	task.update(q.state, (state : QueueState(A, E)) {
		(QueueState(..state, offers = without(state.offers, ticket)), ())
	})
}

/// Adds `value` at the back of the queue. A `bounded` queue that is full
/// makes the offer wait, in offer order, until a take makes room; it then
/// gives `True`. A full `dropping` queue gives `False` and keeps the value
/// out. A full `sliding` queue drops its oldest value and gives `True`. An
/// `unbounded` queue always gives `True`.
/// Needs: the task scheduler. A waiting offer that is cancelled leaves the
/// queue without inserting its value.
/// Errors: `Closed` after `end`, `fail` or `shutdown`, also for an offer
/// that was waiting when the queue closed.
/// Example: queue.offer(q, 1) == True for an open `unbounded` queue.
offer = <A, E>(q : Queue(A, E), value : A) <- (
	own : task.Scheduler,
	error : Fail(QueueError),
) -> Bool {
	step = task.update(q.state, (state : QueueState(A, E)) {
		offer_first(state, q.strategy, value)
	}) |>
		(OfferWait(ticket)) {
			resource.ensure(
				() { offer_wait(q, ticket, value) },
				() { forget_offer(q, ticket) },
			)
		}
		& (other) { other }
	step |>
		(OfferDone(accepted)) { accepted }
		& (OfferClosed) { failure.raise(Closed) }
		& (OfferWait(_)) { panic("queue: offer still waiting") }
}

// The answer for a take that is first in line and finds the buffer empty.
empty_answer = <A, E>(state : QueueState(A, E), ticket : Int) -> (
	QueueState(A, E),
	TakeStep(A, E),
) {
	state.terminal |>
		(Open) { (state, TakeWait(ticket)) }
		& (Ended | Shutdown) {
			(
				QueueState(..state, takers = without(state.takers, ticket)),
				TakeDone(None),
			)
		}
		& (Failed(e)) {
			(
				QueueState(..state, takers = without(state.takers, ticket)),
				TakeFailed(e),
			)
		}
}

// Pops one value from the state, or answers as `empty_answer` does when the
// buffer is empty. `ticket` is removed from the takers in either case unless
// the take must wait.
take_now = <A, E>(state : QueueState(A, E), ticket : Int) -> (
	QueueState(A, E),
	TakeStep(A, E),
) {
	sequence.pop_front(state.buffer) |>
		(Some((value, rest))) {
			(
				QueueState(
					..state,
					buffer = rest,
					takers = without(state.takers, ticket),
				),
				TakeDone(Some(value)),
			)
		}
		& (None) { empty_answer(state, ticket) }
}

take_first = <A, E>(state : QueueState(A, E)) -> (
	QueueState(A, E),
	TakeStep(A, E),
) {
	ticket = state.next
	{ state.takers == [] } |>
		(True) {
			(next, step) = take_now(state, ticket)
			step |>
				(TakeWait(_)) {
					(
						QueueState(
							..next,
							takers = [ticket],
							next = ticket + 1,
						),
						step,
					)
				}
				& (_) { (next, step) }
		}
		& (False) {
			(
				QueueState(
					..state,
					takers = append_ticket(state.takers, ticket),
					next = ticket + 1,
				),
				TakeWait(ticket),
			)
		}
}

take_again = <A, E>(state : QueueState(A, E), ticket : Int) -> (
	QueueState(A, E),
	TakeStep(A, E),
) {
	{ is_head(state.takers, ticket) } |>
		(True) { take_now(state, ticket) }
		& (False) { (state, TakeWait(ticket)) }
}

take_wait = <A, E>(q : Queue(A, E), ticket : Int) <- (
	own : task.Scheduler,
) -> TakeStep(A, E) {
	task.yield()
	task.update(q.state, (state : QueueState(A, E)) {
		take_again(state, ticket)
	}) |>
		(TakeWait(_)) { take_wait(q, ticket) }
		& (done) { done }
}

forget_take = <A, E>(q : Queue(A, E), ticket : Int) <- (
	own : task.Scheduler,
) -> () {
	task.update(q.state, (state : QueueState(A, E)) {
		(QueueState(..state, takers = without(state.takers, ticket)), ())
	})
}

finish_take = <A, E>(step : TakeStep(A, E)) <- (error : Fail(E)) -> Option(A) {
	step |>
		(TakeDone(value)) { value }
		& (TakeFailed(e)) { failure.raise(e) }
		& (TakeWait(_)) { panic("queue: take still waiting") }
}

/// Removes and gives the oldest value, waiting while the queue is open and
/// empty. Waiting takes are served in the order they started. Gives `None`
/// once the queue has ended and is drained, or has shut down.
/// Needs: the task scheduler. A waiting take that is cancelled leaves the
/// line and takes no value.
/// Errors: the queue's error `E` once a failed queue is drained, on this and
/// every later take.
/// Example: after offering 1 and 2, queue.take(q) == Some(1).
take = <A, E>(q : Queue(A, E)) <- (
	own : task.Scheduler,
	error : Fail(E),
) -> Option(A) {
	step = task.update(q.state, take_first) |>
		(TakeWait(ticket)) {
			resource.ensure(
				() { take_wait(q, ticket) },
				() { forget_take(q, ticket) },
			)
		}
		& (other) { other }
	finish_take(step)
}

/// Removes and gives the oldest value when there is one, without waiting.
/// Gives `None` when the queue is empty and open, ended or shut down, and
/// when a taker is already waiting: the poll does not overtake it, and the
/// value stays for that taker. Takes at most one value.
/// Needs: the task scheduler.
/// Errors: the queue's error `E` when a failed queue is drained.
/// Example: queue.poll(q) == None for a new queue.
poll = <A, E>(q : Queue(A, E)) <- (
	own : task.Scheduler,
	error : Fail(E),
) -> Option(A) {
	step = task.update(q.state, (state : QueueState(A, E)) {
		// A waiting taker is first in line; a poll never takes its value.
		{ state.takers != [] } |>
			(True) { nothing_to_take(state) }
			& (False) {
				sequence.pop_front(state.buffer) |>
					(Some((value, rest))) {
						(
							QueueState(..state, buffer = rest),
							TakeDone(Some(value)),
						)
					}
					& (None) { nothing_to_take(state) }
			}
	})
	finish_take(step)
}

// The answer when nothing can be taken now: raise on a drained failure,
// otherwise give `None`.
nothing_to_take = <A, E>(state : QueueState(A, E)) -> (
	QueueState(A, E),
	TakeStep(A, E),
) {
	state.terminal |>
		(Failed(e)) { (state, TakeFailed(e)) }
		& (_) { (state, TakeDone(None)) }
}

/// The number of buffered values. For observation only: another task may
/// take or offer before the next step.
/// Needs: the task scheduler. Errors: none.
/// Example: queue.size(q) == 0 for a new queue.
size = <A, E>(q : Queue(A, E)) <- (own : task.Scheduler) -> Int {
	state : QueueState(A, E) = task.get(q.state)
	sequence.length(state.buffer)
}

/// Ends an open queue: new offers raise `Closed`, takes drain the buffered
/// values and then give `None`. Does nothing when the queue has already
/// ended, failed or shut down.
/// Needs: the task scheduler. Errors: none.
/// Example: after queue.end(q) on an empty queue, queue.take(q) == None.
end = <A, E>(q : Queue(A, E)) <- (own : task.Scheduler) -> () {
	task.update(q.state, (state : QueueState(A, E)) {
		{ is_open(state.terminal) } |>
			(True) { (QueueState(..state, terminal = Ended), ()) }
			& (False) { (state, ()) }
	})
}

/// Fails an open queue with `e`: new offers raise `Closed`, takes drain the
/// buffered values and then raise `e`. Does nothing when the queue has
/// already ended, failed or shut down.
/// Needs: the task scheduler. Errors: none.
/// Example: after queue.fail(q, "boom") on an empty queue, fail.catch(() { queue.take(q) }) == Error("boom").
fail = <A, E>(q : Queue(A, E), e : E) <- (own : task.Scheduler) -> () {
	task.update(q.state, (state : QueueState(A, E)) {
		{ is_open(state.terminal) } |>
			(True) { (QueueState(..state, terminal = Failed(e)), ()) }
			& (False) { (state, ()) }
	})
}

/// Shuts the queue down, also after `end` or `fail`: the buffered values are
/// discarded, waiting and later takes give `None`, and waiting and later
/// offers raise `Closed`. The end of the queue's resource scope calls this.
/// Needs: the task scheduler. Errors: none.
/// Example: after offering 1 and queue.shutdown(q), queue.take(q) == None.
shutdown = <A, E>(q : Queue(A, E)) <- (own : task.Scheduler) -> () {
	task.update(q.state, (state : QueueState(A, E)) {
		(QueueState(..state, buffer = sequence.new(), terminal = Shutdown), ())
	})
}
