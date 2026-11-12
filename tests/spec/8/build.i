// spec: 8.1, 8.2, 8.3
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

show = (x) { core.inspect(x) }

test("integers", () {
	check(show(<<>>), "<<>>")
	check(show(<<1, 2, 255>>), "<<1, 2, 255>>")
	check(show(<<256>>), "<<0>>")
	check(show(<<-1>>), "<<255>>")
	check(show(<<-1:i8>>), "<<255>>")
	check(show(<<1:u1, 0:u1, 1:u1>>), "<<5:u3>>")
	check(show(<<5:u3, 1:u5>>), "<<161>>")
	check(show(<<258:u16be>>), "<<1, 2>>")
	check(show(<<258:u16le>>), "<<2, 1>>")
	check(show(<<-2:i16be>>), "<<255, 254>>")
	check(show(<<1:u32le>>), "<<1, 0, 0, 0>>")
	check(show(<<1:u24be>>), "<<0, 0, 1>>")
	check(show(<<4294967296:u40be>>), "<<1, 0, 0, 0, 0>>")
	check(show(<<18446744073709551617:u72be>>), "<<1, 0, 0, 0, 0, 0, 0, 0, 1>>")
	check(show(<<3:u12>>), "<<0, 3:u4>>")
	n = 12
	check(show(<<3:u(n)>>), "<<0, 3:u4>>")
	check(show(<<-1:i(n - 2)>>), "<<255, 3:u2>>")
	check(show(<<7:u(0)>>), "<<>>")
})

test("floats", () {
	check(show(<<1.5:f64be>>), "<<63, 248, 0, 0, 0, 0, 0, 0>>")
	check(show(<<1.5:f64le>>), "<<0, 0, 0, 0, 0, 0, 248, 63>>")
	check(show(<<1.0:f32be>>), "<<63, 128, 0, 0>>")
	check(show(<<-.2.0:f32le>>), "<<0, 0, 0, 192>>")
	check(show(<<1.0:f16be>>), "<<60, 0>>")
	check(show(<<0.1:f16be>>), "<<46, 102>>")
})

test("bits_and_bytes", () {
	b = <<1, 2, 3>>
	check(show(<<b:bits>>), "<<1, 2, 3>>")
	check(show(<<b:bits(12)>>), "<<1, 0:u4>>")
	check(show(<<b:bytes(2), 9>>), "<<1, 2, 9>>")
	check(show(<<b:bytes>>), "<<1, 2, 3>>")
	check(show(<<1:u1, b:bits>>), "<<128, 129, 1, 1:u1>>")
	check(show(<<b:bits(0)>>), "<<>>")
})

test("strings", () {
	check(show(<<"hé":utf8>>), "<<104, 195, 169>>")
	check(show(<<"hé":utf16be>>), "<<0, 104, 0, 233>>")
	check(show(<<"h":utf16le>>), "<<104, 0>>")
	check(show(<<"😀":utf16be>>), "<<216, 61, 222, 0>>")
	check(show(<<"h":utf32be>>), "<<0, 0, 0, 104>>")
	check(show(<<"h":utf32le>>), "<<104, 0, 0, 0>>")
	cps = core.codepoints("é")
	cps |>
		([c]) {
			check(show(<<c:utf8_codepoint>>), "<<195, 169>>")
			check(show(<<c:utf16le_codepoint>>), "<<233, 0>>")
			check(show(<<c:utf32be_codepoint>>), "<<0, 0, 0, 233>>")
		}
		| (_) { panic("no") }
})

test("order", () {
	x = 5
	check(show(<<x, x + 1, { x * 2 }:u16be>>), "<<5, 6, 0, 10>>")
})
