//// A first program in i. It imports the standard service `Io` from `i/io`
//// (§9.7), and `i run` gives it, so the program can print: `i run` prints
//// `Hello from i!`.
(
	(Io) = i/io
)

(
	Greeting(..)
	greet
	repeat
)

Greeting = (name = String, times = Int)

/// `text` `times` times, with `sep` between.
repeat = (text: String, times: Int, sep: String) -> String {
	times |>
		(0) { "" }
		| (1) { text }
		| (_) { text <> sep <> repeat(text, times - 1, sep) }
}

greet = (g: Greeting) { repeat("hello \(g.name)!", g.times, " ") }

// Program entry.
main = () <- (io: Io) { io.print("Hello from i!") }
