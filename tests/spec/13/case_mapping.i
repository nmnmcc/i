// spec: 13
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

test("ascii", () {
	check(core.to_upper("Hello, World 42!"), "HELLO, WORLD 42!")
	check(core.to_lower("Hello, World 42!"), "hello, world 42!")
	check(core.to_upper(""), "")
	check(core.to_lower(""), "")
})

test("greek_and_cyrillic", () {
	check(core.to_upper("αβγ ωμέγα"), "ΑΒΓ ΩΜΈΓΑ")
	check(core.to_lower("ΑΒΓ ΩΜΈΓΑ"), "αβγ ωμέγα")
	// Final sigma has the simple uppercase mapping of sigma.
	check(core.to_upper("ς"), "Σ")
	check(core.to_lower("Σ"), "σ")
	check(core.to_upper("привет, мир"), "ПРИВЕТ, МИР")
	check(core.to_lower("ПРИВЕТ, МИР"), "привет, мир")
})

test("one_character_for_one", () {
	// ß has no simple uppercase mapping; capital ẞ maps down to it.
	check(core.to_upper("straße"), "STRAßE")
	check(core.to_lower("ẞ"), "ß")
	// Dotless ı goes up to I, and I comes down to i, with no locale.
	check(core.to_upper("ı"), "I")
	check(core.to_lower("I"), "i")
	check(core.to_lower("İ"), "i")
	check(core.to_upper("i"), "I")
	// The title case letter ǅ.
	check(core.to_upper("ǅ"), "Ǆ")
	check(core.to_lower("ǅ"), "ǆ")
	// ŉ has only a special uppercase mapping, so it stays.
	check(core.to_upper("ŉ"), "ŉ")
	// A mapping may change the number of bytes: ⱥ is three, Ⱥ is two.
	check(core.to_upper("ⱥ"), "Ⱥ")
	check(core.to_lower("Ⱥ"), "ⱥ")
	check(
		core.inspect(
			core.byte_size(core.to_lower("ȺȺ")),
		),
		"6",
	)
})

test("outside_the_basic_plane", () {
	// Deseret.
	check(core.to_upper("𐐨𐐩𐐪"), "𐐀𐐁𐐂")
	check(core.to_lower("𐐀𐐁𐐂"), "𐐨𐐩𐐪")
	check(core.to_upper("a😀b"), "A😀B")
	check(core.to_lower("日本"), "日本")
})
