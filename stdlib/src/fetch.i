//// Fetch transport. Response handles belong to the current resource context.
(
	i/request
	i/resource
	i/fail
)

(
	Fetch(..)
	Response(..)
	open
	status
	headers
	read
	close
)

Response = (Int)

@host("i:fetch/v1", "request")
Fetch = (
	start = (String, String, String, BitArray) -> Request(Result(Int, String)),
	status = (Int) -> Request(Result(Int, String)),
	headers = (Int) -> Request(Result(String, String)),
	read = (Int, Int) -> Request(Result(BitArray, String)),
	close = (Int) -> Request(()),
)

// Opens the host handle. A cancel while the open is in flight cancels the
// request and closes a handle that has arrived already.
open_handle = (method, url, headers, body) <- (raw : Fetch) {
	r = raw.start(method, url, headers, body)
	number = resource.ensure(
		() {
			request.await(r) |>
				(Ok(value)) { value }
				& (Error(code)) { fail.raise(code) }
		},
		() {
			request.state(r) |>
				(request.Ready(Ok(value))) {
					request.await_owned(raw.close(value))
				}
				& (_) { () }
			request.cancel(request.ticket(r))
		},
	)
	Response(number)
}

/// Opens a response and gives it owned by the current `resource.Scope`: its
/// `close` is attached to the scope before `open` gives the `Response`, and
/// runs when the scope ends.
///
/// Needs `resource.Scope`, `Fetch`, `Await` and `Fail(String)`.
open = (method, url, headers, body) {
	resource.acquire(
		() { open_handle(method, url, headers, body) },
		(response) { close(response) },
	)
}

status = (Response(id)) <- (raw : Fetch) {
	request.await_owned(raw.status(id)) |>
		(Ok(value)) { value }
		& (Error(code)) { fail.raise(code) }
}

headers = (Response(id)) <- (raw : Fetch) {
	request.await_owned(raw.headers(id)) |>
		(Ok(value)) { value }
		& (Error(code)) { fail.raise(code) }
}

read = (Response(id), max) <- (raw : Fetch) {
	request.await_owned(raw.read(id, max)) |>
		(Ok(value)) { value }
		& (Error(code)) { fail.raise(code) }
}

/// Closes a response. Closing a response that is closed already does nothing.
/// The scope that owns the response calls this; a program does not need to.
///
/// Needs `Fetch` and `Await`.
close = (Response(id)) <- (raw : Fetch) { request.await_owned(raw.close(id)) }
