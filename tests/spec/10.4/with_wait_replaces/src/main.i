(
	i/hostcall
	i/request
)

(
	main
)

main = () {
	// The fake wait takes this cell, so `Taken` proves that it ran.
	marker = request.ready(())
	w = hostcall.Wait(wait = (_) { request.take(marker) })
	got = hostcall.with_wait(w, () { hostcall.await(request.ready(3)) })
	assert(got == 3)
	assert(request.state(marker) == request.Taken())
	// Outside the body the ambient wait is back.
	again = request.ready(())
	w2 = hostcall.Wait(wait = (_) { request.take(again) })
	hostcall.with_wait(w2, () { () })
	assert(hostcall.await(request.ready(4)) == 4)
	assert(request.state(again) == request.Ready(()))
}
