(
	i/fail
	i/stdio
	std/http
	std/string
)

main = () {
	// The program chooses the HTTP client of its platform (§2.10.11).
	{
		res = http.get("https://example.com")
		assert(res.status == http.Status(200))
		text = fail.catch(() { http.body_text(res) }) |>
			(Ok(value)) { value }
			& (Error(_)) { panic("invalid response text") }
		assert(string.contains(text, "Example Domain"))
		stdio.print("http ok")
	} <- (http.native_client())
}
