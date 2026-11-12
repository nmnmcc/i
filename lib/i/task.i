//// Structured tasks, rendezvous channels and events (§9.9).

(
	i/control
	i/key
	i/core
	i/fail
	i/resource
	i/request
	rng = i/random
)

(
	Scheduler
	Runner
	Scope
	Task
	Channel
	Event
	Shared
	Ref
	ScheduleError(..)
	top
	run
	test
	scope
	local_scope
	go
	join
	yield
	path
	random
	keep
	channel
	local_channel
	send
	receive
	send_event
	receive_event
	join_event
	always
	never
	choose
	map
	sync
	share
	get
	ref
	update
	both
	race
	map_concurrently
)

Address {
	Address(task: Int, index: Int)
}

Witness {
	Portable
	Local(Boundary(Turn, Never, Never))
}

ScopeInfo {
	ScopeInfo(run: Key(Nil), id: Keyed, witness: Witness)
}

TaskInfo {
	TaskInfo(scope: ScopeInfo, id: Keyed)
}

ChannelInfo {
	ChannelInfo(run: Key(Nil), id: Keyed, witness: Witness)
}

Scope(E) {
	Scope(info: ScopeInfo, errors: Key(E))
}

Task(A, E) {
	Task(info: TaskInfo, result: Key(Result(A, E)), owner: Scope(E))
}

Channel(A) {
	Channel(info: ChannelInfo, values: Key(A))
}

Shared(A) {
	Shared(run: Key(Nil), owner: Keyed, id: Keyed, values: Key(A))
}

Ref(A) {
	Ref(run: Key(Nil), id: Keyed, values: Key(A))
}

ScheduleError {
	NotRunnable(path: List(Int))
}

StepAnswer {
	Proceed(Keyed)
	CancelStep(() -> Never)
	ForwardStep(
		ForeignNotice
		Keyed
		Bool
		(ForeignNotice, Keyed, Bool) -> Keyed
	)
}

Completion {
	Succeeded(Keyed)
	Failed(Keyed, Keyed)
	Abandoned
}

Turn {
	Switched(Int, Command, Continuation(StepAnswer, Turn, Never, Never))
	Finished(Int, Completion)
	Sent(Int)
}

Start {
	Start(
		body:   (Scheduler) -> Completion
		ticket: Int
	)
}

/// What a task on another worker tells the scheduler (native N7/N8): its
/// next step with the `finalizing` answers of its frames, or its end.
Remote {
	Stepped(Command, List(Bool))
	Exited(Completion)
	/// A request that the task waits for on its worker settled.
	Woken
}

/// What the scheduler answers a task on another worker: the step's answer
/// and the `final` queries of the task's frames, to ask at its next step.
Resume {
	Resume(
		answer: StepAnswer
		finals: List((Continuation(StepAnswer, Turn, Never, Never)) -> Bool)
		run:    Key(Nil)
	)
}

/// A published task record (multicore D8, native N7). The claiming worker
/// runs `begin(job)` and later `resume`; the record holds the complete start
/// function with its kept services, the task's path, run, root and runner.
Job {
	Job(
		begin:  (job: Job) -> (Remote, Continuation(StepAnswer, Turn, Never, Never), List(Ticket))
		resume: (
			input: (Continuation(StepAnswer, Turn, Never, Never), Resume)
		) -> (Remote, Continuation(StepAnswer, Turn, Never, Never), List(Ticket))
		body:   (Scheduler) -> Completion
		path:   List(Int)
		run:    Key(Nil)
		root:   request.Root
		runner: Runner
		woken:  Remote
	)
}

Policy {
	Fifo
	Script(List(List(Int)))
}

Runner {
	DetachedRunner
	Runner(
		seed:     rng.Generator
		context:  request.Root
		policy:   Policy
		allocate: () -> List(Int)
		active:   (Key(Nil), Active) -> RunControl
		wait:     (Key(Nil), request.Root, List(Ticket)) -> RunControl
		poll:     (Key(Nil), List(Ticket), Bool) -> RunControl
		forget:   (Key(Nil)) -> Nil
	)
}

Scheduler {
	DetachedScheduler
	Scheduler(
		run:     Key(Nil)
		root:    request.Root
		path:    Keyed
		seed:    () -> rng.Generator
		witness: Witness
		step:    (Command) -> StepAnswer
		runner:  Runner
		waiter:  request.Await
	)
}

Maybe(A) {
	Absent
	Present(A)
}

Mode {
	Ordinary
	Racing
}

BodyMark {
	BodyOpen
	BodyOk
	BodyError
	BodyCancelled
}

ScopeReply {
	ScopeOk
	ScopeError(Keyed)
	ScopeWinner(Keyed)
}

@external("ilang:control", "finalizing")
finalizing(A, R, E, N, B, F, M): (
	k: Continuation(A, R, E, N)
	b: Boundary(B, F, M)
) -> Bool

Frame {
	Frame(
		scope:     Maybe(Address)
		abort:     () -> Never
		interrupt: () -> Never
		relay:     (ForeignNotice, Keyed, Bool) -> Keyed
		foreign:   List(Hop)
		immune:    List(Cause)
		final:     (Continuation(StepAnswer, Turn, Never, Never)) -> Bool
		finishing: Bool
		unwinding: Maybe(Int)
		deferred:  Bool
		cause:     Maybe(Cause)
	)
}

Cause {
	Cause(run: Key(Nil), scope: Maybe(Address), epoch: Int)
}

Outcome {
	Pending
	Success(Keyed)
	Failure(Keyed)
	Cancelled(Cause)
}

Status {
	Dormant(Start)
	Runnable(Continuation(StepAnswer, Turn, Never, Never), StepAnswer)
	WaitingTicket(Continuation(StepAnswer, Turn, Never, Never), List(Ticket))
	WaitingScope(Continuation(StepAnswer, Turn, Never, Never), Address)
	WaitingEvent(
		Continuation(StepAnswer, Turn, Never, Never)
		Request(Registration)
	)
	WaitingDrain(Continuation(StepAnswer, Turn, Never, Never))
	Running
	/// Running on another worker until its next message.
	Away
	Ended
}

TaskState {
	TaskState(
		path:         List(Int)
		serial:       Int
		alive:        Request(Nil)
		queued:       Bool
		parent:       Maybe(Address)
		origin:       Maybe(ScopeInfo)
		base:         List(Address)
		chain:        List(Address)
		frames:       List(Frame)
		immune:       List(Cause)
		spawn:        Int
		handles:      Int
		nested:       List(Nest)
		driver:       Maybe(Key(Nil))
		foreign:      List(ForeignGuard)
		messages:     List(ForeignNotice)
		cancelled_by: Maybe(Cause)
		status:       Status
		outcome:      Outcome
		clean:        Bool
		joiners:      WaitQueue
		open_scopes:  Int
		cleanup:      Bool
	)
}

ScopeState {
	ScopeState(
		info:           ScopeInfo
		address:        Address
		body:           Int
		chain:          List(Address)
		immune:         List(Cause)
		cleanup:        Bool
		foreign:        List(Hop)
		token:          Key(Nil)
		children:       List(Int)
		pending:        Int
		mark:           BodyMark
		errors:         List((List(Int), Keyed))
		cancellation:   Maybe(Cause)
		mode:           Mode
		winner:         Maybe(Keyed)
		retiring:       Bool
		prior_cleanup:  Bool
		owned_channels: List(Address)
		owned_shares:   List(Address)
	)
}

ChannelState {
	ChannelState(
		info:      ChannelInfo
		address:   Address
		owner:     Maybe(Address)
		senders:   WaitQueue
		receivers: WaitQueue
	)
}

ShareState {
	ShareState(id: Address, owner: Maybe(Address), value: Keyed)
}

Leaf {
	SendLeaf(ChannelInfo, Keyed)
	ReceiveLeaf(ChannelInfo)
	JoinLeaf(TaskInfo)
	AlwaysLeaf(Keyed)
}

Branch(A, E, N) {
	Branch(leaf: Leaf, decode: (Keyed) -> A ^ E ! N)
}

Event(A, E, N) {
	Event(identity: Key(Nil), branches: List(Branch(A, E, N)))
}

Selection {
	Selected(Int, Keyed)
	SelectedCancelled
}

Waiter {
	Waiter(flag: Request(Registration), index: Int)
}

WaitQueue {
	WaitQueue(front: List(Waiter), back: List(Waiter), total: Int, live: Int)
}

QueueHead {
	QueueHead(queue: WaitQueue, waiter: Maybe(Waiter))
}

no_waiters = WaitQueue([], [], 0, 0)

Registration {
	Registration(id: Int, task: Int, leaves: List(Leaf))
}

Nest {
	Nest(
		id:       Key(Nil)
		anchor:   Maybe(Address)
		active:   Active
		notified: List(Cause)
	)
}

Forward {
	Forward(id: Key(Nil), tickets: List(Ticket))
}

ActiveTask {
	ActiveTask(id: Int, alive: Request(Nil))
}

State {
	State(
		run:             Key(Nil)
		runner:          Runner
		root:            request.Root
		tasks:           TaskStore(TaskState)
		active:          List(ActiveTask)
		active_count:    Int
		active_size:     Int
		unclean:         List(Int)
		task_serial:     Int
		root_id:         Int
		cancel_history:  Bool
		message_history: Bool
		scopes:          TaskStore(ScopeState)
		dirty_scopes:    List(Address)
		channels:        TaskStore(ChannelState)
		shares:          TaskStore(ShareState)
		current:         Maybe(Int)
		waiting:         List(Int)
		ready:           List(Int)
		ready_back:      List(Int)
		changed_sources: List(Int)
		forwards:        List(Forward)
		choices:         List(List(Int))
		scripted:        Bool
		failure:         Maybe(List(Int))
		cancellation:    Maybe(Cause)
		outer_cancel:    Maybe(() -> Never)
		epoch:           Int
		registration:    Int
		top:             Bool
		register_flag:   Bool
	)
}

Command {
	Register(
		Maybe(Address)
		() -> Never
		() -> Never
		(ForeignNotice, Keyed, Bool) -> Keyed
		(Continuation(StepAnswer, Turn, Never, Never)) -> Bool
	)
	ExternalCancel
	Relay(ForeignNotice, Keyed, Bool)
	FinishScope(ScopeInfo)
	NewScope(Witness, Mode)
	EndScope(ScopeInfo, Completion)
	Spawn(ScopeInfo, Start)
	NextRun
	Yield
	Wait(List(Ticket))
	NestedActive(Key(Nil), Active)
	NestedWait(Key(Nil), List(Ticket))
	ForwardWait(Key(Nil), List(Ticket))
	ForgetWait(Key(Nil))
	NewChannel(Witness)
	SendNow(ChannelInfo, Keyed)
	Select(List(Leaf))
	NewShare(ScopeInfo, Keyed)
	GetShare(Key(Nil), Keyed, Keyed)
	NewRef(Keyed)
	UpdateRef(Key(Nil), Keyed, (Keyed) -> (Keyed, Keyed))
	Drain
}

RunReply {
	RunOk(Keyed)
	RunError(List(Int))
	RunCancelled(() -> Never)
}

RunControl {
	ContinueRun
	ParentCancel(() -> Never)
	ForwardRun(ForeignNotice)
}

ScopeResult(A) {
	BodyValue(A)
	WinnerValue(Keyed)
}

CancelPlan {
	CancelPlan(frames: List(Frame), abort: () -> Never)
}

Commit {
	Commit(state: State, selection: Maybe(Selection))
}

Partner {
	Partner(state: State, waiter: Maybe(Waiter))
}

address_key: Key(Address) = key.new()

path_key: Key(List(Int)) = key.new()

task_key: Key(Int) = key.new()

scope_key: Key(ScopeInfo) = key.new()

channel_key: Key(ChannelInfo) = key.new()

unit_key: Key(Nil) = key.new()

scope_reply_key: Key(ScopeReply) = key.new()

selection_key: Key(Selection) = key.new()

read(A) = (k: Key(A), value: Keyed) -> A {
	key.unwrap(k, value) |> {
		key.Found(v) -> v
		key.Missing  -> panic("task: invalid protocol key")
	}
}

retain = {
	value -> value
}

unit_value = key.wrap(unit_key, Nil)

unit = () { unit_value }

answer = (reply: StepAnswer) {
	reply |> {
		Proceed(v)         -> v
		CancelStep(cancel) -> cancel()
		ForwardStep(notice, value, discard, relay) -> relay(
			notice
			value
			discard
		)
	}
}

ask = (own: Scheduler, command: Command) -> Keyed {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..)     -> answer(own.step(command))
	}
}

check_run = (actual: Key(Nil), wanted: Key(Nil)) {
	{ actual == wanted } |> {
		True  -> Nil
		False -> panic("task: handle of another run")
	}
}

portable = (value) {
	core.portable(value) |> {
		True  -> Nil
		False -> panic("task: value is not portable")
	}
}

contains(A) = (xs: List(A), value: A) -> Bool {
	xs |> {
		[]          -> False
		[x, ..rest] -> x == value || contains(rest, value)
	}
}

append(A) = (xs: List(A), ys: List(A)) -> List(A) {
	xs |> {
		[]          -> ys
		[x, ..rest] -> [x, ..append(rest, ys)]
	}
}

reverse(A) = (xs: List(A), acc: List(A)) -> List(A) {
	xs |> {
		[]          -> acc
		[x, ..rest] -> reverse(rest, [x, ..acc])
	}
}

without(A) = (xs: List(A), value: A) -> List(A) {
	xs |> {
		[] -> []
		[x, ..rest] -> { x == value } |> {
			True  -> without(rest, value)
			False -> [x, ..without(rest, value)]
		}
	}
}

// Persistent AVL storage gives retained results logarithmic lookup without
// scanning them at every scheduling point. Active and unfinished-cleanup
// indexes use creation serials, preserving cancellation and delivery order.
TaskTree(A) {
	NoTasks
	TaskNode(Int, Int, A, TaskTree(A), TaskTree(A))
}

tree_height(A) = (tree: TaskTree(A)) -> Int {
	tree |> {
		NoTasks                 -> 0
		TaskNode(h, _, _, _, _) -> h
	}
}

tree_node(A) = (
	id: Int
	task: A
	left: TaskTree(A)
	right: TaskTree(A)
) -> TaskTree(A) {
	a = tree_height(left)
	b = tree_height(right)
	h = { a > b } |> {
		True  -> a
		False -> b
	}
	TaskNode(h + 1, id, task, left, right)
}

tree_balance(A) = (
	id: Int
	task: A
	left: TaskTree(A)
	right: TaskTree(A)
) -> TaskTree(A) {
	delta = tree_height(left) - tree_height(right)
	(delta, left, right) |> {
		(d, TaskNode(_, lid, lt, ll, lr), _) ? d > 1 -> {
			{ tree_height(ll) >= tree_height(lr) } |> {
				True -> tree_node(lid, lt, ll, tree_node(id, task, lr, right))
				False -> lr |> {
					TaskNode(_, mid, mt, ml, mr) -> tree_node(
						mid
						mt
						tree_node(lid, lt, ll, ml)
						tree_node(id, task, mr, right)
					)
					NoTasks -> panic("task: invalid left rotation")
				}
			}
		}
		(d, _, TaskNode(_, rid, rt, rl, rr)) ? d < -1 -> {
			{ tree_height(rr) >= tree_height(rl) } |> {
				True -> tree_node(rid, rt, tree_node(id, task, left, rl), rr)
				False -> rl |> {
					TaskNode(_, mid, mt, ml, mr) -> tree_node(
						mid
						mt
						tree_node(id, task, left, ml)
						tree_node(rid, rt, mr, rr)
					)
					NoTasks -> panic("task: invalid right rotation")
				}
			}
		}
		_ -> tree_node(id, task, left, right)
	}
}

tree_find(A) = (tree: TaskTree(A), wanted: Int) -> Maybe(A) {
	tree |> {
		NoTasks -> Absent
		TaskNode(_, id, task, left, right) -> { wanted == id } |> {
			True -> Present(task)
			False -> { wanted < id } |> {
				True  -> tree_find(left, wanted)
				False -> tree_find(right, wanted)
			}
		}
	}
}

// Four slots per immutable page. Two record caches handle the running
// task and its peer without copying table paths; two page caches amortize
// persistent-index updates across sequential task ids.
TaskPage(A) {
	TaskPage(count: Int, p0: Maybe(A), p1: Maybe(A), p2: Maybe(A), p3: Maybe(A))
}

empty_page(A) = () -> TaskPage(A) {
	TaskPage(0, Absent, Absent, Absent, Absent)
}

PageCache(A) {
	PageCache(id: Int, page: TaskPage(A), dirty: Bool)
}

TaskStore(A) {
	TaskStore(
		focus_id: Int
		focus:    Maybe(A)
		spare_id: Int
		spare:    Maybe(A)
		first:    Maybe(PageCache(A))
		second:   Maybe(PageCache(A))
		rows:     TaskTree(TaskPage(A))
	)
}

empty_store(A) = () -> TaskStore(A) {
	TaskStore(-1, Absent, -1, Absent, Absent, Absent, NoTasks)
}

page_get(A) = (page: TaskPage(A), slot: Int) -> Maybe(A) {
	slot |> {
		0 -> page.p0
		1 -> page.p1
		2 -> page.p2
		_ -> page.p3
	}
}

page_put(A) = (page: TaskPage(A), slot: Int, value: Maybe(A)) -> TaskPage(A) {
	old = page_get(page, slot) |> {
		Absent -> 0
		_      -> 1
	}
	new = value |> {
		Absent -> 0
		_      -> 1
	}
	count = page.count + new - old
	slot |> {
		0 -> TaskPage(..page, p0: value, count: count)
		1 -> TaskPage(..page, p1: value, count: count)
		2 -> TaskPage(..page, p2: value, count: count)
		_ -> TaskPage(..page, p3: value, count: count)
	}
}

page_find(A) = (store: TaskStore(A), id: Int) -> TaskPage(A) {
	store.first |> {
		Present(c) ? c.id == id -> c.page
		_ -> store.second |> {
			Present(c) ? c.id == id -> c.page
			_ -> tree_find(store.rows, id) |> {
				Present(page) -> page
				Absent        -> empty_page()
			}
		}
	}
}

flush_page(A) = (
	rows: TaskTree(TaskPage(A))
	cache: Maybe(PageCache(A))
) -> TaskTree(TaskPage(A)) {
	cache |> {
		Present(c) ? c.dirty -> { c.page.count == 0 } |> {
			True  -> drop_task(rows, c.id)
			False -> put_task(rows, c.id, c.page)
		}
		_ -> rows
	}
}

write_slot(A) = (
	store: TaskStore(A)
	id: Int
	value: Maybe(A)
) -> TaskStore(A) {
	number = id / 4
	slot = id % 4
	store.first |> {
		Present(c) ? c.id == number -> TaskStore(
			..store
			first: Present(
				PageCache(number, page_put(c.page, slot, value), True)
			)
		)
		_ -> store.second |> {
			Present(c) ? c.id == number -> TaskStore(
				..store
				second: Present(
					PageCache(number, page_put(c.page, slot, value), True)
				)
			)
			_ -> {
				page = page_find(store, number)
				rows = flush_page(drop_task(store.rows, number), store.second)
				TaskStore(
					..store
					rows:   rows
					second: store.first
					first: Present(
						PageCache(number, page_put(page, slot, value), True)
					)
				)
			}
		}
	}
}

store_find(A) = (store: TaskStore(A), id: Int) -> Maybe(A) {
	{ store.focus_id == id } |> {
		True -> store.focus
		False -> { store.spare_id == id } |> {
			True  -> store.spare
			False -> page_get(page_find(store, id / 4), id % 4)
		}
	}
}

find_task = (store: TaskStore(TaskState), id: Int) -> TaskState {
	store_find(store, id) |> {
		Present(t) -> t
		Absent     -> panic("task: task is gone")
	}
}

flush_record(A) = (store: TaskStore(A)) -> TaskStore(A) {
	store.spare |> {
		Absent     -> store
		Present(_) -> write_slot(store, store.spare_id, store.spare)
	}
}

store_update(A) = (store: TaskStore(A), id: Int, value: A) -> TaskStore(A) {
	{ store.focus_id == id } |> {
		True -> TaskStore(..store, focus: Present(value))
		False -> { store.spare_id == id } |> {
			True -> TaskStore(..store, spare: Present(value))
			False -> TaskStore(
				..flush_record(write_slot(store, id, Absent))
				spare_id: id
				spare:    Present(value)
			)
		}
	}
}

focus_task(A) = (store: TaskStore(A), id: Int) -> TaskStore(A) {
	{ store.focus_id == id } |> {
		True -> store
		False -> { store.spare_id == id } |> {
			True -> TaskStore(
				..store
				focus_id: id
				focus:    store.spare
				spare_id: store.focus_id
				spare:    store.focus
			)
			False -> {
				value = store_find(store, id) |> {
					Present(t) -> t
					Absent     -> panic("task: missing table row")
				}
				next = flush_record(write_slot(store, id, Absent))
				TaskStore(
					..next
					focus_id: id
					focus:    Present(value)
					spare_id: store.focus_id
					spare:    store.focus
				)
			}
		}
	}
}

put_task(A) = (tree: TaskTree(A), id: Int, task: A) -> TaskTree(A) {
	tree |> {
		NoTasks -> tree_node(id, task, NoTasks, NoTasks)
		TaskNode(_, old, value, left, right) -> { id == old } |> {
			True -> tree_node(id, task, left, right)
			False -> { id < old } |> {
				True -> tree_balance(
					old
					value
					put_task(left, id, task)
					right
				)
				False -> tree_balance(
					old
					value
					left
					put_task(right, id, task)
				)
			}
		}
	}
}

// Replacing a value cannot change subtree heights. Rebalance only insertions
// and removals; normal task status updates preserve the existing shape.
update_task(A) = (tree: TaskTree(A), wanted: Int, value: A) -> TaskTree(A) {
	tree |> {
		NoTasks -> panic("task: missing task update")
		TaskNode(h, id, task, left, right) -> { wanted == id } |> {
			True -> TaskNode(h, id, value, left, right)
			False -> { wanted < id } |> {
				True -> TaskNode(
					h
					id
					task
					update_task(left, wanted, value)
					right
				)
				False -> TaskNode(
					h
					id
					task
					left
					update_task(right, wanted, value)
				)
			}
		}
	}
}

tree_first(A) = (tree: TaskTree(A)) -> (Int, A) {
	tree |> {
		NoTasks                        -> panic("task: empty task index")
		TaskNode(_, id, t, NoTasks, _) -> (id, t)
		TaskNode(_, _, _, left, _)     -> tree_first(left)
	}
}

drop_task(A) = (tree: TaskTree(A), wanted: Int) -> TaskTree(A) {
	tree |> {
		NoTasks -> NoTasks
		TaskNode(_, id, task, left, right) -> { wanted == id } |> {
			True -> (left, right) |> {
				(NoTasks, _) -> right
				(_, NoTasks) -> left
				_ -> {
					(next, value) = tree_first(right)
					tree_balance(next, value, left, drop_task(right, next))
				}
			}
			False -> { wanted < id } |> {
				True  -> tree_balance(id, task, drop_task(left, wanted), right)
				False -> tree_balance(id, task, left, drop_task(right, wanted))
			}
		}
	}
}

tree_rows(A) = (tree: TaskTree(A), rest: List(A)) -> List(A) {
	tree |> {
		NoTasks -> rest
		TaskNode(_, _, task, left, right) -> tree_rows(
			left
			[task, ..tree_rows(right, rest)]
		)
	}
}

store_add(A) = (store: TaskStore(A), id: Int, value: A) -> TaskStore(A) {
	store.focus |> {
		Absent -> TaskStore(..store, focus_id: id, focus: Present(value))
		_ -> TaskStore(
			..flush_record(store)
			spare_id: id
			spare:    Present(value)
		)
	}
}

drop_children(A) = (store: TaskStore(A), ids: List(Int)) -> TaskStore(A) {
	ids |> {
		[] -> store
		[id, ..rest] -> {
			next = { store.focus_id == id } |> {
				True -> TaskStore(..store, focus_id: -1, focus: Absent)
				False -> { store.spare_id == id } |> {
					True  -> TaskStore(..store, spare_id: -1, spare: Absent)
					False -> write_slot(store, id, Absent)
				}
			}
			drop_children(next, rest)
		}
	}
}

address_index = (id: Address) -> Int {
	n = id.task + id.index
	n * { n + 1 } / 2 + id.index
}

find_scope = (scopes: TaskStore(ScopeState), id: Address) -> ScopeState {
	store_find(scopes, address_index(id)) |> {
		Present(s) -> s
		Absent     -> panic("task: scope is gone")
	}
}

put_scope = (scopes: TaskStore(ScopeState), scope: ScopeState) {
	store_update(scopes, address_index(scope.address), scope)
}

active_live = (task: ActiveTask) {
	request.state(task.alive) |> {
		request.Ready(_) -> True
		_                -> False
	}
}

record_task = (
	s: State
	t: TaskState
	tasks: TaskStore(TaskState)
	new: Bool
) -> State {
	previous = { new } |> {
		True  -> t
		False -> find_task(s.tasks, t.serial)
	}
	was_clean = new || previous.clean
	became_clean = core.not(was_clean) && t.clean
	became_clean |> {
		True -> {
			request.take(t.alive)
			Nil
		}
		False -> Nil
	}
	added = new && core.not(t.clean)
	count = s.active_count + {
		added |> {
			True  -> 1
			False -> 0
		}
	} - {
		became_clean |> {
			True  -> 1
			False -> 0
		}
	}
	size = s.active_size + {
		added |> {
			True  -> 1
			False -> 0
		}
	}
	active = added |> {
		True  -> [ActiveTask(t.serial, t.alive), ..s.active]
		False -> s.active
	}
	compact = size > count * 2
	active = compact |> {
		True  -> filter(active, active_live)
		False -> active
	}
	size = compact |> {
		True  -> count
		False -> size
	}
	ended = t.status |> {
		Ended -> True
		_     -> False
	}
	was_ended = previous.status |> {
		Ended -> True
		_     -> False
	}
	unclean = {
		ended && core.not(t.clean) && { new || core.not(was_ended) }
	} |> {
		True -> [t.serial, ..s.unclean]
		False -> { became_clean } |> {
			True  -> without(s.unclean, t.serial)
			False -> s.unclean
		}
	}
	changed = { new } |> {
		True -> False
		False -> previous.outcome |> {
			Pending -> t.outcome |> {
				Pending -> False
				_       -> True
			}
			_ -> False
		}
	}
	next = State(
		..s
		tasks:        tasks
		active:       active
		active_count: count
		active_size:  size
		unclean:      unclean
		changed_sources: changed |> {
			True  -> [t.serial, ..s.changed_sources]
			False -> s.changed_sources
		}
		cancel_history:  s.cancel_history || t.cancelled_by != Absent
		message_history: s.message_history || core.not(t.messages == [])
	)
	became_clean |> {
		False -> next
		True -> t.parent |> {
			Absent -> next
			Present(parent) -> {
				scope = find_scope(next.scopes, parent)
				set_scope(next, ScopeState(..scope, pending: scope.pending - 1))
			}
		}
	}
}

set_task = (s: State, t: TaskState) {
	record_task(s, t, store_update(s.tasks, t.serial, t), False)
}

add_task = (s: State, t: TaskState) {
	record_task(s, t, store_add(s.tasks, t.serial, t), True)
}

set_scope = (s: State, scope: ScopeState) {
	closed = scope.mark |> {
		BodyOpen -> False
		_        -> True
	}
	ready = closed && scope.pending == 0 && core.not(scope.retiring)
	updated = ready |> {
		True  -> ScopeState(..scope, retiring: True)
		False -> scope
	}
	State(..s, scopes: put_scope(s.scopes, updated), dirty_scopes: ready |> {
		True  -> [scope.address, ..s.dirty_scopes]
		False -> s.dirty_scopes
	}, cancel_history: s.cancel_history || scope.cancellation != Absent)
}

/// An inner delimiter has a fixed pure result type. Its body was kept at
/// its typed call site, so cancellation needs no cast or polymorphic field.
guarded = (
	own: Scheduler
	owner: Maybe(Address)
	body: () -> Completion
) -> Completion {
	control.boundary((b) {
		ask(own, Register(owner, () {
			control.suspend(b, (k) {
				control.cancel(k)
				Abandoned
			})
		}, () {
			ask(own, ExternalCancel)
			panic("task: cancellation returned")
		}, (notice: ForeignNotice, value: Keyed, discard: Bool) {
			ask(own, Relay(notice, value, discard))
		}, (k) { finalizing(k, b) }))
		body()
	})
}

complete(A, E, N) = (
	body: () -> A ^ E ! N
	result: Key(Result(A, E))
	errors: Key(E)
	check: Bool
) -> Completion ! N {
	fail.catch(body) |> {
		Ok(value) -> {
			{ check } |> {
				True  -> portable(value)
				False -> Nil
			}
			Succeeded(key.wrap(result, Ok(value)))
		}
		Error(error) -> {
			{ check } |> {
				True  -> portable(error)
				False -> Nil
			}
			Failed(
				key.wrap(result, Error(error))
				key.wrap(errors, error)
			)
		}
	}
}

make_start(A, E, N) = (
	body: () -> A ^ E ! N
	result: Key(Result(A, E))
	errors: Key(E)
	check: Bool
) -> Start ! N - Scheduler - Runner - request.Await {
	! Scheduler = DetachedScheduler
	! Runner = DetachedRunner
	! request.Await = request.Await((_) { panic("task: no scheduler") })
	kept = control.keep_env((own: Scheduler) {
		own |> {
			DetachedScheduler -> panic("task: no scheduler")
			Scheduler(..) -> {
				! Scheduler = own
				! Runner = own.runner
				! request.Await = own.waiter
				local = control.keep_env((_: Nil) {
					complete(body, result, errors, check)
				})
				retain(guarded(own, Absent, () { local(Nil) }))
			}
		}
	})
	Start(kept, 0)
}

@external("ilang:worker", "enabled")
worker_enabled: () -> Bool

@external("ilang:worker", "publish")
worker_publish: (job: Job) -> Int

@external("ilang:worker", "claim")
worker_claim: (ticket: Int) -> Bool

@external("ilang:worker", "adopt")
worker_adopt: (ticket: Int, task: Int) -> Nil

@external("ilang:worker", "placeholder")
worker_placeholder: () -> Continuation(StepAnswer, Turn, Never, Never)

@external("ilang:worker", "away")
worker_away: (k: Continuation(StepAnswer, Turn, Never, Never)) -> Bool

@external("ilang:worker", "send")
worker_send: (task: Int, resume: Resume) -> Nil

@external("ilang:worker", "inbox")
worker_inbox: () -> Int

@external("ilang:worker", "message")
worker_message: () -> Remote

@external("ilang:worker", "pending")
worker_pending: () -> Bool

@external("ilang:worker", "wait")
worker_wait: () -> Nil

@external("ilang:worker", "share_put")
worker_share_put: (run: Key(Nil), address: Address, value: Keyed) -> Nil

@external("ilang:worker", "share_get")
worker_share_get: (run: Key(Nil), address: Address) -> List(Keyed)

@external("ilang:worker", "share_drop")
worker_share_drop: (run: Key(Nil), address: Address) -> Nil

drop_shares = (s: State, ids: List(Address)) {
	ids |> {
		[] -> Nil
		[id, ..rest] -> {
			worker_share_drop(s.run, id)
			drop_shares(s, rest)
		}
	}
}

/// A task on a claiming worker runs to its next step there. A `get` of a
/// value shared in its run is answered from the host transport at once
/// (the value is there exactly while its scope lives); every other step
/// goes to the scheduler of the run.
remote_turn = (
	turn: Turn
	finals: List((Continuation(StepAnswer, Turn, Never, Never)) -> Bool)
	run: Key(Nil)
) -> (Remote, Continuation(StepAnswer, Turn, Never, Never), List(Ticket)) {
	turn |> {
		Switched(_, command, k) -> command |> {
			GetShare(handle, _, id) ? handle == run -> worker_share_get(
				run
				read(address_key, id)
			) |> {
				[value] -> remote_turn(control.resume(k, Proceed(value)), finals, run)
				_ -> (Stepped(command, list_map(finals, (f) { f(k) })), k, [])
			}
			// Requests of this worker are waited for here; the scheduler
			// only sees a wait that a `Woken` message or a cancel ends.
			Wait(tickets) -> (Stepped(Wait([]), list_map(finals, (f) { f(k) })), k, tickets)
			Register(_, _, _, _, final) -> (
				Stepped(command, append(list_map(finals, (f) { f(k) }), [final(k)]))
				k
				[]
			)
			_ -> (Stepped(command, list_map(finals, (f) { f(k) })), k, [])
		}
		Finished(_, completion) -> (Exited(completion), worker_placeholder(), [])
		Sent(_) -> panic("task: a worker task was sent")
	}
}

remote_begin = (job: Job) -> (Remote, Continuation(StepAnswer, Turn, Never, Never), List(Ticket)) {
	task_path = job.path
	run = job.run
	root = job.root
	runner = job.runner
	body = job.body
	remote_turn(control.boundary((b) {
		own = make_scheduler(b, task_path, 0, run, root, runner)
		Finished(0, body(own))
	}), [], run)
}

remote_resume = (
	input: (Continuation(StepAnswer, Turn, Never, Never), Resume)
) -> (Remote, Continuation(StepAnswer, Turn, Never, Never), List(Ticket)) {
	(k, next) = input
	remote_turn(control.resume(k, next.answer), next.finals, next.run)
}

/// Publish the record of a new task of the entry run when more than one
/// worker runs (D8). A nested run keeps its tasks (runtime §6.5).
publish = (s: State, task_path: List(Int), start: Start) -> Start {
	{ s.scripted || core.not(s.top) || core.not(worker_enabled()) } |> {
		True  -> start
		False -> Start(
			..start
			ticket: worker_publish(Job(
				remote_begin
				remote_resume
				start.body
				task_path
				s.run
				s.root
				s.runner
				Woken
			))
		)
	}
}

frame_finals = (frames: List(Frame)) -> List((Continuation(StepAnswer, Turn, Never, Never)) -> Bool) {
	list_map(frames, (f) { f.final })
}

/// The answers for the task's frames, then the one for a frame it registers.
split_flags = (frames: List(Frame), flags: List(Bool)) -> (List(Bool), List(Bool)) {
	(frames, flags) |> {
		([_, ..rest], [flag, ..more]) -> {
			(own, extra) = split_flags(rest, more)
			([flag, ..own], extra)
		}
		_ -> ([], flags)
	}
}

apply_finals = (frames: List(Frame), flags: List(Bool)) -> List(Frame) {
	(frames, flags) |> {
		([], []) -> []
		([f, ..rest], [flag, ..more]) -> [
			Frame(..f, finishing: flag)
			..apply_finals(rest, more)
		]
		_ -> panic("task: frames of a worker task changed")
	}
}

/// Take one message of a task on another worker, when no task is current.
pull = (s: State) -> State {
	{ s.top && s.current == Absent } |> {
		False -> s
		True -> worker_inbox() |> {
			0 -> s
			serial -> {
				t = find_task(s.tasks, serial)
				running = set_task(s, TaskState(..t, status: Running))
				worker_message() |> {
					Stepped(command, flags) -> on_command(
						running
						serial
						command
						worker_placeholder()
						Present(flags)
					)
					Exited(completion) -> observe_completion(
						running
						find_task(running.tasks, serial)
						completion
					)
					Woken -> t.status |> {
						WaitingTicket(k, _) -> queue(
							set_task(
								State(..s, waiting: without(s.waiting, serial))
								TaskState(..t, status: Runnable(k, Proceed(unit())))
							)
							serial
						)
						_ -> s
					}
				}
			}
		}
	}
}

path = (own ! Scheduler) {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..)     -> read(path_key, own.path)
	}
}

random = (own ! Scheduler) {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..)     -> own.seed()
	}
}

yield = (own ! Scheduler) {
	ask(own, Yield)
	Nil
}

top = (seed: rng.Generator, current ! Runner) -> Runner ! Runner {
	current |> {
		DetachedRunner -> panic("task: no scheduler")
		Runner(..)     -> Runner(..current, seed: seed)
	}
}

@external("ilang:request", "observe_change")
observe_change: (root: Key(Nil)) -> Request(Nil)

@external("ilang:request", "has_work")
has_work: (ticket: Ticket) -> Bool

wait_root = (_run: Key(Nil), root: request.Root, tickets: List(Ticket)) {
	{ request.count_root(root) > 0 } |> {
		False -> request.wait_any(tickets)
		True -> {
			changed = observe_change(root.key)
			resource.ensure(
				() { request.wait_any([request.ticket(changed), ..tickets]) }
				() { request.cancel(request.ticket(changed)) }
			)
		}
	}
	ContinueRun
}

root_poll = (_run: Key(Nil), tickets: List(Ticket), runnable: Bool) {
	{ runnable && core.not(tickets == []) } |> {
		False -> Nil
		True  -> request.poll()
	}
	ContinueRun
}

root_forget = (_run: Key(Nil)) { Nil }

root_allocate = () -> List(Int) { [] }

initial = (root: request.Root) -> Runner {
	Runner(
		rng.from_seed(0)
		root
		Fifo
		root_allocate
		root_active
		wait_root
		root_poll
		root_forget
	)
}

run_control = (reply: StepAnswer) {
	reply |> {
		Proceed(_)                   -> ContinueRun
		CancelStep(abort)            -> ParentCancel(abort)
		ForwardStep(notice, _, _, _) -> ForwardRun(notice)
	}
}

make_scheduler = (
	b: Boundary(Turn, Never, Never)
	task_path: List(Int)
	id: Int
	run: Key(Nil)
	root: request.Root
	upstream: Runner
) -> Scheduler {
	upstream |> {
		DetachedRunner -> panic("task: no scheduler")
		Runner(..) -> {
			route = (command: Command) {
				control.suspend(b, (k) { Switched(id, command, k) })
			}
			policy = upstream.policy |> {
				Fifo      -> Fifo
				Script(_) -> Script([])
			}
			runner = Runner(
				upstream.seed
				upstream.context
				policy
				() { read(path_key, answer(route(NextRun))) }
				(id: Key(Nil), active: Active) {
					run_control(route(NestedActive(id, active)))
				}
				(id: Key(Nil), root: request.Root, tickets: List(Ticket)) {
					{ request.count_root(root) > 0 } |> {
						False -> run_control(route(NestedWait(id, tickets)))
						True -> {
							changed = observe_change(root.key)
							resource.ensure(
								() {
									run_control(
										route(
											NestedWait(
												id
												[
													request.ticket(changed)
													..tickets
												]
											)
										)
									)
								}
								() { request.cancel(request.ticket(changed)) }
							)
						}
					}
				}
				(id: Key(Nil), tickets: List(Ticket), _runnable: Bool) {
					run_control(route(ForwardWait(id, tickets)))
				}
				(id: Key(Nil)) {
					answer(route(ForgetWait(id)))
					Nil
				}
			)
			waiter = request.Await((ticket) {
				answer(route(Wait([ticket])))
				Nil
			})
			// Derive only when random() is read. Task paths/seed are immutable;
			// starting an empty child does not need any of the generator's words.
			seed = upstream.seed
			generator = () {
				fail.catch(() { rng.at_path(seed, task_path) }) |> {
					Ok(g)    -> g
					Error(_) -> panic("task: invalid task path")
				}
			}
			Scheduler(
				run
				root
				key.wrap(path_key, task_path)
				generator
				Local(b)
				route
				runner
				waiter
			)
		}
	}
}

list_map(A, B) = (xs: List(A), f: (A) -> B) -> List(B) {
	xs |> {
		[]          -> []
		[x, ..rest] -> [f(x), ..list_map(rest, f)]
	}
}

any(A) = (xs: List(A), f: (A) -> Bool) -> Bool {
	xs |> {
		[]          -> False
		[x, ..rest] -> f(x) || any(rest, f)
	}
}

filter(A) = (xs: List(A), f: (A) -> Bool) -> List(A) {
	xs |> {
		[] -> []
		[x, ..rest] -> { f(x) } |> {
			True  -> [x, ..filter(rest, f)]
			False -> filter(rest, f)
		}
	}
}

// Paths are semantic data, not temporary comparison tuples.
first_path = (a: List(Int), b: List(Int)) -> Bool {
	a |> {
		[] -> True
		[x, ..xs] -> b |> {
			[] -> False
			[y, ..ys] -> { x == y } |> {
				True  -> first_path(xs, ys)
				False -> x < y
			}
		}
	}
}

active_epochs = (frames: List(Frame)) -> List(Cause) {
	frames |> {
		[] -> []
		[f, ..rest] -> f.cause |> {
			Absent     -> active_epochs(rest)
			Present(c) -> [c, ..active_epochs(rest)]
		}
	}
}

exemptions = (s: State, t: TaskState) {
	{ s.epoch == 0 && core.not(s.message_history) } |> {
		True  -> t.immune
		False -> append(t.immune, active_epochs(t.frames))
	}
}

frame_plan = (frames: List(Frame), cause: Cause) -> Maybe(CancelPlan) {
	frames |> {
		[] -> Absent
		[f, ..rest] -> { f.finishing } |> {
			True -> Absent
			False -> f.unwinding |> {
				Present(_) -> Absent
				Absent -> { contains(f.immune, cause) } |> {
					False -> Present(
						CancelPlan(
							[
								Frame(
									..f
									unwinding: Present(cause.epoch)
									cause:     Present(cause)
								)
								..rest
							]
							f.abort
						)
					)
					True -> frame_plan(rest, cause) |> {
						Absent -> Absent
						Present(plan) -> Present(
							CancelPlan([f, ..plan.frames], plan.abort)
						)
					}
				}
			}
		}
	}
}

cause_for_chain = (
	s: State
	t: TaskState
	chain: List(Address)
) -> Maybe(Cause) {
	chain |> {
		[] -> {
			t.cancelled_by |> {
				Present(c) -> Present(c)
				Absent     -> s.cancellation
			}
		} |> {
			Absent -> Absent
			Present(c) -> { cause_allowed(s, t, c) } |> {
				True  -> Present(c)
				False -> Absent
			}
		}
		[id, ..rest] -> {
			scope = find_scope(s.scopes, id)
			scope.cancellation |> {
				Absent -> cause_for_chain(s, t, rest)
				Present(c) -> {
					contains(t.immune, c) || core.not(cause_allowed(s, t, c))
				} |> {
					True -> cause_for_chain(s, t, rest)
					False -> frame_plan(t.frames, c) |> {
						Present(_) -> Present(c)
						Absent -> t.status |> {
							Dormant(_) -> Present(c)
							_          -> cause_for_chain(s, t, rest)
						}
					}
				}
			}
		}
	}
}

// A consumed fake cell is the shared one-shot transaction marker. It starts
// no producer and contributes nothing to program-Root quiescence accounting.
withdraw = (s: State, flag: Request(Registration)) -> State {
	request.state(flag) |> {
		request.Ready(row) -> {
			request.take(flag)
			counted = adjust_waiters(s, row.leaves)
			compact_sources(counted, row.leaves)
		}
		_ -> s
	}
}

remove_registration = (s: State, task_path: Int) {
	find_task(s.tasks, task_path).status |> {
		WaitingEvent(_, flag) -> withdraw(s, flag)
		_                     -> s
	}
}

waiter_live = (w: Waiter) {
	request.state(w.flag) |> {
		request.Ready(_) -> True
		_                -> False
	}
}

compact_queue = (q: WaitQueue) -> WaitQueue {
	{ q.total > q.live * 2 } |> {
		False -> q
		True -> { q.live == 0 } |> {
			True -> no_waiters
			False -> WaitQueue(
				append(
					filter(q.front, waiter_live)
					reverse(filter(q.back, waiter_live), [])
				)
				[]
				q.live
				q.live
			)
		}
	}
}

waiter_queue = (q: WaitQueue, waiter: Waiter) {
	WaitQueue(q.front, [waiter, ..q.back], q.total + 1, q.live + 1)
}

// Removed links retain no payload. Compact after all leaves of a transaction
// are counted, including duplicate leaves on one source. Deletions pay for
// each periodic compaction, bounding retained links by twice the live count.
queue_head = (
	q: WaitQueue
	excluded: Maybe(Request(Registration))
	skipped: List(Waiter)
) -> QueueHead {
	q.front |> {
		[] -> q.back |> {
			[] -> QueueHead(
				WaitQueue(reverse(skipped, []), [], q.total, q.live)
				Absent
			)
			_ -> queue_head(
				WaitQueue(reverse(q.back, []), [], q.total, q.live)
				excluded
				skipped
			)
		}
		[w, ..rest] -> { waiter_live(w) } |> {
			True -> {
				excluded |> {
					Absent        -> False
					Present(flag) -> flag == w.flag
				}
			} |> {
				True -> queue_head(
					WaitQueue(rest, q.back, q.total, q.live)
					excluded
					[w, ..skipped]
				)
				False -> QueueHead(
					WaitQueue(
						append(reverse(skipped, []), [w, ..rest])
						q.back
						q.total
						q.live
					)
					Present(w)
				)
			}
			False -> queue_head(
				WaitQueue(rest, q.back, q.total - 1, q.live)
				excluded
				skipped
			)
		}
	}
}

queue_live = (q: WaitQueue) { q.live > 0 }

adjust_waiters = (s: State, leaves: List(Leaf)) -> State {
	leaves |> {
		[] -> s
		[leaf, ..rest] -> {
			next = leaf |> {
				SendLeaf(info, _) -> {
					c = find_channel(s.channels, read(address_key, info.id))
					set_channel(
						s
						ChannelState(
							..c
							senders: WaitQueue(
								..c.senders
								live: c.senders.live - 1
							)
						)
					)
				}
				ReceiveLeaf(info) -> {
					c = find_channel(s.channels, read(address_key, info.id))
					set_channel(
						s
						ChannelState(
							..c
							receivers: WaitQueue(
								..c.receivers
								live: c.receivers.live - 1
							)
						)
					)
				}
				JoinLeaf(info) -> {
					t = find_task(s.tasks, read(task_key, info.id))
					t.outcome |> {
						Pending -> set_task(
							s
							TaskState(
								..t
								joiners: WaitQueue(
									..t.joiners
									live: t.joiners.live - 1
								)
							)
						)
						_ -> s
					}
				}
				AlwaysLeaf(_) -> s
			}
			adjust_waiters(next, rest)
		}
	}
}

compact_sources = (s: State, leaves: List(Leaf)) -> State {
	leaves |> {
		[] -> s
		[leaf, ..rest] -> {
			next = leaf |> {
				SendLeaf(info, _) -> {
					c = find_channel(s.channels, read(address_key, info.id))
					set_channel(
						s
						ChannelState(..c, senders: compact_queue(c.senders))
					)
				}
				ReceiveLeaf(info) -> {
					c = find_channel(s.channels, read(address_key, info.id))
					set_channel(
						s
						ChannelState(..c, receivers: compact_queue(c.receivers))
					)
				}
				JoinLeaf(info) -> {
					t = find_task(s.tasks, read(task_key, info.id))
					t.outcome |> {
						Pending -> set_task(
							s
							TaskState(..t, joiners: compact_queue(t.joiners))
						)
						_ -> s
					}
				}
				AlwaysLeaf(_) -> s
			}
			compact_sources(next, rest)
		}
	}
}

set_channel = (s: State, channel: ChannelState) {
	State(
		..s
		channels: store_update(
			s.channels
			address_index(channel.address)
			channel
		)
	)
}

queue = (s: State, task_path: Int) {
	task = find_task(s.tasks, task_path)
	{ task.queued } |> {
		True -> s
		False -> {
			next = set_task(s, TaskState(..task, queued: True))
			State(..next, ready_back: [task_path, ..next.ready_back])
		}
	}
}

has_ready = (s: State) { core.not(s.ready == [] && s.ready_back == []) }

reply = (
	s: State
	task_path: Int
	k: Continuation(StepAnswer, Turn, Never, Never)
	value: StepAnswer
	switch: Bool
) {
	t = find_task(s.tasks, task_path)
	next = set_task(s, TaskState(..t, status: Runnable(k, value)))
	{ switch } |> {
		True  -> queue(State(..next, current: Absent), task_path)
		False -> State(..next, current: Present(task_path))
	}
}

accept_cancel = (s: State, t: TaskState, c: Cause) -> State {
	{ contains(t.immune, c) || core.not(cause_allowed(s, t, c)) } |> {
		True -> s
		False -> t.status |> {
			Dormant(_) -> {
				next = set_task(
					s
					TaskState(
						..t
						status:  Ended
						outcome: Cancelled(c)
						clean:   True
						queued:  False
					)
				)
				State(
					..next
					ready:      without(next.ready, t.serial)
					ready_back: without(next.ready_back, t.serial)
				)
			}
			Ended | Running | Away -> s
			_ -> frame_plan(t.frames, c) |> {
				Absent -> s
				Present(plan) -> {
					terminal = c.run != s.run || {
						c.scope |> {
							Absent      -> True
							Present(id) -> contains(t.base, id)
						}
					}
					outcome = t.outcome |> {
						Pending -> { terminal } |> {
							True  -> Cancelled(c)
							False -> Pending
						}
						_ -> t.outcome
					}
					deferred = affected_nests(t, plan.frames) |> {
						[] -> False
						_  -> True
					}
					frames = list_map(plan.frames, (f: Frame) {
						f.cause |> {
							Present(cause) ? cause == c -> Frame(
								..f
								deferred: deferred
							)
							_ -> f
						}
					})
					updated = TaskState(..t, frames: frames, outcome: outcome)
					saved = set_task(s, updated)
					{ deferred } |> {
						False -> plan.frames |> {
							[] -> panic("task: missing cancellation delimiter")
							[frame, .._] -> frame.foreign |> {
								[] -> cancel_response(
									saved
									updated
									plan.abort
								)
								hops -> forward_response(
									saved
									updated
									BeginForeign(
										hops
										scope_token(saved, frame.scope)
										plan.abort
										c
									)
								)
							}
						}
						True -> t.driver |> {
							Present(id) -> notify_nested(saved, updated, id)
							Absent -> nested_abort(
								affected_nests(t, plan.frames)
							) |> {
								Absent -> saved
								Present(abort) -> cancel_response(
									saved
									updated
									abort
								)
							}
						}
					}
				}
			}
		}
	}
}

propagate_tasks = (s: State, rows: List(ActiveTask)) -> State {
	rows |> {
		[] -> s
		[entry, ..rest] -> store_find(s.tasks, entry.id) |> {
			Present(t) ? core.not(t.clean) -> {
				next = cause_for_chain(s, t, t.chain) |> {
					Absent     -> s
					Present(c) -> accept_cancel(s, t, c)
				}
				propagate_tasks(next, rest)
			}
			_ -> propagate_tasks(s, rest)
		}
	}
}

propagate = (s: State) {
	{ s.cancel_history || s.cancellation != Absent } |> {
		False -> s
		True  -> propagate_tasks(s, reverse(s.active, []))
	}
}

cancel_scope = (s: State, id: Address) -> State {
	scope = find_scope(s.scopes, id)
	scope.cancellation |> {
		Present(_) -> s
		Absent -> {
			epoch = s.epoch + 1
			set_scope(
				State(..s, epoch: epoch)
				ScopeState(
					..scope
					cancellation: Present(Cause(s.run, Present(id), epoch))
				)
			)
		}
	}
}

cancel_run = (s: State) -> State {
	s.cancellation |> {
		Present(_) -> s
		Absent -> State(
			..s
			epoch:        s.epoch + 1
			cancellation: Present(Cause(s.run, Absent, s.epoch + 1))
		)
	}
}

portable_witness = (w: Witness) { core.portable(w) }

check_scope = (s: State, t: TaskState, scope: ScopeInfo, message: String) {
	check_run(s.run, scope.run)
	{ contains(t.chain, read(address_key, scope.id)) } |> {
		True  -> Nil
		False -> panic(message)
	}
}

find_channel = (
	channels: TaskStore(ChannelState)
	id: Address
) -> ChannelState {
	store_find(channels, address_index(id)) |> {
		Present(c) -> c
		Absent     -> panic("task: channel is gone")
	}
}

check_channel = (s: State, info: ChannelInfo) {
	check_run(s.run, info.run)
	find_channel(s.channels, read(address_key, info.id))
	Nil
}

check_leaf_run = (s: State, leaf: Leaf) {
	leaf |> {
		SendLeaf(c, _) | ReceiveLeaf(c) -> check_run(s.run, c.run)
		JoinLeaf(t)                     -> check_run(s.run, t.scope.run)
		AlwaysLeaf(_)                   -> Nil
	}
}

check_leaf = (s: State, t: TaskState, leaf: Leaf) {
	leaf |> {
		SendLeaf(c, _) | ReceiveLeaf(c) -> check_channel(s, c)
		JoinLeaf(task) -> check_scope(
			s
			t
			task.scope
			"task: join outside its scope"
		)
		AlwaysLeaf(_) -> Nil
	}
}

check_leaves = (s: State, t: TaskState, leaves: List(Leaf)) {
	list_map(leaves, (leaf: Leaf) { check_leaf_run(s, leaf) })
	list_map(leaves, (leaf: Leaf) { check_leaf(s, t, leaf) })
	Nil
}

leaf_at = (leaves: List(Leaf), index: Int) -> Leaf {
	leaves |> {
		[] -> panic("task: invalid waiter leaf")
		[leaf, ..rest] -> { index == 0 } |> {
			True  -> leaf
			False -> leaf_at(rest, index - 1)
		}
	}
}

partner = (
	s: State
	leaf: Leaf
	excluded: Maybe(Request(Registration))
) -> Partner {
	leaf |> {
		SendLeaf(info, _) -> {
			c = find_channel(s.channels, read(address_key, info.id))
			head = queue_head(c.receivers, excluded, [])
			Partner(
				set_channel(s, ChannelState(..c, receivers: head.queue))
				head.waiter
			)
		}
		ReceiveLeaf(info) -> {
			c = find_channel(s.channels, read(address_key, info.id))
			head = queue_head(c.senders, excluded, [])
			Partner(
				set_channel(s, ChannelState(..c, senders: head.queue))
				head.waiter
			)
		}
		_ -> panic("task: invalid channel alternative")
	}
}

commit_partner = (s: State, waiter: Waiter, payload: Keyed) -> State {
	row = request.state(waiter.flag) |> {
		request.Ready(row) -> row
		_                  -> panic("task: stale partner")
	}
	next = withdraw(s, waiter.flag)
	t = find_task(next.tasks, row.task)
	t.status |> {
		WaitingEvent(k, _) -> reply(
			next
			t.serial
			k
			Proceed(key.wrap(selection_key, Selected(waiter.index, payload)))
			True
		)
		_ -> panic("task: invalid event registration")
	}
}

probe_leaf = (
	s: State
	leaf: Leaf
	index: Int
	excluded: Maybe(Request(Registration))
) -> Commit {
	leaf |> {
		AlwaysLeaf(value) -> Commit(s, Present(Selected(index, value)))
		JoinLeaf(task) -> {
			target = find_task(s.tasks, read(task_key, task.id))
			target.outcome |> {
				Pending -> Commit(s, Absent)
				Success(value) | Failure(value) -> Commit(
					s
					Present(Selected(index, value))
				)
				Cancelled(_) -> Commit(s, Present(SelectedCancelled))
			}
		}
		SendLeaf(_, value) -> {
			p = partner(s, leaf, excluded)
			p.waiter |> {
				Absent -> Commit(p.state, Absent)
				Present(w) -> Commit(
					commit_partner(p.state, w, value)
					Present(Selected(index, unit()))
				)
			}
		}
		ReceiveLeaf(_) -> {
			p = partner(s, leaf, excluded)
			p.waiter |> {
				Absent -> Commit(p.state, Absent)
				Present(w) -> {
					row = request.state(w.flag) |> {
						request.Ready(r) -> r
						_                -> panic("task: stale waiter")
					}
					value = leaf_at(row.leaves, w.index) |> {
						SendLeaf(_, value) -> value
						_                  -> panic("task: invalid sender")
					}
					Commit(
						commit_partner(p.state, w, unit())
						Present(Selected(index, value))
					)
				}
			}
		}
	}
}

probe = (
	s: State
	leaves: List(Leaf)
	index: Int
	excluded: Maybe(Request(Registration))
) -> Commit {
	leaves |> {
		[] -> Commit(s, Absent)
		[leaf, ..rest] -> {
			result = probe_leaf(s, leaf, index, excluded)
			result.selection |> {
				Absent -> probe(result.state, rest, index + 1, excluded)
				_      -> result
			}
		}
	}
}

register_leaves = (
	s: State
	flag: Request(Registration)
	leaves: List(Leaf)
	index: Int
) -> State {
	leaves |> {
		[] -> s
		[leaf, ..rest] -> {
			waiter = Waiter(flag, index)
			next = leaf |> {
				SendLeaf(info, _) -> {
					c = find_channel(s.channels, read(address_key, info.id))
					set_channel(
						s
						ChannelState(
							..c
							senders: waiter_queue(c.senders, waiter)
						)
					)
				}
				ReceiveLeaf(info) -> {
					c = find_channel(s.channels, read(address_key, info.id))
					set_channel(
						s
						ChannelState(
							..c
							receivers: waiter_queue(c.receivers, waiter)
						)
					)
				}
				JoinLeaf(info) -> {
					target = find_task(s.tasks, read(task_key, info.id))
					set_task(
						s
						TaskState(
							..target
							joiners: waiter_queue(target.joiners, waiter)
						)
					)
				}
				AlwaysLeaf(_) -> panic("task: always cannot block")
			}
			register_leaves(next, flag, rest, index + 1)
		}
	}
}

select = (
	s: State
	t: TaskState
	k: Continuation(StepAnswer, Turn, Never, Never)
	leaves: List(Leaf)
) {
	check_leaves(s, t, leaves)
	result = probe(s, leaves, 0, Absent)
	result.selection |> {
		Present(selected) -> reply(
			result.state
			t.serial
			k
			Proceed(key.wrap(selection_key, selected))
			True
		)
		Absent -> {
			id = s.registration + 1
			flag = request.ready(Registration(id, t.serial, leaves))
			next = set_task(
				result.state
				TaskState(..t, status: WaitingEvent(k, flag))
			)
			register_leaves(
				State(..next, current: Absent, registration: id)
				flag
				leaves
				0
			)
		}
	}
}

wake_source_waiters = (s: State, rows: List(Waiter)) -> State {
	rows |> {
		[] -> s
		[w, ..rest] -> request.state(w.flag) |> {
			request.Ready(row) -> {
				result = probe(s, row.leaves, 0, Present(w.flag))
				next = result.selection |> {
					Absent -> panic(
						"task: completed source did not become ready"
					)
					Present(selected) -> {
						released = withdraw(result.state, w.flag)
						t = find_task(released.tasks, row.task)
						t.status |> {
							WaitingEvent(k, _) -> reply(
								released
								t.serial
								k
								Proceed(key.wrap(selection_key, selected))
								True
							)
							_ -> panic("task: invalid join waiter")
						}
					}
				}
				wake_source_waiters(next, rest)
			}
			_ -> wake_source_waiters(s, rest)
		}
	}
}

wake_source_rows = (s: State, rows: List(Int)) -> State {
	rows |> {
		[] -> s
		[path, ..rest] -> {
			t = find_task(s.tasks, path)
			next = set_task(s, TaskState(..t, joiners: no_waiters))
			wake_source_rows(
				wake_source_waiters(
					next
					append(t.joiners.front, reverse(t.joiners.back, []))
				)
				rest
			)
		}
	}
}

wake_sources = (s: State) {
	wake_source_rows(
		State(..s, changed_sources: [])
		reverse(s.changed_sources, [])
	)
}

wake_tickets = (s: State, paths: List(Int)) -> State {
	paths |> {
		[] -> s
		[p, ..rest] -> {
			t = find_task(s.tasks, p)
			next = t.status |> {
				WaitingTicket(k, tickets) -> {
					any(tickets, request.settled)
				} |> {
					False -> s
					True -> queue(
						set_task(
							State(..s, waiting: without(s.waiting, p))
							TaskState(..t, status: Runnable(k, Proceed(unit())))
						)
						p
					)
				}
				_ -> State(..s, waiting: without(s.waiting, p))
			}
			wake_tickets(next, rest)
		}
	}
}

lowest_error = (
	errors: List((List(Int), Keyed))
	best: (List(Int), Keyed)
) -> Keyed {
	errors |> {
		[] -> {
			(_, value) = best
			value
		}
		[candidate, ..rest] -> {
			(a, _) = candidate
			(b, _) = best
			{ first_path(a, b) } |> {
				True  -> lowest_error(rest, candidate)
				False -> lowest_error(rest, best)
			}
		}
	}
}

scope_reply = (scope: ScopeState) -> ScopeReply {
	scope.errors |> {
		[first, ..rest] -> ScopeError(lowest_error(rest, first))
		[] -> scope.winner |> {
			Absent         -> ScopeOk
			Present(value) -> ScopeWinner(value)
		}
	}
}

refresh_rows = (s: State, rows: List(Int)) -> State {
	rows |> {
		[] -> s
		[path, ..rest] -> {
			t = find_task(s.tasks, path)
			{ t.open_scopes > 0 } |> {
				True -> refresh_rows(s, rest)
				False -> refresh_rows(
					set_task(s, TaskState(..t, clean: True))
					rest
				)
			}
		}
	}
}

refresh_clean = (s: State) { refresh_rows(s, s.unclean) }

owned_channel_bad = (s: State, ids: List(Address)) -> Bool {
	ids |> {
		[] -> False
		[id, ..rest] -> {
			c = find_channel(s.channels, id)
			queue_live(c.senders) ||
				queue_live(c.receivers) ||
				owned_channel_bad(s, rest)
		}
	}
}

owned_indices = (ids: List(Address)) -> List(Int) {
	ids |> {
		[]           -> []
		[id, ..rest] -> [address_index(id), ..owned_indices(rest)]
	}
}

retire_scope = (s: State, scope: ScopeState) -> State {
	bad = owned_channel_bad(s, scope.owned_channels)
	{ bad } |> {
		True  -> panic("task: channel is gone")
		False -> Nil
	}
	{ worker_enabled() } |> {
		True  -> drop_shares(s, scope.owned_shares)
		False -> Nil
	}
	next = State(
		..s
		scopes:   drop_children(s.scopes, [address_index(scope.address)])
		channels: drop_children(s.channels, owned_indices(scope.owned_channels))
		shares:   drop_children(s.shares, owned_indices(scope.owned_shares))
		tasks:    drop_children(s.tasks, scope.children)
	)
	body = find_task(next.tasks, scope.body)
	body = TaskState(..body, open_scopes: body.open_scopes - 1)
	next = set_task(next, body)
	body.status |> {
		WaitingScope(k, id) ? id == scope.address -> queue(
			set_task(
				next
				TaskState(
					..body
					status: Runnable(k, scope_end_reply(next, body, scope))
				)
			)
			body.serial
		)
		_ -> queue_scope_end(next, body, scope)
	}
}

retire_scopes = (s: State) -> State {
	s.dirty_scopes |> {
		[] -> s
		[id, ..rest] -> {
			scope = find_scope(s.scopes, id)
			next = retire_scope(State(..s, dirty_scopes: rest), scope)
			retire_scopes(refresh_clean(next))
		}
	}
}

maintain = (s: State) -> State {
	cancelled = propagate(pump_messages(propagate(s)))
	tickets = wake_tickets(cancelled, cancelled.waiting)
	events = wake_sources(tickets)
	retire_scopes(refresh_clean(events))
}

new_address = (t: TaskState) { Address(t.serial, t.handles + 1) }

head_scope = (chain: List(Address)) {
	chain |> {
		[]        -> Absent
		[id, .._] -> Present(id)
	}
}

find_share = (shares: TaskStore(ShareState), id: Address) -> Keyed {
	store_find(shares, address_index(id)) |> {
		Present(row) -> row.value
		Absent       -> panic("task: shared value is gone")
	}
}

prevalidate = (s: State, t: TaskState, command: Command) {
	command |> {
		Spawn(info, _) | NewShare(info, _) -> check_scope(
			s
			t
			info
			"task: go outside its scope"
		)
		Select(leaves)   -> check_leaves(s, t, leaves)
		SendNow(info, _) -> check_channel(s, info)
		GetShare(run, _, id) | UpdateRef(run, id, _) -> {
			check_run(s.run, run)
			find_share(s.shares, read(address_key, id))
			Nil
		}
		_ -> Nil
	}
}

add_forward = (
	rows: List(Forward)
	id: Key(Nil)
	tickets: List(Ticket)
) -> List(Forward) {
	rows |> {
		[] -> [Forward(id, tickets)]
		[r, ..rest] -> { r.id == id } |> {
			True  -> [Forward(id, tickets), ..rest]
			False -> [r, ..add_forward(rest, id, tickets)]
		}
	}
}

end_frame = (frames: List(Frame), id: Address) -> List(Frame) {
	frames |> {
		[frame, ..rest] -> frame.scope |> {
			Present(owner) ? owner == id -> rest
			_                            -> panic("task: invalid scope delimiter")
		}
		[] -> panic("task: missing scope delimiter")
	}
}

do_command = (
	s: State
	t: TaskState
	k: Continuation(StepAnswer, Turn, Never, Never)
	command: Command
) -> State {
	command |> {
		Register(owner, abort, interrupt, relay, final) -> {
			foreign = t.nested |> {
				[]         -> []
				[row, .._] -> row.active.hops
			}
			frame = Frame(
				owner
				abort
				interrupt
				relay
				foreign
				exemptions(s, t)
				final
				{ worker_away(k) } |> {
					True  -> s.register_flag
					False -> final(k)
				}
				Absent
				False
				Absent
			)
			next = set_task(s, TaskState(..t, frames: [frame, ..t.frames]))
			owner |> {
				Absent -> next
				Present(id) -> {
					scope = find_scope(next.scopes, id)
					set_scope(next, ScopeState(..scope, foreign: foreign))
				}
			}
		}
		ExternalCancel -> cancel_run(s)
		Relay(notice, value, discard) -> relay_notice(
			s
			t
			k
			notice
			value
			discard
		)
		FinishScope(info) -> finish_scope(s, t, k, info)
		NextRun -> {
			index = t.spawn + 1
			child_path = append(t.path, [index])
			next = set_task(s, TaskState(..t, spawn: index))
			reply(
				next
				t.serial
				k
				Proceed(key.wrap(path_key, child_path))
				False
			)
		}
		Spawn(info, start) -> {
			scope = find_scope(s.scopes, read(address_key, info.id))
			index = t.spawn + 1
			child_path = append(t.path, [index])
			immune = exemptions(s, t)
			(status, outcome, clean) = scope.cancellation |> {
				Absent         -> (Dormant(publish(s, child_path, start)), Pending, False)
				Present(cause) -> (Ended, Cancelled(cause), True)
			}
			child = TaskState(
				child_path
				s.task_serial + 1
				request.ready(Nil)
				False
				Present(scope.address)
				Present(info)
				scope.chain
				scope.chain
				[]
				immune
				0
				0
				[]
				Absent
				[]
				[]
				Absent
				status
				outcome
				clean
				no_waiters
				0
				scope.cleanup
			)
			opener = TaskState(..t, spawn: index)
			next = add_task(
				set_task(State(..s, task_serial: s.task_serial + 1), opener)
				child
			)
			recorded = set_scope(
				next
				ScopeState(
					..scope
					children: [child.serial, ..scope.children]
					pending: { clean } |> {
						True  -> scope.pending
						False -> scope.pending + 1
					}
				)
			)
			queued = { clean } |> {
				True  -> recorded
				False -> queue(recorded, child.serial)
			}
			reply(
				queued
				t.serial
				k
				Proceed(key.wrap(task_key, child.serial))
				False
			)
		}
		NewScope(witness, mode) -> {
			id = new_address(t)
			info = ScopeInfo(s.run, key.wrap(address_key, id), witness)
			chain = [id, ..t.chain]
			row = ScopeState(
				info
				id
				t.serial
				chain
				exemptions(s, t)
				t.cleanup || foreign_active(t) || any(t.frames, (f: Frame) {
					{ f.finishing } || {
						f.unwinding |> {
							Present(_) -> True
							Absent     -> False
						}
					}
				})
				[]
				key.new()
				[]
				0
				BodyOpen
				[]
				Absent
				mode
				Absent
				False
				t.cleanup
				[]
				[]
			)
			next = set_task(
				s
				TaskState(
					..t
					chain:       chain
					handles:     t.handles + 1
					open_scopes: t.open_scopes + 1
					cleanup:     row.cleanup
				)
			)
			reply(
				State(
					..next
					scopes: store_add(next.scopes, address_index(id), row)
				)
				t.serial
				k
				Proceed(key.wrap(scope_key, info))
				False
			)
		}
		EndScope(info, completion) -> {
			id = read(address_key, info.id)
			scope = find_scope(s.scopes, id)
			(mark, errors) = completion |> {
				Succeeded(_) -> (BodyOk, scope.errors)
				Failed(_, error) -> scope.winner |> {
					Absent -> (
						BodyError
						append(scope.errors, [(t.path, error)])
					)
					Present(_) -> (BodyError, scope.errors)
				}
				Abandoned -> (BodyCancelled, scope.errors)
			}
			body = TaskState(
				..t
				frames:  end_frame(t.frames, id)
				cleanup: scope.prior_cleanup
				chain:   without(t.chain, id)
				status:  WaitingScope(k, id)
			)
			next = set_scope(
				set_task(State(..s, current: Absent), body)
				ScopeState(..scope, mark: mark, errors: errors)
			)
			mark |> {
				BodyError | BodyCancelled -> cancel_scope(next, id)
				_                         -> next
			}
		}
		Yield -> queue(State(..s, current: Absent), t.serial)
		Wait(tickets) | NestedWait(_, tickets) -> {
			{ any(tickets, request.settled) } |> {
				True -> queue(State(..s, current: Absent), t.serial)
				False -> {
					next = set_task(
						s
						TaskState(..t, status: WaitingTicket(k, tickets))
					)
					State(
						..next
						current: Absent
						waiting: append(
							without(next.waiting, t.serial)
							[t.serial]
						)
					)
				}
			}
		}
		NestedActive(_, _) -> s
		ForwardWait(id, tickets) -> {
			next = State(
				..s
				forwards: add_forward(s.forwards, id, tickets)
				current:  Absent
			)
			queue(next, t.serial)
		}
		ForgetWait(id) -> set_task(
			State(..s, forwards: filter(s.forwards, (row: Forward) {
				row.id != id
			}))
			TaskState(
				..t
				nested: filter(t.nested, (row: Nest) { row.id != id })
				driver: Absent
			)
		)
		NewChannel(witness) -> {
			id = new_address(t)
			info = ChannelInfo(s.run, key.wrap(address_key, id), witness)
			row = ChannelState(
				info
				id
				head_scope(t.chain)
				no_waiters
				no_waiters
			)
			next = set_task(s, TaskState(..t, handles: t.handles + 1))
			next = State(
				..next
				channels: store_add(next.channels, address_index(id), row)
			)
			next = row.owner |> {
				Absent -> next
				Present(owner) -> {
					scope = find_scope(next.scopes, owner)
					set_scope(
						next
						ScopeState(
							..scope
							owned_channels: [id, ..scope.owned_channels]
						)
					)
				}
			}
			reply(
				next
				t.serial
				k
				Proceed(key.wrap(channel_key, info))
				False
			)
		}
		SendNow(info, value) -> {
			{ portable_witness(info.witness) } |> {
				True  -> portable(value)
				False -> Nil
			}
			select(s, t, k, [SendLeaf(info, value)])
		}
		Select(leaves) -> select(s, t, k, leaves)
		NewShare(info, value) -> {
			portable(value)
			id = new_address(t)
			{ worker_enabled() } |> {
				True  -> worker_share_put(s.run, id, value)
				False -> Nil
			}
			row = ShareState(id, Present(read(address_key, info.id)), value)
			next = set_task(s, TaskState(..t, handles: t.handles + 1))
			scope = find_scope(next.scopes, read(address_key, info.id))
			next = set_scope(
				State(
					..next
					shares: store_add(next.shares, address_index(id), row)
				)
				ScopeState(..scope, owned_shares: [id, ..scope.owned_shares])
			)
			reply(next, t.serial, k, Proceed(key.wrap(address_key, id)), False)
		}
		NewRef(value) -> {
			portable(value)
			id = new_address(t)
			owner = head_scope(t.chain)
			row = ShareState(id, owner, value)
			next = set_task(
				State(..s, shares: store_add(s.shares, address_index(id), row))
				TaskState(..t, handles: t.handles + 1)
			)
			next = owner |> {
				Absent -> next
				Present(owner) -> {
					scope = find_scope(next.scopes, owner)
					set_scope(
						next
						ScopeState(
							..scope
							owned_shares: [id, ..scope.owned_shares]
						)
					)
				}
			}
			reply(next, t.serial, k, Proceed(key.wrap(address_key, id)), False)
		}
		UpdateRef(_, id, change) -> {
			address = read(address_key, id)
			row = store_find(s.shares, address_index(address)) |> {
				Present(row) -> row
				Absent       -> panic("task: shared value is gone")
			}
			(value, answer) = change(row.value)
			portable(value)
			next = State(
				..s
				shares: store_update(
					s.shares
					address_index(address)
					ShareState(..row, value: value)
				)
			)
			reply(next, t.serial, k, Proceed(answer), False)
		}
		GetShare(_, _, id) -> reply(
			s
			t.serial
			k
			Proceed(find_share(s.shares, read(address_key, id)))
			False
		)
		Drain -> set_task(
			State(..s, current: Absent)
			TaskState(..t, status: WaitingDrain(k))
		)
	}
}

cancellation_reply = (t: TaskState) {
	t.status |> {
		Runnable(_, CancelStep(_)) -> True
		_                          -> False
	}
}

finals_changed = (
	frames: List(Frame)
	k: Continuation(StepAnswer, Turn, Never, Never)
) -> Bool {
	frames |> {
		[]          -> False
		[f, ..rest] -> f.finishing != f.final(k) || finals_changed(rest, k)
	}
}

update_finals = (
	frames: List(Frame)
	k: Continuation(StepAnswer, Turn, Never, Never)
) -> List(Frame) {
	frames |> {
		[] -> []
		[f, ..rest] -> [
			Frame(..f, finishing: f.final(k))
			..update_finals(rest, k)
		]
	}
}

on_command = (
	s: State
	task_path: Int
	command: Command
	k: Continuation(StepAnswer, Turn, Never, Never)
	flags: Maybe(List(Bool))
) {
	original = find_task(s.tasks, task_path)
	updated = flags |> {
		Present(answers) -> {
			(own, extra) = split_flags(original.frames, answers)
			set_task(
				State(..s, register_flag: extra == [True])
				TaskState(..original, frames: apply_finals(original.frames, own))
			)
		}
		Absent -> { finals_changed(original.frames, k) } |> {
			False -> s
			True -> set_task(
				s
				TaskState(..original, frames: update_finals(original.frames, k))
			)
		}
	}
	t = find_task(updated.tasks, task_path)
	prevalidate(updated, t, command)
	prepared = register_driver(
		reply(updated, task_path, k, Proceed(unit()), False)
		task_path
		command
	)
	incoming = find_task(prepared.tasks, task_path)
	next = command |> {
		Register(..)
			| Spawn(..)
			| EndScope(..)
			| ForgetWait(..)
			| ExternalCancel
			| Relay(..)
			| FinishScope(..) -> propagate(
			do_command(prepared, incoming, k, command)
		)
		_ -> {
			checked = propagate(prepared)
			caller = find_task(checked.tasks, task_path)
			{ cancellation_reply(caller) } |> {
				True  -> checked
				False -> do_command(checked, caller, k, command)
			}
		}
	}
	deliver_deferred(next, task_path, command)
}

observe_completion = (s: State, t: TaskState, completion: Completion) -> State {
	outcome = t.outcome |> {
		Pending -> completion |> {
			Succeeded(value) -> Success(value)
			Failed(value, _) -> Failure(value)
			Abandoned        -> Cancelled(Cause(s.run, Absent, s.epoch))
		}
		_ -> t.outcome
	}
	next = set_task(
		State(..s, current: Absent)
		TaskState(..t, status: Ended, frames: [], outcome: outcome)
	)
	t.parent |> {
		Absent -> next
		Present(id) -> {
			scope = find_scope(next.scopes, id)
			t.outcome |> {
				Pending -> completion |> {
					Failed(_, error) -> scope.winner |> {
						Present(_) -> next
						Absent -> cancel_scope(
							set_scope(
								next
								ScopeState(
									..scope
									errors: append(
										scope.errors
										[(t.path, error)]
									)
								)
							)
							id
						)
					}
					Succeeded(value) -> (
						scope.mode
						scope.winner
						scope.errors
					) |> {
						(Racing, Absent, []) -> cancel_scope(
							set_scope(
								next
								ScopeState(..scope, winner: Present(value))
							)
							id
						)
						_ -> next
					}
					Abandoned -> next
				}
				_ -> next
			}
		}
	}
}

turn = (s: State, t: TaskState) -> Turn {
	// Project before the callback: suspended tasks must not retain State or a
	// TaskState snapshot through the boundary body's closure environment.
	t.status |> {
		Dormant(start) ? start.ticket != 0 && core.not(worker_claim(start.ticket)) -> {
			worker_adopt(start.ticket, t.serial)
			Sent(t.serial)
		}
		Dormant(start) -> {
			path = t.path
			id = t.serial
			run = s.run
			root = s.root
			runner = s.runner
			body = start.body
			control.boundary((b) {
				own = make_scheduler(b, path, id, run, root, runner)
				Finished(id, body(own))
			})
		}
		Runnable(k, value) ? worker_away(k) -> {
			worker_send(t.serial, Resume(value, frame_finals(t.frames), s.run))
			Sent(t.serial)
		}
		Runnable(k, value) -> control.resume(k, value)
		_                  -> panic("task: task is not runnable")
	}
}

interests_tasks = (s: State, paths: List(Int)) -> List(Ticket) {
	paths |> {
		[] -> []
		[path, ..rest] -> find_task(s.tasks, path).status |> {
			WaitingTicket(_, tickets) -> append(
				tickets
				interests_tasks(s, rest)
			)
			_ -> interests_tasks(s, rest)
		}
	}
}

interests_forward = (rows: List(Forward)) -> List(Ticket) {
	rows |> {
		[]            -> []
		[row, ..rest] -> append(row.tickets, interests_forward(rest))
	}
}

interests = (s: State) {
	all = append(interests_tasks(s, s.waiting), interests_forward(s.forwards))
	filter(all, (ticket: Ticket) { core.not(request.settled(ticket)) })
}

apply_run_control = (s: State, result: RunControl) {
	result |> {
		ContinueRun        -> s
		ForwardRun(notice) -> post_notice(s, notice)
		ParentCancel(abort) -> cancel_run(
			State(..s, outer_cancel: Present(abort))
		)
	}
}

finished_run = (s: State) { s.active_count == 0 }

run_reply = (s: State) -> RunReply {
	s.outer_cancel |> {
		Present(abort) -> RunCancelled(abort)
		Absent -> s.failure |> {
			Present(p) -> RunError(p)
			Absent -> find_task(s.tasks, s.root_id).outcome |> {
				Success(v) | Failure(v) -> RunOk(v)
				_                       -> panic("task: missing root result")
			}
		}
	}
}

script_rows = (
	s: State
	ids: List(ActiveTask)
	wanted: List(Int)
) -> Maybe(TaskState) {
	ids |> {
		[] -> Absent
		[entry, ..rest] -> store_find(s.tasks, entry.id) |> {
			Present(t) ? t.path == wanted -> Present(t)
			_                             -> script_rows(s, rest, wanted)
		}
	}
}

script_task = (s: State, wanted: List(Int)) -> Maybe(TaskState) {
	script_rows(s, s.active, wanted)
}

choose_turn = (state: State) -> State {
	s = (state.ready, state.ready_back) |> {
		([], [_, .._]) -> State(
			..state
			ready:      reverse(state.ready_back, [])
			ready_back: []
		)
		_ -> state
	}
	s.current |> {
		Present(_) -> s
		Absent -> s.choices |> {
			[] -> s.ready |> {
				[] -> s
				[p, ..rest] -> {
					focused = State(..s, tasks: focus_task(s.tasks, p))
					t = find_task(focused.tasks, p)
					next = set_task(focused, TaskState(..t, queued: False))
					State(..next, current: Present(p), ready: rest)
				}
			}
			[wanted, ..rest] -> script_task(s, wanted) |> {
				Present(t) ? t.queued -> {
					focused = State(..s, tasks: focus_task(s.tasks, t.serial))
					next = set_task(focused, TaskState(..t, queued: False))
					State(
						..next
						current:    Present(t.serial)
						ready:      without(s.ready, t.serial)
						ready_back: without(s.ready_back, t.serial)
						choices:    rest
					)
				}
				_ -> cancel_run(
					State(..s, failure: Present(wanted), choices: [])
				)
			}
		}
	}
}

loop = (state: State) -> RunReply {
	s = maintain(pull(state))
	{ finished_run(s) } |> {
		True -> run_reply(s)
		False -> {
			runner = s.runner
			runner |> {
				DetachedRunner -> panic("task: no scheduler")
				Runner(..) -> {
					work = interests(s)
					{ s.scripted && any(work, has_work) } |> {
						True  -> panic("task: test needs fake IO")
						False -> Nil
					}
					polled = s.current |> {
						Present(_) ? work == [] -> s
						_ -> apply_run_control(
							s
							runner.poll(
								s.run
								work
								s.current != Absent || has_ready(s)
							)
						)
					}

					after_poll = {
						work == [] &&
							polled.epoch == s.epoch &&
							core.not(polled.message_history) &&
							polled.cancellation == s.cancellation
					} |> {
						True  -> polled
						False -> maintain(polled)
					}
					chosen = choose_turn(after_poll)
					prepared = { chosen.epoch == after_poll.epoch } |> {
						True  -> chosen
						False -> choose_turn(maintain(chosen))
					}
					prepared.current |> {
						Present(p) -> {
							t = find_task(prepared.tasks, p)
							runner.active(
								prepared.run
								effective_active(prepared, t)
							) |> {
								ParentCancel(abort) -> loop(
									cancel_run(
										State(
											..prepared
											outer_cancel: Present(abort)
										)
									)
								)
								ForwardRun(notice) -> loop(
									post_notice(prepared, notice)
								)
								ContinueRun -> {

									running = set_task(
										State(
											..prepared
											current: Absent
											ready: { t.queued } |> {
												True -> without(
													prepared.ready
													p
												)
												False -> prepared.ready
											}
											ready_back: { t.queued } |> {
												True -> without(
													prepared.ready_back
													p
												)
												False -> prepared.ready_back
											}
										)
										TaskState(
											..t
											status: Running
											queued: False
										)
									)
									turn(running, t) |> {
										Switched(path, command, k) -> loop(
											on_command(
												running
												path
												command
												k
												Absent
											)
										)
										Sent(path) -> loop(
											set_task(
												running
												TaskState(
													..find_task(running.tasks, path)
													status: Away
												)
											)
										)
										Finished(path, completion) -> loop(
											observe_completion(
												running
												find_task(running.tasks, path)
												completion
											)
										)
									}
								}
							}
						}
						Absent -> {
							{ finished_run(prepared) } |> {
								True -> run_reply(prepared)
								False -> {
									tickets = interests(prepared)
									// A nested run also waits while a task of the
									// entry run is away: its next step may end it.
									possible = request.count_root(
										prepared.root
									) >
										0 ||
										any(tickets, has_work) ||
										core.not(prepared.top) && worker_pending()
									away = prepared.top && worker_pending()
									{ away } |> {
										True -> {
											worker_wait()
											loop(
												apply_run_control(
													prepared
													runner.poll(prepared.run, tickets, True)
												)
											)
										}
										False -> { possible } |> {
										False -> panic("task: deadlock")
										True -> {
											{ prepared.scripted } |> {
												True -> panic(
													"task: test needs fake IO"
												)
												False -> Nil
											}
											loop(
												apply_run_control(
													prepared
													runner.wait(
														prepared.run
														prepared.root
														tickets
													)
												)
											)
										}
									}
									}
								}
							}
						}
					}
				}
			}
		}
	}
}

scope_call_body(A, E, N) = (
	body: (Scope(E)) -> A ^ E ! N
	witness: Witness
	mode: Mode
	own: Scheduler
) -> ScopeResult(A) ^ E ! N {
	info = read(scope_key, ask(own, NewScope(witness, mode)))
	errors: Key(E) = key.new()
	results: Key(Result(A, E)) = key.new()
	handle = Scope(info, errors)
	checked = portable_witness(info.witness)
	kept = control.keep_env((_: Nil) {
		fail.catch(() { body(handle) }) |> {
			Ok(value) -> Succeeded(key.wrap(results, Ok(value)))
			Error(error) -> {
				{ checked } |> {
					True  -> portable(error)
					False -> Nil
				}
				Failed(key.wrap(results, Error(error)), key.wrap(errors, error))
			}
		}
	})
	resource.ensure(() {
		completed = guarded(own, Present(read(address_key, info.id)), () {
			kept(Nil)
		})
		mark = completed |> {
			Succeeded(_) -> Succeeded(unit())
			other        -> other
		}
		read(scope_reply_key, ask(own, EndScope(info, mark))) |> {
			ScopeError(error)  -> fail.raise(read(errors, error))
			ScopeWinner(value) -> WinnerValue(value)
			ScopeOk -> completed |> {
				Succeeded(value) -> BodyValue(
					fail.from_result(read(results, value))
				)
				_ -> panic("task: missing scope result")
			}
		}
	}, () {
		ask(own, FinishScope(info))
		Nil
	})
}

scope(A, E, N) = (
	body: (s: Scope(E)) -> A ^ E ! N
	own ! Scheduler
) -> A ^ E ! N | Scheduler {
	scope_call(body, Portable, Ordinary, own) |> {
		BodyValue(value) -> value
		WinnerValue(_)   -> panic("task: invalid scope winner")
	}
}

local_scope(A, E, N) = (
	body: (s: Scope(E)) -> A ^ E ! N
	own ! Scheduler
) -> A ^ E ! N | Scheduler {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..) -> scope_call(body, own.witness, Ordinary, own) |> {
			BodyValue(value) -> value
			WinnerValue(_)   -> panic("task: invalid scope winner")
		}
	}
}

go_key(A, E, N) = (
	s: Scope(E)
	body: () -> A ^ E ! N
	result: Key(Result(A, E))
	own ! Scheduler
) -> Task(A, E) ! Scheduler | N - Scheduler - Runner - request.Await {
	start = make_start(body, result, s.errors, portable_witness(s.info.witness))
	id = ask(own, Spawn(s.info, start))
	Task(TaskInfo(s.info, id), result, s)
}

go(A, E, N) = (
	s: Scope(E)
	body: () -> A ^ E ! N
) -> Task(A, E) ! Scheduler | N - Scheduler - Runner - request.Await {
	result: Key(Result(A, E)) = key.new()
	go_key(s, body, result)
}

branch_at(A, E, N) = (
	branches: List(Branch(A, E, N))
	index: Int
) -> Branch(A, E, N) {
	branches |> {
		[] -> panic("task: invalid selected alternative")
		[branch, ..rest] -> { index == 0 } |> {
			True  -> branch
			False -> branch_at(rest, index - 1)
		}
	}
}

sync_body(A, E, N) = (e: Event(A, E, N), own: Scheduler) -> A ^ E ! N {
	leaves = list_map(e.branches, (branch: Branch(A, E, N)) { branch.leaf })
	read(selection_key, ask(own, Select(leaves))) |> {
		Selected(index, value) -> {
			branch = branch_at(e.branches, index)
			branch.decode(value)
		}
		SelectedCancelled -> panic("task: join of a cancelled task")
	}
}

always(A) = (value: A) -> Event(A, Never, Never) {
	values: Key(A) = key.new()
	Event(key.new(), [Branch(AlwaysLeaf(key.wrap(values, value)), (v: Keyed) {
		read(values, v)
	})])
}

never = () -> Event(Never, Never, Never) { Event(key.new(), []) }

branches_of(A, E, N) = (events: List(Event(A, E, N))) -> List(Branch(A, E, N)) {
	events |> {
		[]          -> []
		[e, ..rest] -> append(e.branches, branches_of(rest))
	}
}

choose(A, E, N) = (events: List(Event(A, E, N))) -> Event(A, E, N) {
	Event(key.new(), branches_of(events))
}

map(A, B, E, F, N, M) = (
	e: Event(A, E, N)
	f: (A) -> B ^ F ! M
) -> Event(B, E | F, N | M) {
	Event(key.new(), list_map(e.branches, (branch: Branch(A, E, N)) {
		Branch(branch.leaf, (value: Keyed) { f(branch.decode(value)) })
	}))
}

join_event(A, E) = (t: Task(A, E)) -> Event(A, E, Never) {
	Event(key.new(), [Branch(JoinLeaf(t.info), (value: Keyed) {
		fail.from_result(read(t.result, value))
	})])
}

join(A, E) = (t: Task(A, E)) -> A ^ E ! Scheduler { sync(join_event(t)) }

channel(A) = (own ! Scheduler) -> Channel(A) ! Scheduler {
	Channel(read(channel_key, ask(own, NewChannel(Portable))), key.new())
}

local_channel(A) = (own ! Scheduler) -> Channel(A) ! Scheduler {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..) -> Channel(
			read(channel_key, ask(own, NewChannel(own.witness)))
			key.new()
		)
	}
}

send_event(A) = (c: Channel(A), value: A) -> Event(Nil, Never, Never) {
	{ portable_witness(c.info.witness) } |> {
		True  -> portable(value)
		False -> Nil
	}
	Event(
		key.new()
		[Branch(SendLeaf(c.info, key.wrap(c.values, value)), (v: Keyed) {
			read(unit_key, v)
		})]
	)
}

receive_event(A) = (c: Channel(A)) -> Event(A, Never, Never) {
	Event(key.new(), [Branch(ReceiveLeaf(c.info), (value: Keyed) {
		read(c.values, value)
	})])
}

send(A) = (c: Channel(A), value: A, own ! Scheduler) -> Nil ! Scheduler {
	read(selection_key, ask(own, SendNow(c.info, key.wrap(c.values, value))))
	Nil
}

receive(A) = (c: Channel(A)) -> A ! Scheduler { sync(receive_event(c)) }

share(A, E) = (
	s: Scope(E)
	value: A
	own ! Scheduler
) -> Shared(A) ! Scheduler {
	values: Key(A) = key.new()
	id = ask(own, NewShare(s.info, key.wrap(values, value)))
	Shared(s.info.run, s.info.id, id, values)
}

get(A) = (s: Shared(A), own ! Scheduler) -> A ! Scheduler {
	read(s.values, ask(own, GetShare(s.run, s.owner, s.id)))
}

keep(S, A, E, N) = (
	f: (S) -> A ^ E ! N
	_own ! Scheduler
) -> { (S) -> A ^ E ! Scheduler } ! Scheduler | N - Scheduler - Runner - request.Await {
	! Scheduler = DetachedScheduler
	! Runner = DetachedRunner
	! request.Await = request.Await((_) { panic("task: no scheduler") })
	kept = control.keep_env((input: (S, Scheduler)) {
		(value, own) = input
		own |> {
			DetachedScheduler -> panic("task: no scheduler")
			Scheduler(..) -> {
				! Scheduler = own
				! Runner = own.runner
				! request.Await = own.waiter
				retain(f(value))
			}
		}
	})
	(value: S, own ! Scheduler) { kept((value, own)) }
}

/// A state ref owned by the current scope, or by the run at its root.
/// Values are portable; update callbacks are pure and execute atomically.
ref(A) = (value: A, own ! Scheduler) -> Ref(A) ! Scheduler {
	values: Key(A) = key.new()
	id = ask(own, NewRef(key.wrap(values, value)))
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..)     -> Ref(own.run, id, values)
	}
}

update(A, B) = (
	ref: Ref(A)
	change: (A) -> (A, B)
	own ! Scheduler
) -> B ! Scheduler {
	answers: Key(B) = key.new()
	read(answers, ask(own, UpdateRef(ref.run, ref.id, (value: Keyed) {
		(next, answer) = change(read(ref.values, value))
		(key.wrap(ref.values, next), key.wrap(answers, answer))
	})))
}

both(A, B, E, N) = (
	left: () -> A ^ E ! N
	right: () -> B ^ E ! N
) -> (A, B) ^ E ! Scheduler | N - Scheduler - Runner - request.Await {
	scope((s) {
		a = go(s, left)
		b = go(s, right)
		(join(a), join(b))
	})
}

race(A, E, N) = (
	left: () -> A ^ E ! N
	right: () -> A ^ E ! N
	own ! Scheduler
) -> A ^ E ! Scheduler | N - Scheduler - Runner - request.Await {
	results: Key(Result(A, E)) = key.new()
	scope_call((s) {
		go_key(s, left, results)
		go_key(s, right, results)
		ask(own, Drain)
		Nil
	}, Portable, Racing, own) |> {
		WinnerValue(value) -> fail.from_result(read(results, value))
		BodyValue(_)       -> panic("task: missing race winner")
	}
}

spawn_values(A, B, E, N) = (
	s: Scope(E)
	xs: List(A)
	f: (A) -> B ^ E ! N
) -> List(Task(B, E)) ! Scheduler | N - Scheduler - Runner - request.Await {
	xs |> {
		[] -> []
		[x, ..rest] -> {
			child = go(s, () { f(x) })
			[child, ..spawn_values(s, rest, f)]
		}
	}
}

join_values(A, E) = (tasks: List(Task(A, E))) -> List(A) ^ E ! Scheduler {
	tasks |> {
		[] -> []
		[t, ..rest] -> {
			value = join(t)
			[value, ..join_values(rest)]
		}
	}
}

map_concurrently(A, B, E, N) = (
	xs: List(A)
	f: (A) -> B ^ E ! N
) -> List(B) ^ E ! Scheduler | N - Scheduler - Runner - request.Await {
	scope((s) { join_values(spawn_values(s, xs, f)) })
}

execute_body(A, E, N) = (
	body: () -> A ^ E ! N
	runner: Runner
	seed: rng.Generator
	policy: Policy
) -> Result(Result(A, E), ScheduleError) ! N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	runner |> {
		DetachedRunner -> panic("task: no scheduler")
		Runner(..) -> {
			prefix = runner.allocate()
			run_id: Key(Nil) = key.new()
			results: Key(Result(A, E)) = key.new()
			errors: Key(E) = key.new()
			! request.Root = runner.context
			control_root = request.open_root()
			root = request.open_root()
			owner = {
				! request.Root = root
				retain(request.open_owner())
			}
			control_owner = {
				! request.Root = control_root
				retain(request.open_owner())
			}
			! request.Root = root
			! request.Owner = owner
			start = make_start(body, results, errors, False)
			root_task = TaskState(
				prefix
				0
				request.ready(Nil)
				True
				Absent
				Absent
				[]
				[]
				[]
				[]
				0
				0
				[]
				Absent
				[]
				[]
				Absent
				Dormant(start)
				Pending
				False
				no_waiters
				0
				False
			)
			(choices, scripted) = policy |> {
				Fifo          -> ([], False)
				Script(paths) -> (paths, True)
			}
			routing = Runner(..runner, seed: seed, policy: policy)
			state = State(
				run_id
				routing
				root
				TaskStore(
					0
					Present(root_task)
					-1
					Absent
					Absent
					Absent
					NoTasks
				)
				[ActiveTask(0, root_task.alive)]
				1
				1
				[]
				0
				0
				False
				False
				empty_store()
				[]
				empty_store()
				empty_store()
				Absent
				[]
				[0]
				[]
				[]
				[]
				choices
				scripted
				Absent
				Absent
				Absent
				0
				0
				prefix == []
				False
			)
			ended = retain(resource.ensure(() { loop(state) }, () {
				runner.forget(run_id)
				request.close_owner(control_owner)
				request.close_root(control_root)
				request.close_root(root)
			}))
			ended |> {
				RunOk(value)        -> Ok(read(results, value))
				RunError(path)      -> Error(NotRunnable(path))
				RunCancelled(abort) -> abort()
			}
		}
	}
}

run(A, E, N) = (
	body: () -> A ^ E ! N
	runner ! Runner
) -> A ^ E ! Runner | N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	runner |> {
		DetachedRunner -> panic("task: no scheduler")
		Runner(..) -> execute(body, runner, runner.seed, runner.policy) |> {
			Ok(result) -> fail.from_result(result)
			Error(_)   -> panic("task: invalid inherited schedule")
		}
	}
}

test(A, E, N) = (
	seed: rng.Generator
	choices: List(List(Int))
	body: () -> A ^ E ! N
	runner ! Runner
) -> A ^ E | ScheduleError ! Runner | N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	execute(body, runner, seed, Script(choices)) |> {
		Ok(result)   -> fail.from_result(result)
		Error(error) -> fail.raise(error)
	}
}

_entry(A, E, N) = (
	body: () -> A ^ E ! N
	root ! request.Root
) -> A ^ E ! request.Root | N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	! Runner = initial(root)
	retain(run(body))
}

scope_call(A, E, N) = (
	body: (Scope(E)) -> A ^ E ! N
	witness: Witness
	mode: Mode
	own: Scheduler
) -> ScopeResult(A) ^ E ! N {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..) -> {
			owner = {
				! request.Root = own.root
				retain(request.open_owner())
			}
			kept = control.keep_env((_: Nil) {
				resource.ensure(() {
					! request.Owner = owner
					retain(scope_call_body(body, witness, mode, own))
				}, () { request.close_owner(owner) })
			})
			retain(kept(Nil))
		}
	}
}

execute(A, E, N) = (
	body: () -> A ^ E ! N
	runner: Runner
	seed: rng.Generator
	policy: Policy
) -> Result(Result(A, E), ScheduleError) ! N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	kept = control.keep_env((_: Nil) {
		execute_body(body, runner, seed, policy)
	})
	retain(kept(Nil))
}

sync(A, E, N) = (e: Event(A, E, N), own ! Scheduler) -> A ^ E ! N | Scheduler {
	kept = control.keep_env((_: Nil) { sync_body(e, own) })
	retain(kept(Nil))
}

root_active = (_run: Key(Nil), _active: Active) { ContinueRun }

cause_allowed = (s: State, t: TaskState, cause: Cause) -> Bool {
	scopes_ok = core.not(any(t.chain, (id: Address) {
		scope = find_scope(s.scopes, id)
		scope.cleanup && {
			cause.run != s.run || {
				cause.scope |> {
					Absent -> True
					Present(source) -> core.not(
						contains(find_scope(s.scopes, source).chain, id)
					)
				}
			}
		}
	}))
	guards_ok = core.not(any(t.foreign, (guard: ForeignGuard) {
		cause.run != s.run || {
			cause.scope |> {
				Absent -> True
				Present(source) -> guard.anchor |> {
					Absent -> False
					Present(anchor) -> source == anchor ||
						core.not(
							contains(find_scope(s.scopes, source).chain, anchor)
						)
				}
			}
		}
	}))
	scopes_ok && guards_ok
}

frame_abort = (frames: List(Frame)) -> Maybe(() -> Never) {
	frames |> {
		[] -> Absent
		[f, .._] -> { f.finishing } |> {
			True -> Absent
			False -> f.unwinding |> {
				Absent     -> Present(f.interrupt)
				Present(_) -> Absent
			}
		}
	}
}

effective_active = (s: State, t: TaskState) -> Active {
	relay = t.frames |> {
		[]           -> empty_relay
		[frame, .._] -> frame.relay
	}
	hop = Hop(
		s.run
		key.wrap(task_key, t.serial)
		frame_owner(t.frames)
		frame_epoch(t.frames)
		list_map(t.foreign, (guard: ForeignGuard) { guard.token })
	)
	own = { t.cleanup || foreign_active(t) } |> {
		True  -> Absent
		False -> frame_abort(t.frames)
	}
	t.nested |> {
		[] -> Active([hop], own, relay)
		[row, .._] -> {
			abort = own |> {
				Absent     -> Absent
				Present(_) -> nested_abort(t.nested)
			}
			Active([hop, ..row.active.hops], abort, relay)
		}
	}
}

put_nest = (
	rows: List(Nest)
	id: Key(Nil)
	anchor: Maybe(Address)
	active: Active
) -> List(Nest) {
	old = filter(rows, (row: Nest) { row.id == id })
	notified = old |> {
		[]         -> []
		[row, .._] -> row.notified
	}
	parent = old |> {
		[]         -> anchor
		[row, .._] -> row.anchor
	}
	[Nest(id, parent, active, notified), ..filter(rows, (row: Nest) {
		row.id != id
	})]
}

register_driver = (s: State, task_path: Int, command: Command) -> State {
	t = find_task(s.tasks, task_path)
	(nested, driver) = command |> {
		NestedActive(id, active) -> (
			put_nest(t.nested, id, frame_owner(t.frames), active)
			Present(id)
		)
		ForwardWait(id, _) | NestedWait(id, _) -> {
			rows = { any(t.nested, (row: Nest) { row.id == id }) } |> {
				True -> t.nested
				False -> put_nest(
					t.nested
					id
					frame_owner(t.frames)
					empty_active
				)
			}
			(rows, Present(id))
		}
		ForgetWait(id) -> (list_map(t.nested, (row: Nest) {
			{ row.id == id } |> {
				True  -> Nest(..row, active: Active(..row.active, abort: Absent))
				False -> row
			}
		}), Absent)
		_ -> (t.nested, Absent)
	}
	set_task(s, TaskState(..t, nested: nested, driver: driver))
}

pending_frame = (frames: List(Frame)) -> Maybe(Frame) {
	frames |> {
		[] -> Absent
		[f, .._] -> { f.deferred } |> {
			True  -> Present(f)
			False -> Absent
		}
	}
}

older_shield = (rows: List(Nest), id: Key(Nil)) -> Bool {
	rows |> {
		[] -> False
		[row, ..rest] -> { row.id == id } |> {
			False -> older_shield(rest, id)
			True -> any(rest, (older: Nest) {
				older.active.abort |> {
					Absent     -> True
					Present(_) -> False
				}
			})
		}
	}
}

cancel_response = (s: State, t: TaskState, abort: () -> Never) -> State {
	status = t.status |> {
		Runnable(k, _)
			| WaitingTicket(k, _)
			| WaitingScope(k, _)
			| WaitingEvent(k, _)
			| WaitingDrain(k) -> Runnable(k, CancelStep(abort))
		_ -> panic("task: invalid cancellation response")
	}
	released = remove_registration(s, t.serial)
	latest = find_task(released.tasks, t.serial)
	next = set_task(
		released
		TaskState(..t, joiners: latest.joiners, status: status)
	)

	queue(State(..next, waiting: without(next.waiting, t.serial)), t.serial)
}

notify_nested = (s: State, t: TaskState, id: Key(Nil)) -> State {
	pending_frame(t.frames) |> {
		Absent -> s
		Present(frame) -> frame.cause |> {
			Absent -> panic("task: invalid deferred cancellation")
			Present(cause) -> {
				rows = affected_nests(t, t.frames)
				relevant = any(rows, (row: Nest) { row.id == id })
				notified = any(rows, (row: Nest) {
					row.id == id && contains(row.notified, cause)
				})
				{
					core.not(relevant) || notified || older_shield(rows, id)
				} |> {
					True -> s
					False -> {
						nested = list_map(t.nested, (row: Nest) {
							{ row.id == id } |> {
								True -> Nest(
									..row
									notified: [cause, ..row.notified]
								)
								False -> row
							}
						})
						cancel_response(
							s
							TaskState(..t, nested: nested)
							frame.abort
						)
					}
				}
			}
		}
	}
}

deliver_deferred = (s: State, path: Int, command: Command) -> State {
	t = find_task(s.tasks, path)
	{ cancellation_reply(t) } |> {
		True -> s
		False -> t.driver |> {
			Present(id) -> notify_nested(s, t, id)
			Absent -> pending_frame(t.frames) |> {
				Absent -> s
				Present(frame) -> affected_nests(t, t.frames) |> {
					[] -> {
						frames = t.frames |> {
							[]          -> []
							[f, ..rest] -> [Frame(..f, deferred: False), ..rest]
						}
						frame.foreign |> {
							[] -> cancel_response(
								s
								TaskState(..t, frames: frames)
								frame.abort
							)
							hops -> frame.cause |> {
								Absent -> panic("task: missing deferred cause")
								Present(cause) -> forward_response(
									s
									TaskState(..t, frames: frames)
									BeginForeign(
										hops
										scope_token(s, frame.scope)
										frame.abort
										cause
									)
								)
							}
						}
					}
					[first, .._] -> command |> {
						ForgetWait(_) -> {
							any(affected_nests(t, t.frames), (row: Nest) {
								row.active.abort |> {
									Absent     -> True
									Present(_) -> False
								}
							})
						} |> {
							True -> s
							False -> first.active.abort |> {
								Absent         -> s
								Present(abort) -> cancel_response(s, t, abort)
							}
						}
						_ -> s
					}
				}
			}
		}
	}
}

frame_owner = (frames: List(Frame)) -> Maybe(Address) {
	frames |> {
		[]       -> Absent
		[f, .._] -> f.scope
	}
}

anchor_within = (
	frames: List(Frame)
	anchor: Maybe(Address)
	target: Maybe(Address)
) -> Bool {
	frames |> {
		[] -> False
		[f, ..rest] -> { f.scope == anchor } |> {
			True -> True
			False -> { f.scope == target } |> {
				True  -> False
				False -> anchor_within(rest, anchor, target)
			}
		}
	}
}

affected_nests = (t: TaskState, frames: List(Frame)) -> List(Nest) {
	frames |> {
		[] -> []
		[f, .._] -> filter(t.nested, (row: Nest) {
			anchor_within(t.frames, row.anchor, f.scope)
		})
	}
}

nested_abort = (rows: List(Nest)) -> Maybe(() -> Never) {
	rows |> {
		[] -> Absent
		[first, .._] -> {
			any(rows, (row: Nest) {
				row.active.abort |> {
					Absent     -> True
					Present(_) -> False
				}
			})
		} |> {
			True  -> Absent
			False -> first.active.abort
		}
	}
}

Hop {
	Hop(
		run:     Key(Nil)
		task:    Keyed
		anchor:  Maybe(Address)
		epoch:   Maybe(Cause)
		foreign: List(Key(Nil))
	)
}

Active {
	Active(
		hops:  List(Hop)
		abort: Maybe(() -> Never)
		relay: (ForeignNotice, Keyed, Bool) -> Keyed
	)
}

ForeignGuard {
	ForeignGuard(token: Key(Nil), anchor: Maybe(Address))
}

ForeignNotice {
	BeginForeign(List(Hop), Key(Nil), () -> Never, Cause)
	EndForeign(List(Hop), Key(Nil))
}

empty_relay = (_notice: ForeignNotice, _value: Keyed, _discard: Bool) -> Keyed {
	panic("task: missing foreign relay")
}

empty_active = Active([], Absent, empty_relay)

foreign_active = (t: TaskState) {
	t.foreign |> {
		[] -> False
		_  -> True
	}
}

frame_epoch = (frames: List(Frame)) -> Maybe(Cause) {
	frames |> {
		[]       -> Absent
		[f, .._] -> f.cause
	}
}

scope_token = (s: State, owner: Maybe(Address)) -> Key(Nil) {
	owner |> {
		Absent      -> panic("task: foreign root delimiter")
		Present(id) -> find_scope(s.scopes, id).token
	}
}

notice_with = (notice: ForeignNotice, hops: List(Hop)) -> ForeignNotice {
	notice |> {
		BeginForeign(_, token, abort, cause) -> BeginForeign(
			hops
			token
			abort
			cause
		)
		EndForeign(_, token) -> EndForeign(hops, token)
	}
}

current_relay = (t: TaskState) -> (ForeignNotice, Keyed, Bool) -> Keyed {
	t.driver |> {
		Present(id) -> {
			rows = filter(t.nested, (row: Nest) { row.id == id })
			rows |> {
				[]         -> empty_relay
				[row, .._] -> row.active.relay
			}
		}
		Absent -> t.nested |> {
			[]         -> empty_relay
			[row, .._] -> row.active.relay
		}
	}
}

forward_response = (s: State, t: TaskState, notice: ForeignNotice) -> State {
	relay = current_relay(t)
	status = t.status |> {
		Runnable(k, Proceed(value)) -> Runnable(
			k
			ForwardStep(notice, value, False, relay)
		)
		WaitingTicket(k, _)
			| WaitingScope(k, _)
			| WaitingEvent(k, _)
			| WaitingDrain(k) -> {
			discard = t.driver |> {
				Absent     -> True
				Present(_) -> False
			}
			Runnable(k, ForwardStep(notice, unit(), discard, relay))
		}
		_ -> t.status
	}
	released = remove_registration(s, t.serial)
	latest = find_task(released.tasks, t.serial)
	next = set_task(
		released
		TaskState(..t, joiners: latest.joiners, status: status)
	)

	queue(State(..next, waiting: without(next.waiting, t.serial)), t.serial)
}

scope_end_reply = (s: State, t: TaskState, scope: ScopeState) -> StepAnswer {
	value = key.wrap(scope_reply_key, scope_reply(scope))
	scope.foreign |> {
		[] -> Proceed(value)
		hops -> ForwardStep(
			EndForeign(hops, scope.token)
			value
			False
			current_relay(t)
		)
	}
}

queue_scope_end = (s: State, t: TaskState, scope: ScopeState) -> State {
	scope.foreign |> {
		[] -> s
		hops -> set_task(
			s
			TaskState(
				..t
				messages: append(t.messages, [EndForeign(hops, scope.token)])
			)
		)
	}
}

finish_scope = (
	s: State
	t: TaskState
	k: Continuation(StepAnswer, Turn, Never, Never)
	info: ScopeInfo
) -> State {
	id = read(address_key, info.id)
	alive = store_find(s.scopes, address_index(id)) != Absent
	{ alive } |> {
		False -> reply(s, t.serial, k, Proceed(unit()), False)
		True -> {
			scope = find_scope(s.scopes, id)
			(frames, chain) = scope.mark |> {
				BodyOpen -> (end_frame(t.frames, id), without(t.chain, id))
				_        -> (t.frames, t.chain)
			}
			body = TaskState(
				..t
				frames: frames
				chain:  chain
				status: WaitingScope(k, id)
			)
			closed = ScopeState(..scope, mark: BodyCancelled)
			cancel_scope(
				set_scope(set_task(State(..s, current: Absent), body), closed)
				id
			)
		}
	}
}

anchor_epoch = (
	frames: List(Frame)
	anchor: Maybe(Address)
) -> Maybe(Maybe(Cause)) {
	frames |> {
		[] -> Absent
		[frame, ..rest] -> { frame.scope == anchor } |> {
			True  -> Present(frame.cause)
			False -> anchor_epoch(rest, anchor)
		}
	}
}

unwinding_above = (frames: List(Frame), anchor: Maybe(Address)) -> Bool {
	frames |> {
		[] -> False
		[frame, ..rest] -> { frame.scope == anchor } |> {
			True -> False
			False -> { frame.finishing } |> {
				True -> True
				False -> frame.unwinding |> {
					Present(_) -> True
					Absent     -> unwinding_above(rest, anchor)
				}
			}
		}
	}
}

notice_ready = (t: TaskState, hop: Hop) -> Bool {
	core.not(unwinding_above(t.frames, hop.anchor))
}

notice_live = (t: TaskState, hop: Hop) -> Bool {
	epoch_ok = anchor_epoch(t.frames, hop.anchor) |> {
		Absent         -> False
		Present(epoch) -> epoch == hop.epoch
	}
	tokens_ok = core.not(any(t.foreign, (guard: ForeignGuard) {
		core.not(contains(hop.foreign, guard.token))
	}))
	epoch_ok && tokens_ok
}

post_notice = (s: State, notice: ForeignNotice) -> State {
	notice_hops(notice) |> {
		[] -> s
		[hop, .._] -> {
			check_run(s.run, hop.run)
			path = read(task_key, hop.task)
			alive = store_find(s.tasks, path) != Absent
			{ alive } |> {
				False -> s
				True -> {
					t = find_task(s.tasks, path)
					set_task(
						s
						TaskState(..t, messages: append(t.messages, [notice]))
					)
				}
			}
		}
	}
}

apply_notice = (s: State, t: TaskState, notice: ForeignNotice) -> State {
	notice_hops(notice) |> {
		[] -> s
		[hop, ..rest] -> rest |> {
			[] -> notice |> {
				BeginForeign(_, token, abort, _) -> { notice_live(t, hop) } |> {
					False -> s
					True -> {
						guards = [ForeignGuard(token, hop.anchor), ..t.foreign]
						cancel_response(
							s
							TaskState(..t, foreign: guards)
							abort
						)
					}
				}
				EndForeign(_, token) -> set_task(
					s
					TaskState(
						..t
						foreign: filter(t.foreign, (guard: ForeignGuard) {
							guard.token != token
						})
					)
				)
			}
			_ -> forward_response(s, t, notice_with(notice, rest))
		}
	}
}

pump_task_messages = (s: State, path: Int) -> State {
	t = find_task(s.tasks, path)
	t.messages |> {
		[] -> s
		[notice, ..rest] -> {
			responsive = t.status |> {
				Runnable(_, Proceed(_)) -> True
				WaitingTicket(_, _)
					| WaitingScope(_, _)
					| WaitingEvent(_, _)
					| WaitingDrain(_) -> t.driver |> {
					Present(_)        -> True
					Absent -> notice |> {
						BeginForeign(..) -> True
						EndForeign(..)   -> False
					}
				}
				_ -> False
			}
			ready = notice |> {
				EndForeign(..) -> True
				BeginForeign(hops, _, _, _) -> hops |> {
					[hop] -> notice_ready(t, hop)
					_     -> True
				}
			}
			{ responsive && ready } |> {
				False -> s
				True -> {
					updated = TaskState(..t, messages: rest)
					apply_notice(set_task(s, updated), updated, notice)
				}
			}
		}
	}
}

pump_rows = (s: State, rows: List(ActiveTask)) -> State {
	rows |> {
		[] -> s
		[entry, ..rest] -> store_find(s.tasks, entry.id) |> {
			Present(t) ? core.not(t.clean) -> t.messages |> {
				[] -> pump_rows(s, rest)
				_  -> pump_rows(pump_task_messages(s, entry.id), rest)
			}
			_ -> pump_rows(s, rest)
		}
	}
}

pump_messages = (s: State) {
	{ s.message_history } |> {
		False -> s
		True  -> pump_rows(s, reverse(s.active, []))
	}
}

relay_notice = (
	s: State
	t: TaskState
	k: Continuation(StepAnswer, Turn, Never, Never)
	notice: ForeignNotice
	value: Keyed
	discard: Bool
) -> State {
	base = reply(s, t.serial, k, Proceed(value), False)
	next = pump_messages(post_notice(base, notice))
	current = find_task(next.tasks, t.serial)
	current.status |> {
		Runnable(continuation, ForwardStep(rest, answer, prior_discard, relay)) -> set_task(
			next
			TaskState(
				..current
				status: Runnable(
					continuation
					ForwardStep(rest, answer, discard || prior_discard, relay)
				)
			)
		)
		Runnable(_, CancelStep(_)) -> next
		_ -> { discard } |> {
			True -> notice |> {
				BeginForeign(_, _, _, cause) -> set_task(
					next
					TaskState(..current, cancelled_by: Present(cause))
				)
				EndForeign(..) -> reply(next, t.serial, k, Proceed(value), True)
			}
			False -> reply(next, t.serial, k, Proceed(value), True)
		}
	}
}

notice_hops = (notice: ForeignNotice) -> List(Hop) {
	notice |> {
		BeginForeign(hops, _, _, _) -> hops
		EndForeign(hops, _)         -> hops
	}
}

// Selected by package data with this module's private authority.
_provider_runner = (
	seeds: List(Runner)
	root ! request.Root
) -> Runner ! request.Root {
	seeds |> {
		[runner] -> runner
		[]       -> initial(root)
		_        -> panic("provider: invalid runner seed")
	}
}

_provider_scheduler = (seeds: List(Scheduler)) -> Scheduler {
	seeds |> {
		[scheduler] -> scheduler
		_           -> panic("provider: scheduler seed required")
	}
}
