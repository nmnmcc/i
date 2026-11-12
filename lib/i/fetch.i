//// Fetch transport. Response handles belong to the current resource context.
(
	i/request
	i/resource
	i/fail
)

(
	Fetch
	Response
	open
	status
	headers
	read
	close
)

Response {
	Response(Int)
}

@host("i:fetch/v1", "request")
Fetch {
	Fetch(
		start: (
			method:  String
			url:     String
			headers: String
			body:    BitArray
		) -> Request(Result(Int, String))
		status:  (id: Int) -> Request(Result(Int, String))
		headers: (id: Int) -> Request(Result(String, String))
		read:    (id: Int, max: Int) -> Request(Result(BitArray, String))
		close:   (id: Int) -> Request(Nil)
	)
}

open = (method, url, headers, body, raw ! Fetch) {
	r = raw.start(method, url, headers, body)
	number = resource.ensure(() {
		request.await(r) |> {
			Ok(value)   -> value
			Error(code) -> fail.raise(code)
		}
	}, () {
		request.state(r) |> {
			request.Ready(Ok(value)) -> request.await_owned(raw.close(value))
			_                        -> Nil
		}
		request.cancel(request.ticket(r))
	})
	Response(number)
}

status = (Response(id), raw ! Fetch) {
	request.await_owned(raw.status(id)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(code)
	}
}

headers = (Response(id), raw ! Fetch) {
	request.await_owned(raw.headers(id)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(code)
	}
}

read = (Response(id), max, raw ! Fetch) {
	request.await_owned(raw.read(id, max)) |> {
		Ok(value)   -> value
		Error(code) -> fail.raise(code)
	}
}

close = (Response(id), raw ! Fetch) { request.await_owned(raw.close(id)) }
