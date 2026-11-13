//// Int wire boundary tests, shared by the Rust and JavaScript hosts.
(
	app/app
	i/fail
)

(
	echo
	import_echo
	async_echo
	fail_echo
)

@external("custom", "echo_int")
foreign_echo: (Int) -> Int

echo = (value: Int) -> Int { value }

import_echo = (value: Int) -> Int { foreign_echo(value) }

async_echo = (value: Int) <- (clock: app.Clock) -> Int { clock.tick(value) }

fail_echo = (value: Int) -> Nil { fail.raise(value) }
