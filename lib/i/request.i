//// Typed requests, cancellation groups and replaceable waits (§9.11).

(
	i/resource
)

(
	Await(..)
	Root(..)
	Owner(..)
	RequestState(..)
	open_root
	close_root
	open_owner
	close_owner
	count
	count_root
	settled_owner
	settled_root
	cancel_all
	cancel_all_root
	wait_none
	wait_none_root
	within
	ready
	ticket
	state
	take
	settled
	cancel
	await
	await_owned
	wait_any
	poll
)

Await {
	Await(wait: (ticket: Ticket) -> Nil)
}

Root {
	Root(key: Key(Nil))
}

Owner {
	Owner(key: Key(Nil), root: Root)
}

RequestState(A) {
	Waiting
	Ready(A)
	Cancelled
	Taken
}

@external("ilang:request", "open_root")
new_root: (parent: Key(Nil)) -> Key(Nil)

@external("ilang:request", "close_root")
end_root: (root: Key(Nil)) -> Nil

@external("ilang:request", "open_owner")
new_owner: (root: Key(Nil)) -> Key(Nil)

@external("ilang:request", "close_owner")
end_owner: (owner: Key(Nil)) -> Nil

@external("ilang:request", "count")
group_count: (owner: Key(Nil)) -> Int

@external("ilang:request", "count_root")
root_count: (root: Key(Nil)) -> Int

@external("ilang:request", "cancel_all")
cancel_group: (owner: Key(Nil)) -> Nil

@external("ilang:request", "cancel_all_root")
cancel_root: (root: Key(Nil)) -> Nil

@external("ilang:request", "observe_owner")
observe_owner: (owner: Key(Nil)) -> Request(Nil)

@external("ilang:request", "observe_root")
observe_root: (root: Key(Nil)) -> Request(Nil)

open_root = (parent ! Root) -> Root ! Root { Root(new_root(parent.key)) }

close_root = (root: Root) { end_root(root.key) }

open_owner = (root ! Root) -> Owner ! Root { Owner(new_owner(root.key), root) }

close_owner = (owner: Owner) { end_owner(owner.key) }

count = (owner: Owner) { group_count(owner.key) }

count_root = (root: Root) { root_count(root.key) }

settled_owner = (owner: Owner) { count(owner) == 0 }

settled_root = (root: Root) { count_root(root) == 0 }

cancel_all = (owner: Owner) { cancel_group(owner.key) }

cancel_all_root = (root: Root) { cancel_root(root.key) }

retain = {
	value -> value
}

within = (body) {
	owner = open_owner()
	resource.ensure(() {
		! Owner = owner
		retain(body())
	}, () { close_owner(owner) })
}

wait_none = (owner: Owner) { await_owned(observe_owner(owner.key)) }

wait_none_root = (root: Root) { await_owned(observe_root(root.key)) }

/// A fresh completed fake, with no group or host id.
@external("ilang:request", "ready")
ready(A): (value: A) -> Request(A)

@external("ilang:request", "ticket")
ticket(A): (r: Request(A)) -> Ticket

@external("ilang:request", "state")
read_state(A): (r: Request(A)) -> Result(A, Int)

@external("ilang:request", "take")
take(A): (r: Request(A)) -> A

@external("ilang:request", "settled")
settled: (r: Ticket) -> Bool

@external("ilang:request", "cancel")
cancel: (r: Ticket) -> Nil

state(A) = (r: Request(A)) -> RequestState(A) {
	read_state(r) |> {
		Ok(value) -> Ready(value)
		Error(0)  -> Waiting
		Error(1)  -> Cancelled
		Error(2)  -> Taken
		Error(_)  -> panic("request: invalid state")
	}
}

/// Borrows the producer; cancelling this waiter does not cancel its work.
await = (r, waiter ! Await) {
	waiter.wait(ticket(r))
	state(r) |> {
		Waiting -> panic("request: wait returned early")
		_       -> take(r)
	}
}

/// Owns a non-resource-producing operation through all exits.
await_owned = (r) { resource.ensure(() { await(r) }, () { cancel(ticket(r)) }) }

/// Waits through answers of unlisted requests; terminal tickets return now.
@external("ilang:request", "wait_any")
wait_any: (rs: List(Ticket)) -> Nil

/// Gives the host a progress turn even when no request is outstanding.
@external("ilang:request", "poll")
poll: () -> Nil

// Entry provision calls these ordinary i definitions. Initial registration is
// the narrow entry bootstrap; public open_root always requires a live Root.
@external("ilang:request", "entry_root")
entry_root: () -> Key(Nil)

@external("ilang:request", "entry_end")
entry_end: () -> Nil

_entry = (body) {
	root = Root(entry_root())
	owner = Owner(new_owner(root.key), root)
	resource.ensure(() {
		! Root = root
		! Owner = owner
		! Await = Await((r) { wait_any([r]) })
		retain(body())
	}, () { entry_end() })
}
