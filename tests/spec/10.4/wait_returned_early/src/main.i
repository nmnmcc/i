(
	i/hostcall
)

(
	Later(..)
	main
)

@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))

main = () <- (later : Later) {
	w = hostcall.Wait(wait = (_) { () })
	c = hostcall.adopt(later.echo("x"))
	hostcall.with_wait(w, () { hostcall.await(c) })
}
