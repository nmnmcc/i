(i/request)
@host("test:async/v1", "request")
Later { Later(echo: (text: String) -> Request(String)) }
@main
main = (later ! Later) { later.echo("pending")
 panic("stop-before-answer") }
