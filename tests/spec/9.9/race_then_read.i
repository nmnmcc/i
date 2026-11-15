// spec: 9.9, 9.11
// expect: pass
// runtime.md 5.4/R5: readiness waits borrow the producer; a readiness race
// returns a portable discriminator, then await_owned reads its still-Ready cell.
(
	i/task
	i/request
)

ready_wait = <A>(r : Request(A)) <- (await : request.Await) {
	await.wait(request.ticket(r))
}

main = () {
	a = request.ready(42)
	b = request.ready("answer")
	winner = task.race(
		() {
			ready_wait(a)
			1
		},
		() {
			ready_wait(b)
			2
		},
	)
	assert(winner == 1)
	assert(request.await_owned(a) == 42)
	assert(request.await_owned(b) == "answer")
}
