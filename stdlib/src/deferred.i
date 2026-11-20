//// One completion, many readers: a `Deferred(A, E)` starts waiting, is
//// completed once with a `Result(A, E)`, and from then on every reader sees
//// that same outcome.
////
//// The state is a `task.Ref` made by `new`. It belongs to the scope where the
//// deferred is made (or to the run at its root) and is only reachable through
//// the handle. Every change goes through `task.update` with a pure callback,
//// so `complete` is atomic: the first completion wins and later ones change
//// nothing.
////
//// `await` has no event of its own. A waiter looks at the state once per
//// scheduler turn and calls `task.yield()` while it is still waiting. So a
//// waiter that is cancelled is cancelled inside `task.yield`, holding nothing:
//// cancelling one waiter never cancels the producer or the other waiters, and
//// leaves the deferred as it was.
////
//// ```i
//// task.run(() {
////   task.scope((group) {
////     d : deferred.Deferred(Int, String) = resource.scoped(() {
////       deferred.new()
////     })
////     reader = task.go(group, () { deferred.await(d) })
////     deferred.complete(d, Ok(42))
////     task.join(reader)  // 42
////   })
//// })
//// ```
(
	i/fail
	i/resource
	i/task
	option = (Option, Some, None) = i/option
)

(
	Deferred
	new
	complete
	poll
	is_done
	await
)

/// A completion that many readers wait for. Make one with `new`.
Deferred(A, E) = (state = task.Ref(DeferredState(A, E)))

DeferredState(A, E) = Waiting | Done(A, E)

Waiting = ()

Done(A, E) = (Result(A, E))

/// A new deferred that is waiting. Its state is a `task.Ref` owned by the
/// current task scope; the `resource.Scope` need is the capability to make
/// it. Needs `resource.Scope` and `task.Scheduler`; raises nothing.
///
/// ```i
/// task.run(() {
///   d : deferred.Deferred(Int, String) = resource.scoped(() {
///     deferred.new()
///   })
///   deferred.is_done(d)  // False
/// })
/// ```
new = <A, E>() <- (_scope : resource.Scope, own : task.Scheduler) -> Deferred(
	A,
	E,
) {
	initial : DeferredState(A, E) = Waiting
	Deferred(state = task.ref(initial))
}

/// Completes the deferred with `outcome`, atomically. The first completion
/// stores the outcome and gives `True`; a later one changes nothing and gives
/// `False`. Needs `task.Scheduler`; raises nothing.
///
/// ```i
/// task.run(() {
///   d : deferred.Deferred(Int, String) = resource.scoped(() {
///     deferred.new()
///   })
///   deferred.complete(d, Ok(1))  // True
///   deferred.complete(d, Ok(2))  // False
///   deferred.poll(d)             // Some(Ok(1))
/// })
/// ```
complete = <A, E>(d : Deferred(A, E), outcome : Result(A, E)) <- (
	own : task.Scheduler,
) -> Bool {
	task.update(d.state, (state : DeferredState(A, E)) {
		state |>
			(Waiting) {
				next : DeferredState(A, E) = Done(outcome)
				(next, True)
			}
			& (Done(_)) { (state, False) }
	})
}

/// The outcome: `Some(outcome)` once the deferred is completed and `None`
/// while it is waiting. Does not wait. Needs `task.Scheduler`; raises
/// nothing.
///
/// ```i
/// task.run(() {
///   d : deferred.Deferred(Int, String) = resource.scoped(() {
///     deferred.new()
///   })
///   deferred.poll(d)  // None
///   deferred.complete(d, Error("no"))
///   deferred.poll(d)  // Some(Error("no"))
/// })
/// ```
poll = <A, E>(d : Deferred(A, E)) <- (own : task.Scheduler) -> Option(
	Result(A, E),
) {
	task.get(d.state) |> (Waiting) { None } & (Done(outcome)) { Some(outcome) }
}

/// `True` once the deferred is completed. Needs `task.Scheduler`; raises
/// nothing.
///
/// ```i
/// task.run(() {
///   d : deferred.Deferred(Int, String) = resource.scoped(() {
///     deferred.new()
///   })
///   deferred.is_done(d)  // False
///   deferred.complete(d, Ok(1))
///   deferred.is_done(d)  // True
/// })
/// ```
is_done = <A, E>(d : Deferred(A, E)) <- (own : task.Scheduler) -> Bool {
	poll(d) |> (Some(_)) { True } & (None) { False }
}

/// Waits until the deferred is completed and gives its value, or raises its
/// error. While waiting it calls `task.yield()` once per scheduler turn, so a
/// cancel of this waiter lands in `task.yield` and touches nothing else.
/// Needs `task.Scheduler`; raises `E` through `Fail(E)` when the outcome is
/// `Error(E)`.
///
/// ```i
/// task.run(() {
///   d : deferred.Deferred(Int, String) = resource.scoped(() {
///     deferred.new()
///   })
///   deferred.complete(d, Error("no"))
///   fail.catch(() { deferred.await(d) })  // Error("no")
/// })
/// ```
await = <A, E>(d : Deferred(A, E)) <- (
	own : task.Scheduler,
	failure : Fail(E),
) -> A {
	poll(d) |>
		(Some(outcome)) { fail.from_result(outcome) }
		& (None) {
			task.yield()
			await(d)
		}
}
