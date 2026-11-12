(i/request)
@host("test:async/v1", "request")
Later { Later(echo: (String) -> Request(String)) }

main = (later ! Later) { later.echo("pending")
 panic("stop-before-answer") }
