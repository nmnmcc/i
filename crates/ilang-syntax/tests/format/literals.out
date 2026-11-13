numbers = (1_000_000, 0xFF_FF, 0o7_7, 0b1010_0101, 1_0.5e-1_0, 3.)

strings = (
	"tab\there",
	"quote \" and backslash \\",
	"\u{1F600} and \u{e9}",
	"line one\n  line two\nline three",
)

patterns = (n) {
	n |>
		(-1000) { "minus a thousand" }
		| (0x10) { "sixteen" }
		| ("pre\n" <> rest) { rest }
		| (-.1.5) { "minus one and a half" }
		| (_) { "other" }
}

index = (t) { t.0 + t.1 + { 1 }.x }
