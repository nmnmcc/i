//// Reproduce with: i run --project std/bench/std4 --opt 2
(
	i/clock
	i/stdio
	std/crypto
	std/regex
	std/csv
	std/string
	std/list
	std/int
)

(
	main
)

measure = (label, size, body) {
	start = clock.monotonic_ns()
	body()
	elapsed = clock.monotonic_ns() - start
	stdio.print(
		label <>
			" bytes=" <>
			int.to_string(size) <>
			" ns=" <>
			int.to_string(elapsed)
	)
}

main = () {
	message = string.repeat("a", 1000000)
	data = <<message:utf8>>
	measure("sha256", 1000000, () {
		assert(
			crypto.sha256(data) ==
				<<
					0xcd
					0xc7
					0x6e
					0x5c
					0x99
					0x14
					0xfb
					0x92
					0x81
					0xa1
					0xc7
					0xe2
					0x84
					0xd7
					0x3e
					0x67
					0xf1
					0x80
					0x9a
					0x48
					0xa4
					0x97
					0x20
					0x0e
					0x04
					0x6d
					0x39
					0xcc
					0xc7
					0x11
					0x2c
					0xd0
				>>
		)
		Nil
	})
	re = regex.compile("^a+b$") |> {
		Ok(re)   -> re
		Error(_) -> panic("benchmark regex")
	}
	haystack = message <> "b"
	measure("regex_literal_repeat", 1000001, () {
		assert(regex.is_match(re, haystack))
		Nil
	})
	nested = regex.compile("(a*)*b") |> {
		Ok(re)   -> re
		Error(_) -> panic("benchmark regex")
	}
	adversarial = string.repeat("a", 5000)
	measure("regex_nested_repeat", 5000, () {
		assert(regex.is_match(nested, adversarial) == False)
		Nil
	})
	rows = list.repeat(["hello", "quoted,field", "é🦀"], 10000)
	text = csv.print(rows)
	measure("csv_parse", string.byte_size(text), () {
		assert(csv.parse(text) == Ok(rows))
		Nil
	})
	measure("csv_print", string.byte_size(text), () {
		assert(csv.print(rows) == text)
		Nil
	})
}
