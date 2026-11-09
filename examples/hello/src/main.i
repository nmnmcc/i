//// A first program in i. `i run` and `i test` give it the host service
//// `Io` (`i:io/v1`), so it can print: `i run` prints `Hello from i!`.

export (
	Greeting(..)
	Io
	greet
	repeat
)

/// The host service that prints a line (§9.7). The host gives it.
@host("i:io/v1")
Io {
	Io(print: (text: String) -> Nil)
}

Greeting {
	Greeting(name: String, times: Int)
}

/// `text` `times` times, with `sep` between.
repeat = (text: String, times: Int, sep: String) -> String {
	case times {
		0 -> ""
		1 -> text
		_ -> text <> sep <> repeat(text, times - 1, sep)
	}
}

greet = (g: Greeting) { repeat("hello " <> g.name <> "!", g.times, " ") }

@main
main = (io ! Io) { io.print("Hello from i!") }
