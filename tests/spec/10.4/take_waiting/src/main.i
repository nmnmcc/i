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
	hostcall.take(hostcall.adopt(later.echo("x")))
}
