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

Await = (wait = (Ticket) -> Nil)

Root = (key = Key(Nil))

Owner = (key = Key(Nil), root = Root)

Waiting = ()
Ready(A) = (A)
Cancelled = ()
Taken = ()
RequestState(A) = Waiting | Ready(A) | Cancelled | Taken

@external("ilang:request", "open_root")
new_root: (Key(Nil)) -> Key(Nil)

@external("ilang:request", "close_root")
end_root: (Key(Nil)) -> Nil

@external("ilang:request", "open_owner")
new_owner: (Key(Nil)) -> Key(Nil)

@external("ilang:request", "close_owner")
end_owner: (Key(Nil)) -> Nil

@external("ilang:request", "count")
group_count: (Key(Nil)) -> Int

@external("ilang:request", "count_root")
root_count: (Key(Nil)) -> Int

@external("ilang:request", "cancel_all")
cancel_group: (Key(Nil)) -> Nil

@external("ilang:request", "cancel_all_root")
cancel_root: (Key(Nil)) -> Nil

@external("ilang:request", "observe_owner")
observe_owner: (Key(Nil)) -> Request(Nil)

@external("ilang:request", "observe_root")
observe_root: (Key(Nil)) -> Request(Nil)

open_root = () <- (parent: Root) -> Root { Root(key = new_root(parent.key)) }

close_root = (root: Root) { end_root(root.key) }

open_owner = () <- (root: Root) -> Owner {
	Owner(key = new_owner(root.key), root = root)
}

close_owner = (owner: Owner) { end_owner(owner.key) }

count = (owner: Owner) { group_count(owner.key) }

count_root = (root: Root) { root_count(root.key) }

settled_owner = (owner: Owner) { count(owner) == 0 }

settled_root = (root: Root) { count_root(root) == 0 }

cancel_all = (owner: Owner) { cancel_group(owner.key) }

cancel_all_root = (root: Root) { cancel_root(root.key) }

retain = (value) { value }

within = (body) {
	owner = open_owner()
	resource.ensure(() { retain(body()) <- (owner) }, () { close_owner(owner) })
}

wait_none = (owner: Owner) { await_owned(observe_owner(owner.key)) }

wait_none_root = (root: Root) { await_owned(observe_root(root.key)) }

/// A fresh completed fake, with no group or host id.
@external("ilang:request", "ready")
ready: <A>(A) -> Request(A)

@external("ilang:request", "ticket")
ticket: <A>(Request(A)) -> Ticket

@external("ilang:request", "state")
read_state: <A>(Request(A)) -> Result(A, Int)

@external("ilang:request", "take")
take: <A>(Request(A)) -> A

@external("ilang:request", "settled")
settled: (Ticket) -> Bool

@external("ilang:request", "cancel")
cancel: (Ticket) -> Nil

state = <A>(r: Request(A)) -> RequestState(A) {
	read_state(r) |>
		(Ok(value)) { Ready(value) }
		| (Error(0)) { Waiting }
		| (Error(1)) { Cancelled }
		| (Error(2)) { Taken }
		| (Error(_)) { panic("request: invalid state") }
}

/// Borrows the producer; cancelling this waiter does not cancel its work.
await = (r) <- (waiter: Await) {
	waiter.wait(ticket(r))
	state(r) |>
		(Waiting) { panic("request: wait returned early") }
		| (_) { take(r) }
}

/// Owns a non-resource-producing operation through all exits.
await_owned = (r) { resource.ensure(() { await(r) }, () { cancel(ticket(r)) }) }

/// Waits through answers of unlisted requests; terminal tickets return now.
@external("ilang:request", "wait_any")
wait_any: (List(Ticket)) -> Nil

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
	root = Root(key = entry_root())
	owner = Owner(key = new_owner(root.key), root = root)
	resource.ensure(
		() {
			retain(body()) <- (root, owner, Await(wait = (r) { wait_any([r]) }))
		},
		() { entry_end() },
	)
}

// Plan factories keep replacement values ordinary, including adapter seeds.
_provider_root = (seeds: List(Root), key: Key(Nil)) -> Root {
	seeds |>
		([root]) { root }
		| ([]) { Root(key = key) }
		| (_) { panic("provider: invalid root seed") }
}

_provider_owner = (seeds: List(Owner)) <- (root: Root) -> Owner {
	seeds |>
		([owner]) { owner }
		| ([]) { Owner(key = new_owner(root.key), root = root) }
		| (_) { panic("provider: invalid owner seed") }
}

_provider_await = (seeds: List(Await)) -> Await {
	seeds |>
		([waiter]) { waiter }
		| ([]) { Await(wait = (r) { wait_any([r]) }) }
		| (_) { panic("provider: invalid await seed") }
}
