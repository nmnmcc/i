//// Fetch transport. Response handles belong to the current resource context.
(
	i/request
	i/hostcall
	i/resource
	i/fail
)

(
	Fetch(..)
	Response
	open
	status
	headers
	read
	close
)

Response = (Int)

@import("i:fetch/v1")
Fetch = (
	start = (String, String, String, BitArray) -> Result(Int, String),
	status = (Int) -> Result(Int, String),
	headers = (Int) -> Result(String, String),
	read = (Int, Int) -> Result(BitArray, String),
	close = (Int) -> (),
)

open = (method, url, headers, body) <- (raw : Fetch) {
	number = resource.acquire(
		() {
			r = hostcall.call(() { raw.start(method, url, headers, body) })
			resource.ensure(
				() {
					request.await(r) |>
						(Ok(value)) { value }
						& (Error(code)) { fail.raise(code) }
				},
				() {
					request.state(r) |>
						(request.Ready(Ok(value))) { raw.close(value) }
						& (_) { () }
					hostcall.cancel(r)
				},
			)
		},
		(value) { close(Response(value)) <- (raw) },
	)
	Response(number)
}

status = (Response(id)) <- (raw : Fetch) {
	raw.status(id) |> (Ok(value)) { value } & (Error(code)) { fail.raise(code) }
}

headers = (Response(id)) <- (raw : Fetch) {
	raw.headers(id) |>
		(Ok(value)) { value }
		& (Error(code)) { fail.raise(code) }
}

read = (Response(id), max) <- (raw : Fetch) {
	raw.read(id, max) |>
		(Ok(value)) { value }
		& (Error(code)) { fail.raise(code) }
}

close = (Response(id)) <- (raw : Fetch) { raw.close(id) }
