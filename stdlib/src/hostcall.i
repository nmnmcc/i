//// Dispatcher surface for host calls: groups, wait modes and unwaited starts.
////
//// A package reaches this module only with `hostcall = true` in its own
//// `i.toml`. Most code never needs it: the library wrappers (`i/clock`,
//// `i/fs`, …) start and await their host calls themselves. A dispatcher uses
//// it to choose *where* a start belongs and *how* an await waits:
////
//// ```i
//// g = hostcall.group()
//// hostcall.in_group(g, () { clock.sleep_ns(10) })
//// hostcall.close(g)
//// ```
////
//// `in_group(g) { … }` makes `g` the group of every start in the body,
//// including starts inside library wrappers; `with_wait(w) { … }` makes `w`
//// the wait of every await in the body. Both nest, the innermost wins, and
//// both end with the body however it ends (they are provisions).
////
//// `call(op)` starts one direct field call of an imported service without
//// awaiting it and returns the `Call`; `await` takes its answer later.
////
//// Programmer errors panic with a `hostcall:` text (a closed group, a taken
//// or cancelled Call, a Call taken while still waiting). Host answer errors
//// still go through `Fail`.
////
//// Every function here is a thin wrapper over `i/request`; there is no second
//// implementation of the request table.
(
	req = i/request
	i/core
)

(
	Call
	CallGroup
	Wait(..)
	Root
	RequestState
	group
	group_at
	close
	closed
	in_group
	with_wait
	call
	call_in
	await
	state
	take
	cancel
	settled
	ticket
	adopt
	request
	root
	open_root
	close_root
	wait_any
	poll
)

/// An unwaited start under one group. Its constructor is private: a `Call`
/// is made by `call`, `call_in` or `adopt`.
Call(A) = Request(A)

/// A live group of starts, registered at one `Root`. Its constructor is
/// private: a group is made by `group` or `group_at` and ended by `close`.
CallGroup = req.Owner

/// A wait mode: `wait(ticket)` must return only once `ticket` is settled.
/// `with_wait` makes it the wait of every await in a body.
Wait = (wait = (Ticket) -> ())

/// The request accounting scope a group registers at (`i/request`).
Root = req.Root

/// What `state` reports of a `Call` (`i/request`).
RequestState(A) = req.RequestState(A)

// ---- Runtime operations not yet in `ilang:request` ----

/// Whether the group with this key is still open (not closed, and its Root
/// not closed).
@runtime("ilang:hostcall", "group_live")
group_live : (Key(())) -> Bool

/// Whether the Root with this key is still open.
@runtime("ilang:hostcall", "root_live")
root_live : (Key(())) -> Bool

/// Moves a request into the group with this key; a settled request only
/// changes its provenance.
@runtime("ilang:hostcall", "adopt")
move_to : <A>(Request(A), Key(())) -> Request(A)

// ---- Groups ----
retain = (value) { value }

/// Opens a group at the ambient current Root.
group = () <- (r : Root) -> CallGroup { group_at(r) }

/// Opens a group at `r`. Panics `hostcall: root closed` if `r` is closed.
group_at = (r : Root) -> CallGroup {
	root_live(r.key) |>
		(True()) { req.open_owner() <- (r) }
		& (False()) { panic("hostcall: root closed") }
}

/// Closes `g`: cancels its waiting starts and forbids later ones. Ready Calls
/// keep their answers. Closing twice does nothing.
close = (g : CallGroup) { req.close_owner(g) }

/// Whether `g` is closed, by `close` or by a close of its Root.
closed = (g : CallGroup) -> Bool { core.not(group_live(g.key)) }

open_group = (g : CallGroup) {
	closed(g) |> (True()) { panic("hostcall: group closed") } & (False()) { () }
}

// ---- Scopes ----

/// Runs `body` with `g` as the group of every start in it, and `g`'s Root as
/// its Root. Panics `hostcall: group closed` if `g` is already closed.
in_group = (g : CallGroup, body) {
	open_group(g)
	retain(body()) <- (g, g.root)
}

/// Runs `body` with `w` as the wait of every await in it.
with_wait = (w : Wait, body) { retain(body()) <- (req.Await(wait = w.wait)) }

// ---- Unwaited starts ----

/// Starts `op`, one direct field call of an `@import` service, in the
/// ambient group, without awaiting it. A fake (a closure that is not an
/// import field) is called now and its result is a ready Call.
@runtime("ilang:hostcall", "call")
call : <A, N>(() <- (..N, req.Await, req.Owner) -> A) <- (
	..N,
	req.Await,
	req.Owner,
) -> Call(A)

/// `call` in the group `g`.
@runtime("ilang:hostcall", "call_in")
call_in : <A, N>(CallGroup, () <- (..N, req.Await, req.Owner) -> A) <- (
	..N,
	req.Await,
	req.Owner,
) -> Call(A)

terminal = (state) {
	state |>
		(req.Cancelled()) { panic("hostcall: cancelled") }
		& (req.Taken()) { panic("hostcall: taken") }
		& (_) { () }
}

/// Waits for `c` with the ambient wait and takes its answer. Borrows the
/// start: unwinding the await does not cancel it; closing its group does.
/// Panics `hostcall: taken` / `hostcall: cancelled` on a terminal Call and
/// `hostcall: wait returned early` if the wait returns before `c` settles.
await = <A>(c : Call(A)) <- (waiter : req.Await) -> A {
	terminal(req.state(c))
	waiter.wait(req.ticket(c))
	req.state(c) |>
		(req.Waiting()) { panic("hostcall: wait returned early") }
		& (s) {
			terminal(s)
			req.take(c)
		}
}

/// The state of `c` now.
state = <A>(c : Call(A)) -> RequestState(A) { req.state(c) }

/// Takes the answer of a ready `c`. Panics `hostcall: not ready`,
/// `hostcall: taken` or `hostcall: cancelled` otherwise.
take = <A>(c : Call(A)) -> A {
	req.state(c) |>
		(req.Waiting()) { panic("hostcall: not ready") }
		& (s) {
			terminal(s)
			req.take(c)
		}
}

/// Cancels `c`. Cancelling a terminal Call does nothing.
cancel = <A>(c : Call(A)) { req.cancel(req.ticket(c)) }

/// Whether `c` is no longer waiting.
settled = <A>(c : Call(A)) -> Bool { req.settled(req.ticket(c)) }

/// The ticket of `c`, for `wait_any` and custom waits.
ticket = <A>(c : Call(A)) -> Ticket { req.ticket(c) }

/// Makes `r` a Call of the ambient group. Panics `hostcall: group closed` if
/// that group is closed.
adopt = <A>(r : Request(A)) <- (g : req.Owner) -> Call(A) {
	open_group(g)
	move_to(r, g.key)
}

/// The request cell of `c`.
request = <A>(c : Call(A)) -> Request(A) { c }

// ---- Dispatcher primitives ----

/// The ambient current Root.
root = () <- (r : Root) -> Root { r }

/// Opens a Root under the ambient one.
open_root = () <- (r : Root) -> Root { req.open_root() <- (r) }

/// Closes `r`: cancels everything started under it and closes its groups.
close_root = (r : Root) { req.close_root(r) }

/// Waits until one of `tickets` settles; terminal tickets return now.
wait_any = (tickets : List(Ticket)) { req.wait_any(tickets) }

/// Gives the host a progress turn.
poll = () { req.poll_requests() }
