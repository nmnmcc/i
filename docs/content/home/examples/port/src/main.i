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
read_port = () {
	int.parse(process.env("PORT")) |>
		(Ok(port) ? port > 0 && port < 65536) { port }
		| (_) { fail.raise(BadPort) }
}
main = () {
	fail.catch(read_port) |>
		(Ok(port)) { stdio.print("port " <> int.to_string(port)) }
		| (Error(_)) { stdio.print("invalid PORT") }
}
