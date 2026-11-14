(
	i/process
	i/fail
	i/stdio
	std/int
)

(
	read_port
	BadPort(..)
)
BadPort = ()
read_port = (name) {
	int.parse(process.env(name)) |>
		(Ok(port) ? port > 0 && port < 65536) { port }
		| (_) { fail.raise(BadPort) }
}
main = () {
	text = fail.catch(() { read_port("PORT") }) |>
		(Ok(port)) { "port " <> int.to_string(port) }
		| (Error(_)) { "invalid PORT" }
	stdio.print(text)
}
