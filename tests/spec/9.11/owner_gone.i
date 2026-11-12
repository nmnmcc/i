// spec: 9.11
// expect: stop "panic: request: owner is gone"
// runtime.md 5.1: a closed Owner refuses starts before making an ABI import.
(
	i/request
)
@host("test:async/v1", "request")
Later = (echo = (String) -> Request(String))

main = () <- (later: Later, owner: request.Owner) {
	request.close_owner(owner)
	later.echo("unreachable")
}
