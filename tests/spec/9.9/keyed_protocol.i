// spec: 9.8, 9.9
// expect: pass
// runtime.md 6.3: heterogeneous child results and two channel element types
// use typed keys and one monomorphic scheduling continuation.
(
	i/control
	i/key
	i/fail
)

Proceed = (Keyed)

StepAnswer = Proceed
Send = (Keyed)
Receive = ()
Command = Send | Receive

Step = (Command, Continuation(StepAnswer, Turn, Never, Never))
Done = (Keyed)
Turn = Step | Done

Scheduler = (step = (Command) -> StepAnswer)

Channel(A) = (Key(A))

Start = (run = (Scheduler) -> Turn)

read = (k, value) {
	key.unwrap(k, value) |>
		(key.Found(v)) { v }
		| (key.Missing) { panic("prototype: wrong key") }
}

identity = (x) { x }

send = (c, value) <- (scheduler : Scheduler) {
	Channel(k) = c
	scheduler.step(Send(key.wrap(k, value)))
	Nil
}

receive = (c) <- (scheduler : Scheduler) {
	Channel(k) = c
	Proceed(value) = scheduler.step(Receive)
	read(k, value)
}

start = (body, result_key) {
	{
		kept = control.keep_env((own : Scheduler) {
			identity(
				Done(key.wrap(result_key, fail.catch(body))),
			) <- (own)
		})
		Start(run = kept)
	} <- (Scheduler(step = (_) { panic("prototype: detached") }))
}

begin = (start : Start) {
	control.boundary((b) {
		start.run(
			Scheduler(
				step = (command) {
					control.suspend(b, (k) { Step(command, k) })
				},
			),
		)
	})
}

main = () {
	ints = Channel(key.new())
	strings = Channel(key.new())
	int_result = key.new()
	string_result = key.new()
	a = start(
		() {
			send(ints, 37)
			value = receive(strings)
			assert(value == "hello")
			41
		},
		int_result,
	)
	b = start(
		() {
			value = receive(ints)
			assert(value == 37)
			send(strings, "hello")
			"done"
		},
		string_result,
	)
	Step(Send(number), a1) = begin(a)
	Step(Receive, b1) = begin(b)
	Step(Send(text), b2) = control.resume(b1, Proceed(number))
	Step(Receive, a2) = control.resume(
		a1,
		Proceed(key.wrap(key.new(), Nil)),
	)
	Done(ar) = control.resume(a2, Proceed(text))
	Done(br) = control.resume(
		b2,
		Proceed(key.wrap(key.new(), Nil)),
	)
	assert(read(int_result, ar) == Ok(41))
	assert(read(string_result, br) == Ok("done"))
}
