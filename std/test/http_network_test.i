(
	testlib = (test) = std/test
	i/net
	i/task
	i/core
	i/fail
	i/clock
	std/http
)

handle = (req: http.Request) {
	(req.method, req.target) |> {
		(http.Method("GET"), "/redirect") -> {
			r = http.response(http.Status(302))
			http.Response(
				r.status
				r.reason
				http.Headers([("Location", "/chunked")])
				<<>>
				r.trailers
			)
		}
		(http.Method("GET"), "/chunked") -> {
			r = http.response(http.Status(200))
			http.Response(
				r.status
				r.reason
				http.Headers([("Transfer-Encoding", "chunked")])
				<<"chunked":utf8>>
				r.trailers
			)
		}
		(http.Method("POST"), "/post") -> {
			r = http.response(http.Status(200))
			http.Response(r.status, r.reason, r.headers, req.body, r.trailers)
		}
		_ -> http.response(http.Status(404))
	}
}

test("network_client", () {
	listener = net.listen("127.0.0.1", 0)
	url = "http://127.0.0.1:" <> core.inspect(net.local_port(listener))
	task.run(() {
		task.race(() { http.serve(listener, handle) }, () {
			assert(http.get(url <> "/chunked").body == <<"chunked":utf8>>)
			assert(http.get(url <> "/redirect").body == <<"chunked":utf8>>)
			assert(
				http.post(url <> "/post", <<"hello":utf8>>).body ==
					<<"hello":utf8>>
			)
			Nil
		})
	})
	net.close_listener(listener)
})

test("client_timeout", () {
	result = task.run(() {
		provider = (send: (_: http.Request) { task.sync(task.never()) })
		fail.catch(() {
			http.send_timeout(
				provider
				http.request(http.Method("GET"), "http://localhost")
				1000000
			)
		})
	})
	assert(result == Error(http.Timeout))
})
