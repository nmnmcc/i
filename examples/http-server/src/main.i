(
	i/net
	i/task
	i/stdio
	i/core
	std/http
	std/json
	std/schema
	std/bit_array
)

User = (name = String, age = Int)

user_schema = () {
	names = schema.field(
		schema.object(User(name = "", age = 0)),
		"name",
		schema.string,
		(u) { u.name },
		(u, name) { User(name = name, age = u.age) },
	)
	schema.field(
		names,
		"age",
		schema.int,
		(u) { u.age },
		(u, age) { User(name = u.name, age = age) },
	)
}

reply = (status, text) {
	r = http.response(http.Status(status))
	http.Response(
		status = r.status,
		reason = r.reason,
		headers = http.Headers([("Content-Type", "application/json")]),
		body = <<text:utf8>>,
		trailers = http.Headers([]),
	)
}

handle = (req : http.Request) {
	(req.method, req.target) |>
		((http.Method("GET"), "/health")) {
			r = http.response(http.Status(200))
			http.Response(
				status = r.status,
				reason = r.reason,
				headers = r.headers,
				body = <<"ok":utf8>>,
				trailers = r.trailers,
			)
		}
		| ((http.Method("POST"), "/users")) {
			bit_array.to_string(req.body) |>
				(Ok(text)) {
					schema.parse(user_schema(), text) |>
						(Ok(user)) {
							reply(
								200,
								json.print(
									json.Object(
										[
											("id", json.Integer(1)),
											("name", json.Text(user.name)),
										],
									),
								),
							)
						}
						| (Error(schema.Decode(error))) {
							reply(422, core.inspect(error.path))
						}
						| (Error(_)) { reply(422, "invalid JSON") }
				}
				| (Error(_)) { reply(422, "invalid UTF-8") }
		}
		| (_) { reply(404, "not found") }
}

main = () {
	listener = net.listen("127.0.0.1", 0)
	stdio.print(
		"listening " <> core.inspect(net.local_port(listener)),
	)
	task.run(() { http.serve(listener, handle) })
}
