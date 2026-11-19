(
	testlib = (test) = i/test
	hello/main
)

test("repeats", () {
	main.repeat("a", 3, "-") |> ("a-a-a") { () } & (other) { panic(other) }
})

test("greets_by_label", () {
	main.greet(main.Greeting(times = 1, name = "i")) |>
		("hello i!") { () }
		& (other) { panic(other) }
})
