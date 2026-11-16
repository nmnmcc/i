(
	i/fail
	i/core
	std/http
)

(
	get
)

get = (url: String) <- (handler: Fail(String)) {
	fail.catch(() { http.get(url) <- (http.fetch_client()) }) |>
		(Ok(res)) {
			fail.catch(() { http.body_text(res) }) |>
				(Ok(text)) { text }
				& (Error(_)) { panic("invalid UTF-8") }
		}
		& (Error(error)) { handler.raise(core.inspect(error)) }
}
