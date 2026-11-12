(
	i/fail
	i/core
	std/http
)

(
	get
)

get = (url: String) {
	fail.catch(() { http.get(url) }) |>
		(Ok(res)) {
			http.body_text(res) |>
				(Ok(text)) { text }
				| (Error(_)) { panic("invalid UTF-8") }
		}
		| (Error(error)) { fail.raise(core.inspect(error)) }
}
