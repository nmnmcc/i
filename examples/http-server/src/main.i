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

User {
	User(name: String, age: Int)
}

user_schema = () {
	names = schema.field(
		schema.object(User("", 0))
		"name"
		schema.string
		(u) { u.name }
		(u, name) { User(name, u.age) }
	)
	schema.field(names, "age", schema.int, (u) { u.age }, (u, age) {
		User(u.name, age)
	})
}

reply = (status, text) {
	r = http.response(http.Status(status))
	http.Response(
		r.status
		r.reason
		http.Headers([("Content-Type", "application/json")])
		<<text:utf8>>
		http.Headers([])
	)
}

handle = (req: http.Request) {
	(req.method, req.target) |> {
		(http.Method("GET"), "/health") -> {
			r = http.response(http.Status(200))
			http.Response(
				r.status
				r.reason
				r.headers
				<<"ok":utf8>>
				r.trailers
			)
		}
		(http.Method("POST"), "/users") -> bit_array.to_string(req.body) |> {
			Ok(text) -> schema.parse(user_schema(), text) |> {
				Ok(user) -> reply(
					200
					json.print(
						json.Object([
							("id", json.Integer(1))
							("name", json.Text(user.name))
						])
					)
				)
				Error(schema.Decode(error)) -> reply(
					422
					core.inspect(error.path)
				)
				Error(_) -> reply(422, "invalid JSON")
			}
			Error(_) -> reply(422, "invalid UTF-8")
		}
		_ -> reply(404, "not found")
	}
}

@main
main = () {
	listener = net.listen("127.0.0.1", 0)
	stdio.print("listening " <> core.inspect(net.local_port(listener)))
	task.run(() { http.serve(listener, handle) })
}
