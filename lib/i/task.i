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
	both
	race
	map_concurrently
)

Address {
	Address(path: List(Int), index: Int)
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
	Switched(List(Int), Command, Continuation(StepAnswer, Turn, Never, Never))
	Finished(List(Int), Completion)
}

Start {
	Start(body: (Scheduler) -> Completion)
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
		path:    Keyed
		seed:    rng.Generator
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

Frame {
	Frame(
		scope:     Maybe(Address)
		abort:     () -> Never
		interrupt: () -> Never
		relay:     (ForeignNotice, Keyed, Bool) -> Keyed
		foreign:   List(Hop)
		immune:    List(Cause)
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
	WaitingEvent(Continuation(StepAnswer, Turn, Never, Never), Int)
	WaitingDrain(Continuation(StepAnswer, Turn, Never, Never))
	Running
	Ended
}

TaskState {
	TaskState(
		path:            List(Int)
		parent:          Maybe(Address)
		origin:          Maybe(ScopeInfo)
		base:            List(Address)
		portable_record: Bool
		chain:           List(Address)
		frames:          List(Frame)
		immune:          List(Cause)
		spawn:           Int
		handles:         Int
		nested:          List(Nest)
		driver:          Maybe(Key(Nil))
		foreign:         List(ForeignGuard)
		messages:        List(ForeignNotice)
		cancelled_by:    Maybe(Cause)
		status:          Status
		outcome:         Outcome
		clean:           Bool
	)
}

ScopeState {
	ScopeState(
		info:         ScopeInfo
		address:      Address
		body:         List(Int)
		chain:        List(Address)
		immune:       List(Cause)
		cleanup:      Bool
		foreign:      List(Hop)
		token:        Key(Nil)
		children:     List(List(Int))
		mark:         BodyMark
		errors:       List((List(Int), Keyed))
		cancellation: Maybe(Cause)
		mode:         Mode
		winner:       Maybe(Keyed)
	)
}

ChannelState {
	ChannelState(info: ChannelInfo, address: Address, owner: Maybe(Address))
}

ShareState {
	ShareState(id: Address, owner: Address, value: Keyed)
}

Leaf {
	SendLeaf(ChannelInfo, Keyed)
	ReceiveLeaf(ChannelInfo)
	JoinLeaf(TaskInfo)
	AlwaysLeaf(Keyed)
}

Branch(A, E, N) {
	Branch(leaf: Leaf, decode: (Keyed) -> A ? E ! N)
}

Event(A, E, N) {
	Event(identity: Key(Nil), branches: List(Branch(A, E, N)))
}

Selection {
	Selected(Int, Keyed)
	SelectedCancelled
}

Registration {
	Registration(id: Int, task: List(Int), leaves: List(Leaf))
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

State {
	State(
		run:           Key(Nil)
		runner:        Runner
		root:          request.Root
		tasks:         List(TaskState)
		scopes:        List(ScopeState)
		channels:      List(ChannelState)
		shares:        List(ShareState)
		current:       Maybe(List(Int))
		waiting:       List(List(Int))
		ready:         List(List(Int))
		registrations: List(Registration)
		forwards:      List(Forward)
		choices:       List(List(Int))
		scripted:      Bool
		failure:       Maybe(List(Int))
		cancellation:  Maybe(Cause)
		outer_cancel:  Maybe(() -> Never)
		epoch:         Int
		registration:  Int
	)
}

Command {
	Register(
		Maybe(Address)
		() -> Never
		() -> Never
		(ForeignNotice, Keyed, Bool) -> Keyed
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
	Commit(state: State, selection: Selection)
}

Partner {
	Partner(registration: Registration, index: Int, value: Keyed)
}

address_key: Key(Address) = key.new()

path_key: Key(List(Int)) = key.new()

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

unit = () { key.wrap(unit_key, Nil) }

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

need = (own: Scheduler) -> Scheduler {
	own |> {
		DetachedScheduler -> panic("task: no scheduler")
		Scheduler(..)     -> own
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

find_task = (tasks: List(TaskState), path: List(Int)) -> TaskState {
	tasks |> {
		[] -> panic("task: task is gone")
		[t, ..rest] -> { t.path == path } |> {
			True  -> t
			False -> find_task(rest, path)
		}
	}
}

put_task = (tasks: List(TaskState), task: TaskState) -> List(TaskState) {
	tasks |> {
		[] -> [task]
		[t, ..rest] -> { t.path == task.path } |> {
			True  -> [task, ..rest]
			False -> [t, ..put_task(rest, task)]
		}
	}
}

find_scope = (scopes: List(ScopeState), id: Address) -> ScopeState {
	scopes |> {
		[] -> panic("task: scope is gone")
		[s, ..rest] -> { s.address == id } |> {
			True  -> s
			False -> find_scope(rest, id)
		}
	}
}

put_scope = (scopes: List(ScopeState), scope: ScopeState) -> List(ScopeState) {
	scopes |> {
		[] -> [scope]
		[s, ..rest] -> { s.address == scope.address } |> {
			True  -> [scope, ..rest]
			False -> [s, ..put_scope(rest, scope)]
		}
	}
}

set_task = (s: State, t: TaskState) { State(..s, tasks: put_task(s.tasks, t)) }

set_scope = (s: State, scope: ScopeState) {
	State(..s, scopes: put_scope(s.scopes, scope))
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
		}))
		body()
	})
}

make_start(A, E, N) = (
	body: () -> A ? E ! N
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
				})
				retain(guarded(own, Absent, () { local(Nil) }))
			}
		}
	})
	Start(kept)
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
		Scheduler(..)     -> own.seed
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
	s: State
) -> Scheduler {
	upstream = s.runner
	upstream |> {
		DetachedRunner -> panic("task: no scheduler")
		Runner(..) -> {
			route = (command: Command) {
				control.suspend(b, (k) { Switched(task_path, command, k) })
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
			generator = fail.catch(() {
				rng.at_path(upstream.seed, task_path)
			}) |> {
				Ok(g)    -> g
				Error(_) -> panic("task: invalid task path")
			}
			Scheduler(
				s.run
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

first_path = (a: List(Int), b: List(Int)) -> Bool {
	(a, b) |> {
		([], _) -> True
		(_, []) -> False
		([x, ..xs], [y, ..ys]) -> { x == y } |> {
			True  -> first_path(xs, ys)
			False -> x < y
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

exemptions = (t: TaskState) { append(t.immune, active_epochs(t.frames)) }

frame_plan = (frames: List(Frame), cause: Cause) -> Maybe(CancelPlan) {
	frames |> {
		[] -> Absent
		[f, ..rest] -> f.unwinding |> {
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

remove_registration = (s: State, task_path: List(Int)) {
	State(..s, registrations: filter(s.registrations, (r: Registration) {
		r.task != task_path
	}))
}

queue = (s: State, task_path: List(Int)) {
	{ contains(s.ready, task_path) } |> {
		True  -> s
		False -> State(..s, ready: append(s.ready, [task_path]))
	}
}

reply = (
	s: State
	task_path: List(Int)
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
					)
				)
				State(..next, ready: without(next.ready, t.path))
			}
			Ended | Running -> s
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

propagate_tasks = (s: State, rows: List(TaskState)) -> State {
	rows |> {
		[] -> s
		[t, ..rest] -> {
			next = cause_for_chain(s, t, t.chain) |> {
				Absent     -> s
				Present(c) -> accept_cancel(s, t, c)
			}
			propagate_tasks(next, rest)
		}
	}
}

propagate = (s: State) { propagate_tasks(s, s.tasks) }

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

find_channel = (channels: List(ChannelState), id: Address) -> ChannelState {
	channels |> {
		[] -> panic("task: channel is gone")
		[c, ..rest] -> { c.address == id } |> {
			True  -> c
			False -> find_channel(rest, id)
		}
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

match_leaves = (
	wanted: Leaf
	leaves: List(Leaf)
	index: Int
) -> Maybe((Int, Keyed)) {
	leaves |> {
		[] -> Absent
		[leaf, ..rest] -> (wanted, leaf) |> {
			(SendLeaf(a, _), ReceiveLeaf(b)) ? a.id == b.id -> Present((
				index
				unit()
			))
			(ReceiveLeaf(a), SendLeaf(b, v)) ? a.id == b.id -> Present((
				index
				v
			))
			_ -> match_leaves(wanted, rest, index + 1)
		}
	}
}

partner = (registrations: List(Registration), leaf: Leaf) -> Maybe(Partner) {
	registrations |> {
		[] -> Absent
		[r, ..rest] -> match_leaves(leaf, r.leaves, 0) |> {
			Absent -> partner(rest, leaf)
			Present(pair) -> {
				(i, value) = pair
				Present(Partner(r, i, value))
			}
		}
	}
}

commit_partner = (s: State, p: Partner, payload: Keyed) -> State {
	r = p.registration
	t = find_task(s.tasks, r.task)
	t.status |> {
		WaitingEvent(k, _) -> {
			value = key.wrap(selection_key, Selected(p.index, payload))
			next = remove_registration(s, r.task)
			queue(
				set_task(
					next
					TaskState(..t, status: Runnable(k, Proceed(value)))
				)
				t.path
			)
		}
		_ -> panic("task: invalid event registration")
	}
}

probe_leaf = (s: State, leaf: Leaf, index: Int) -> Maybe(Commit) {
	leaf |> {
		AlwaysLeaf(value) -> Present(Commit(s, Selected(index, value)))
		JoinLeaf(task) -> {
			target = find_task(s.tasks, read(path_key, task.id))
			target.outcome |> {
				Pending -> Absent
				Success(value) | Failure(value) -> Present(
					Commit(s, Selected(index, value))
				)
				Cancelled(_) -> Present(Commit(s, SelectedCancelled))
			}
		}
		SendLeaf(_, value) -> partner(s.registrations, leaf) |> {
			Absent -> Absent
			Present(p) -> Present(
				Commit(commit_partner(s, p, value), Selected(index, unit()))
			)
		}
		ReceiveLeaf(_) -> partner(s.registrations, leaf) |> {
			Absent -> Absent
			Present(p) -> Present(
				Commit(commit_partner(s, p, unit()), Selected(index, p.value))
			)
		}
	}
}

probe = (s: State, leaves: List(Leaf), index: Int) -> Maybe(Commit) {
	leaves |> {
		[] -> Absent
		[leaf, ..rest] -> probe_leaf(s, leaf, index) |> {
			Absent -> probe(s, rest, index + 1)
			result -> result
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
	probe(s, leaves, 0) |> {
		Present(committed) -> reply(
			committed.state
			t.path
			k
			Proceed(key.wrap(selection_key, committed.selection))
			True
		)
		Absent -> {
			id = s.registration + 1
			row = Registration(id, t.path, leaves)
			next = set_task(s, TaskState(..t, status: WaitingEvent(k, id)))
			State(
				..next
				current:       Absent
				registration:  id
				registrations: append(next.registrations, [row])
			)
		}
	}
}

registration_alive = (s: State, id: Int) {
	any(s.registrations, (r: Registration) { r.id == id })
}

recheck_events = (s: State, rows: List(Registration)) -> State {
	rows |> {
		[] -> s
		[r, ..rest] -> { registration_alive(s, r.id) } |> {
			False -> recheck_events(s, rest)
			True -> {
				without_self = remove_registration(s, r.task)
				next = probe(without_self, r.leaves, 0) |> {
					Absent -> s
					Present(committed) -> {
						t = find_task(committed.state.tasks, r.task)
						t.status |> {
							WaitingEvent(k, _) -> queue(
								set_task(
									committed.state
									TaskState(
										..t
										status: Runnable(
											k
											Proceed(
												key.wrap(
													selection_key
													committed.selection
												)
											)
										)
									)
								)
								t.path
							)
							_ -> panic("task: invalid event registration")
						}
					}
				}
				recheck_events(next, rest)
			}
		}
	}
}

wake_tickets = (s: State, paths: List(List(Int))) -> State {
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

scopes_of = (s: State, task_path: List(Int)) {
	any(s.scopes, (scope: ScopeState) { scope.body == task_path })
}

refresh_clean = (s: State) {
	State(..s, tasks: list_map(s.tasks, (t: TaskState) {
		t.status |> {
			Ended ? core.not(t.clean) -> TaskState(
				..t
				clean: core.not(scopes_of(s, t.path))
			)
			_ -> t
		}
	}))
}

channel_owned = (c: ChannelState, id: Address) {
	c.owner |> {
		Absent         -> False
		Present(owner) -> owner == id
	}
}

retire_scope = (s: State, scope: ScopeState) -> State {
	removed = filter(s.channels, (c: ChannelState) {
		channel_owned(c, scope.address)
	})
	bad = any(s.registrations, (r: Registration) {
		any(r.leaves, (leaf: Leaf) {
			leaf |> {
				SendLeaf(info, _) | ReceiveLeaf(info) -> any(
					removed
					(c: ChannelState) { c.info.id == info.id }
				)
				_ -> False
			}
		})
	})
	{ bad } |> {
		True  -> panic("task: channel is gone")
		False -> Nil
	}
	next = State(
		..s
		scopes: filter(s.scopes, (row: ScopeState) {
			row.address != scope.address
		})
		channels: filter(s.channels, (c: ChannelState) {
			core.not(channel_owned(c, scope.address))
		})
		shares: filter(s.shares, (row: ShareState) {
			row.owner != scope.address
		})
		tasks: filter(s.tasks, (t: TaskState) {
			t.parent != Present(scope.address)
		})
	)
	body = find_task(next.tasks, scope.body)
	body.status |> {
		WaitingScope(k, id) ? id == scope.address -> queue(
			set_task(
				next
				TaskState(
					..body
					status: Runnable(k, scope_end_reply(next, body, scope))
				)
			)
			body.path
		)
		_ -> queue_scope_end(next, body, scope)
	}
}

retire_scopes = (s: State, rows: List(ScopeState)) -> State {
	rows |> {
		[] -> s
		[scope, ..rest] -> {
			closed = scope.mark |> {
				BodyOpen -> False
				_        -> True
			}
			can_retire = { closed } |> {
				False -> False
				True -> core.not(any(s.tasks, (t: TaskState) {
					t.parent == Present(scope.address) && core.not(t.clean)
				}))
			}
			{ can_retire } |> {
				False -> retire_scopes(s, rest)
				True -> {
					next = refresh_clean(retire_scope(s, scope))
					retire_scopes(next, next.scopes)
				}
			}
		}
	}
}

maintain = (s: State) -> State {
	cancelled = propagate(pump_messages(propagate(s)))
	tickets = wake_tickets(cancelled, cancelled.waiting)
	events = recheck_events(tickets, tickets.registrations)
	retire_scopes(refresh_clean(events), events.scopes)
}

new_address = (t: TaskState) { Address(t.path, t.handles + 1) }

head_scope = (chain: List(Address)) {
	chain |> {
		[]        -> Absent
		[id, .._] -> Present(id)
	}
}

find_share = (shares: List(ShareState), id: Address) -> Keyed {
	shares |> {
		[] -> panic("task: shared value is gone")
		[row, ..rest] -> { row.id == id } |> {
			True  -> row.value
			False -> find_share(rest, id)
		}
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
		GetShare(run, _, id) -> {
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
		Register(owner, abort, interrupt, relay) -> {
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
				exemptions(t)
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
				t.path
				k
				Proceed(key.wrap(path_key, child_path))
				False
			)
		}
		Spawn(info, start) -> {
			scope = find_scope(s.scopes, read(address_key, info.id))
			index = t.spawn + 1
			child_path = append(t.path, [index])
			immune = exemptions(t)
			(status, outcome, clean) = scope.cancellation |> {
				Absent         -> (Dormant(start), Pending, False)
				Present(cause) -> (Ended, Cancelled(cause), True)
			}
			child = TaskState(
				child_path
				Present(scope.address)
				Present(info)
				scope.chain
				core.portable((start, info, scope.chain))
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
			)
			opener = TaskState(..t, spawn: index)
			next = set_task(set_task(s, opener), child)
			recorded = set_scope(
				next
				ScopeState(
					..scope
					children: append(scope.children, [child_path])
				)
			)
			queued = { clean } |> {
				True  -> recorded
				False -> queue(recorded, child_path)
			}
			reply(
				queued
				t.path
				k
				Proceed(key.wrap(path_key, child_path))
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
				t.path
				chain
				exemptions(t)
				cleanup_chain(s, t.chain) ||
					foreign_active(t) ||
					any(t.frames, (f: Frame) {
						f.unwinding |> {
							Present(_) -> True
							Absent     -> False
						}
					})
				[]
				key.new()
				[]
				BodyOpen
				[]
				Absent
				mode
				Absent
			)
			next = set_task(
				s
				TaskState(..t, chain: chain, handles: t.handles + 1)
			)
			reply(
				State(..next, scopes: [row, ..next.scopes])
				t.path
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
				frames: end_frame(t.frames, id)
				chain:  without(t.chain, id)
				status: WaitingScope(k, id)
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
		Yield -> queue(State(..s, current: Absent), t.path)
		Wait(tickets) | NestedWait(_, tickets) -> {
			{ any(tickets, request.settled) } |> {
				True -> queue(State(..s, current: Absent), t.path)
				False -> {
					next = set_task(
						s
						TaskState(..t, status: WaitingTicket(k, tickets))
					)
					State(
						..next
						current: Absent
						waiting: append(without(next.waiting, t.path), [t.path])
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
			queue(next, t.path)
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
			row = ChannelState(info, id, head_scope(t.chain))
			next = set_task(s, TaskState(..t, handles: t.handles + 1))
			reply(
				State(..next, channels: [row, ..next.channels])
				t.path
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
			row = ShareState(id, read(address_key, info.id), value)
			next = set_task(s, TaskState(..t, handles: t.handles + 1))
			reply(
				State(..next, shares: [row, ..next.shares])
				t.path
				k
				Proceed(key.wrap(address_key, id))
				False
			)
		}
		GetShare(_, _, id) -> reply(
			s
			t.path
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

on_command = (
	s: State
	task_path: List(Int)
	command: Command
	k: Continuation(StepAnswer, Turn, Never, Never)
) {
	t = find_task(s.tasks, task_path)
	prevalidate(s, t, command)
	prepared = register_driver(
		reply(s, task_path, k, Proceed(unit()), False)
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
	t.status |> {
		Dormant(start) -> control.boundary((b) {
			own = make_scheduler(b, t.path, s)
			Finished(t.path, start.body(own))
		})
		Runnable(k, value) -> control.resume(k, value)
		_                  -> panic("task: task is not runnable")
	}
}

interests_tasks = (tasks: List(TaskState)) -> List(Ticket) {
	tasks |> {
		[] -> []
		[t, ..rest] -> t.status |> {
			WaitingTicket(_, tickets) -> append(tickets, interests_tasks(rest))
			_                         -> interests_tasks(rest)
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
	all = append(interests_tasks(s.tasks), interests_forward(s.forwards))
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

finished_run = (s: State) {
	core.not(any(s.tasks, (t: TaskState) { core.not(t.clean) }))
}

run_reply = (s: State) -> RunReply {
	s.outer_cancel |> {
		Present(abort) -> RunCancelled(abort)
		Absent -> s.failure |> {
			Present(p) -> RunError(p)
			Absent -> s.tasks |> {
				[root] -> root.outcome |> {
					Success(v) | Failure(v) -> RunOk(v)
					_                       -> panic("task: missing root result")
				}
				_ -> panic("task: invalid root task")
			}
		}
	}
}

choose_turn = (s: State) -> State {
	s.current |> {
		Present(_) -> s
		Absent -> s.choices |> {
			[] -> s.ready |> {
				[]          -> s
				[p, ..rest] -> State(..s, current: Present(p), ready: rest)
			}
			[wanted, ..rest] -> { contains(s.ready, wanted) } |> {
				True -> State(
					..s
					current: Present(wanted)
					ready:   without(s.ready, wanted)
					choices: rest
				)
				False -> cancel_run(
					State(..s, failure: Present(wanted), choices: [])
				)
			}
		}
	}
}

loop = (state: State) -> RunReply {
	s = maintain(state)
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
						Present(_) -> s
						Absent -> apply_run_control(
							s
							runner.poll(s.run, work, core.not(s.ready == []))
						)
					}

					prepared = choose_turn(
						maintain(choose_turn(maintain(polled)))
					)
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
											ready:   without(prepared.ready, p)
										)
										TaskState(..t, status: Running)
									)
									turn(running, t) |> {
										Switched(path, command, k) -> loop(
											on_command(
												running
												path
												command
												k
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
									possible = request.count_root(
										prepared.root
									) >
										0 ||
										any(tickets, has_work)
									{ possible } |> {
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

scope_call_body(A, E, N) = (
	body: (Scope(E)) -> A ? E ! N
	witness: Witness
	mode: Mode
	own: Scheduler
) -> ScopeResult(A) ? E ! N {
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
	body: (s: Scope(E)) -> A ? E ! N
	own ! Scheduler
) -> A ? E ! N | Scheduler {
	scope_call(body, Portable, Ordinary, own) |> {
		BodyValue(value) -> value
		WinnerValue(_)   -> panic("task: invalid scope winner")
	}
}

local_scope(A, E, N) = (
	body: (s: Scope(E)) -> A ? E ! N
	own ! Scheduler
) -> A ? E ! N | Scheduler {
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
	body: () -> A ? E ! N
	result: Key(Result(A, E))
	own ! Scheduler
) -> Task(A, E) ! Scheduler | N - Scheduler - Runner - request.Await {
	start = make_start(body, result, s.errors, portable_witness(s.info.witness))
	id = ask(own, Spawn(s.info, start))
	Task(TaskInfo(s.info, id), result, s)
}

go(A, E, N) = (
	s: Scope(E)
	body: () -> A ? E ! N
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

sync_body(A, E, N) = (e: Event(A, E, N), own: Scheduler) -> A ? E ! N {
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
	f: (A) -> B ? F ! M
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

join(A, E) = (t: Task(A, E)) -> A ? E ! Scheduler { sync(join_event(t)) }

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
	f: (S) -> A ? E ! N
	_own ! Scheduler
) -> { (S) -> A ? E ! Scheduler } ! Scheduler | N - Scheduler - Runner - request.Await {
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

both(A, B, E, N) = (
	left: () -> A ? E ! N
	right: () -> B ? E ! N
) -> (A, B) ? E ! Scheduler | N - Scheduler - Runner - request.Await {
	scope((s) {
		a = go(s, left)
		b = go(s, right)
		(join(a), join(b))
	})
}

race(A, E, N) = (
	left: () -> A ? E ! N
	right: () -> A ? E ! N
	own ! Scheduler
) -> A ? E ! Scheduler | N - Scheduler - Runner - request.Await {
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
	f: (A) -> B ? E ! N
) -> List(Task(B, E)) ! Scheduler | N - Scheduler - Runner - request.Await {
	xs |> {
		[] -> []
		[x, ..rest] -> {
			child = go(s, () { f(x) })
			[child, ..spawn_values(s, rest, f)]
		}
	}
}

join_values(A, E) = (tasks: List(Task(A, E))) -> List(A) ? E ! Scheduler {
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
	f: (A) -> B ? E ! N
) -> List(B) ? E ! Scheduler | N - Scheduler - Runner - request.Await {
	scope((s) { join_values(spawn_values(s, xs, f)) })
}

execute_body(A, E, N) = (
	body: () -> A ? E ! N
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
				Absent
				Absent
				[]
				False
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
				[root_task]
				[]
				[]
				[]
				Absent
				[]
				[prefix]
				[]
				[]
				choices
				scripted
				Absent
				Absent
				Absent
				0
				0
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
	body: () -> A ? E ! N
	runner ! Runner
) -> A ? E ! Runner | N - Scheduler - Runner - request.Await - request.Owner - request.Root {
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
	body: () -> A ? E ! N
	runner ! Runner
) -> A ? E | ScheduleError ! Runner | N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	execute(body, runner, seed, Script(choices)) |> {
		Ok(result)   -> fail.from_result(result)
		Error(error) -> fail.raise(error)
	}
}

entry(A, E, N) = (
	body: () -> A ? E ! N
	root ! request.Root
) -> A ? E ! request.Root | N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	! Runner = initial(root)
	retain(run(body))
}

scope_call(A, E, N) = (
	body: (Scope(E)) -> A ? E ! N
	witness: Witness
	mode: Mode
	own: Scheduler
) -> ScopeResult(A) ? E ! N {
	kept = control.keep_env((_: Nil) {
		scope_call_body(body, witness, mode, own)
	})
	retain(kept(Nil))
}

execute(A, E, N) = (
	body: () -> A ? E ! N
	runner: Runner
	seed: rng.Generator
	policy: Policy
) -> Result(Result(A, E), ScheduleError) ! N - Scheduler - Runner - request.Await - request.Owner - request.Root {
	kept = control.keep_env((_: Nil) {
		execute_body(body, runner, seed, policy)
	})
	retain(kept(Nil))
}

sync(A, E, N) = (e: Event(A, E, N), own ! Scheduler) -> A ? E ! N | Scheduler {
	kept = control.keep_env((_: Nil) { sync_body(e, own) })
	retain(kept(Nil))
}

root_active = (_run: Key(Nil), _active: Active) { ContinueRun }

cleanup_chain = (s: State, chain: List(Address)) -> Bool {
	any(chain, (id: Address) { find_scope(s.scopes, id).cleanup })
}

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
		[f, .._] -> f.unwinding |> {
			Absent     -> Present(f.interrupt)
			Present(_) -> Absent
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
		key.wrap(path_key, t.path)
		frame_owner(t.frames)
		frame_epoch(t.frames)
		list_map(t.foreign, (guard: ForeignGuard) { guard.token })
	)
	own = { cleanup_chain(s, t.chain) || foreign_active(t) } |> {
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

register_driver = (s: State, task_path: List(Int), command: Command) -> State {
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
	next = remove_registration(
		set_task(s, TaskState(..t, status: status))
		t.path
	)
	queue(State(..next, waiting: without(next.waiting, t.path)), t.path)
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

deliver_deferred = (s: State, path: List(Int), command: Command) -> State {
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
	next = remove_registration(
		set_task(s, TaskState(..t, status: status))
		t.path
	)
	queue(State(..next, waiting: without(next.waiting, t.path)), t.path)
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
	alive = any(s.scopes, (scope: ScopeState) { scope.address == id })
	{ alive } |> {
		False -> reply(s, t.path, k, Proceed(unit()), False)
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
			False -> frame.unwinding |> {
				Present(_) -> True
				Absent     -> unwinding_above(rest, anchor)
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
			path = read(path_key, hop.task)
			alive = any(s.tasks, (t: TaskState) { t.path == path })
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

pump_task_messages = (s: State, path: List(Int)) -> State {
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

pump_rows = (s: State, rows: List(TaskState)) -> State {
	rows |> {
		[] -> s
		[t, ..rest] -> t.messages |> {
			[] -> pump_rows(s, rest)
			_  -> pump_rows(pump_task_messages(s, t.path), rest)
		}
	}
}

pump_messages = (s: State) { pump_rows(s, s.tasks) }

relay_notice = (
	s: State
	t: TaskState
	k: Continuation(StepAnswer, Turn, Never, Never)
	notice: ForeignNotice
	value: Keyed
	discard: Bool
) -> State {
	base = reply(s, t.path, k, Proceed(value), False)
	next = pump_messages(post_notice(base, notice))
	current = find_task(next.tasks, t.path)
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
				EndForeign(..) -> reply(next, t.path, k, Proceed(value), True)
			}
			False -> reply(next, t.path, k, Proceed(value), True)
		}
	}
}

notice_hops = (notice: ForeignNotice) -> List(Hop) {
	notice |> {
		BeginForeign(hops, _, _, _) -> hops
		EndForeign(hops, _)         -> hops
	}
}
