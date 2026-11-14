// spec: 8.4
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

ints = (b : BitArray) {
	b |>
		(<<>>) { "empty" }
		| (<<0xFF:i8>>) { "never: i8 cannot be 255" }
		| (<<-1:i8>>) { "minus one" }
		| (<<x>>) { "byte " <> show(x) }
		| (<<x:u16be>>) { "u16be " <> show(x) }
		| (<<x:i24le>>) { "i24le " <> show(x) }
		| (<<a:u4, b:u4, c:u3>>) { "nibbles " <> show((a, b, c)) }
		| (_) { "other" }
}

test("integers", () {
	check(ints(<<>>), "empty")
	check(ints(<<255>>), "minus one")
	check(ints(<<7>>), "byte 7")
	check(ints(<<1, 2>>), "u16be 258")
	check(ints(<<255, 255, 255>>), "i24le -1")
	check(ints(<<254, 255, 127>>), "i24le 8388606")
	check(ints(<<18, 3:u3>>), "nibbles (1, 2, 3)")
	check(ints(<<1, 2, 3, 4>>), "other")
})

lengths = (b : BitArray) {
	b |>
		(<<n, data:bytes(n), rest:bits>>) { show((n, data, rest)) }
		| (_) { "no" }
}

test("dynamic_lengths", () {
	check(lengths(<<2, 10, 20, 30>>), "(2, <<10, 20>>, <<30>>)")
	check(lengths(<<0>>), "(0, <<>>, <<>>)")
	check(lengths(<<5, 1>>), "no")
	<<3:u4, 5:u3, 1:u1>> |>
		(<<n:u4, x:u(n), rest:bits>>) {
			check(show((n, x, rest)), "(3, 5, <<1:u1>>)")
		}
		| (_) { panic("no") }
	<<1, 2, 3>> |>
		(<<a:bits(4), b:bits>>) {
			check(show((a, b)), "(<<0:u4>>, <<16, 32, 3:u4>>)")
		}
		| (_) { panic("no") }
})

test("bytes_need_whole", () {
	r = <<1, 2:u4>> |>
		(<<_, rest:bytes>>) { "bytes " <> show(rest) }
		| (<<_, rest:bits>>) { "bits " <> show(rest) }
		| (_) { "no" }
	check(r, "bits <<2:u4>>")
})

floats = (b : BitArray) {
	b |>
		(<<1.5:f64be>>) { "one and a half" }
		| (<<x:f32le>>) { "f32le " <> show(x) }
		| (<<x:f16be>>) { "f16be " <> show(x) }
		| (_) { "other" }
}

test("float_segments", () {
	check(floats(<<1.5:f64be>>), "one and a half")
	check(floats(<<2.0:f32le>>), "f32le 2.0")
	check(floats(<<60, 0>>), "f16be 1.0")
	// Not finite: infinity and NaN do not match.
	check(floats(<<0, 0, 128, 127>>), "other")
	check(floats(<<124, 0>>), "other")
	check(floats(<<126, 0>>), "other")
})

texts = (b : BitArray) {
	b |>
		(<<"hi":utf8, rest:bits>>) { "hi then " <> show(rest) }
		| (<<"é":utf16be>>) { "é in utf16be" }
		| (<<"x":utf32le>>) { "x in utf32le" }
		| (<<c:utf8_codepoint, rest:bytes>>) {
			"codepoint " <> show(c) <> " then " <> show(rest)
		}
		| (_) { "other" }
}

test("string_segments", () {
	check(texts(<<"hi!":utf8>>), "hi then <<33>>")
	check(texts(<<0, 233>>), "é in utf16be")
	check(texts(<<120, 0, 0, 0>>), "x in utf32le")
	check(texts(<<"€a":utf8>>), "codepoint //utfcodepoint(€) then <<97>>")
	check(texts(<<255>>), "other")
	check(texts(<<195>>), "other")
	<<"😀":utf16le>> |>
		(<<c:utf16le_codepoint>>) {
			check(
				show(core.codepoint_to_int(c)),
				"128512",
			)
		}
		| (_) { panic("no") }
	<<"😀":utf32be>> |>
		(<<c:utf32be_codepoint>>) { check(show(c), "//utfcodepoint(😀)") }
		| (_) { panic("no") }
})

test("not_enough_bits", () {
	r = <<1>> |>
		(<<_:u16be>>) { "yes" }
		| (<<_:u4>>) { "not all bits used" }
		| (_) { "no" }
	check(r, "no")
	big = 100000000000000000000
	r2 = <<1, 2>> |>
		(<<_:bits(big)>>) { "yes" }
		| (<<_:bits(-1), _:bits>>) { "negative" }
		| (_) { "no" }
	check(r2, "no")
})
