(
	i/stdio
	std/http
	std/string
)

main = () {
	res = http.get("https://example.com")
	assert(res.status == http.Status(200))
	text = http.body_text(res) |> {
		Ok(value) -> value
		Error(_)  -> panic("invalid response text")
	}
	assert(string.contains(text, "Example Domain"))
	stdio.print("http ok")
}
