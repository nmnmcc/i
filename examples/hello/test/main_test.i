(
	testlib = (test) = std/test
	hello/main
)

test("repeats", () {
	main.repeat("a", 3, "-") |> ("a-a-a") { Nil } | (other) { panic(other) }
})

test("greets_by_label", () {
	main.greet(main.Greeting(times = 1, name = "i")) |>
		("hello i!") { Nil }
		| (other) { panic(other) }
})
