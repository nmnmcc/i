(
	i/fail
	testlib = (test) = i/test
	i/core
	i/regex
	i/list
	i/string
	(Some, None) = i/option
)

compile = (pattern : String) -> regex.Regex {
	fail.catch(() { regex.compile(pattern) }) |>
		(Ok(re)) { re }
		& (Error(_)) { panic("test pattern") }
}

text = (pattern : String, input : String) -> String {
	regex.find(compile(pattern), input) |>
		(Some(m)) { m.text }
		& (None()) { "<no match>" }
}

test("literal", () {
	assert(text("abc", "--abc--") == "abc")
	()
})

test("missing", () {
	assert(text("abc", "ab") == "<no match>")
	()
})

test("dot_unicode", () {
	assert(text(".", "🦀") == "🦀")
	()
})

test("dot_no_lf", () {
	assert(text(".", "\n") == "<no match>")
	()
})

test("dotall", () {
	assert(text("(?s).", "\n") == "\n")
	()
})

test("range", () {
	assert(text("[a-z]+", "123hello!") == "hello")
	()
})

test("negated", () {
	assert(text("[^a-c]+", "abcXYZ") == "XYZ")
	()
})

test("digit", () {
	assert(text("\\d+", "x123z") == "123")
	()
})

test("digit_unicode", () {
	assert(text("\\d+", "x١٢٣!") == "١٢٣")
	()
})

test("not_digit", () {
	assert(text("\\D+", "12abc3") == "abc")
	()
})

test("word", () {
	assert(text("\\w+", "--café_2!") == "café_2")
	()
})

test("word_unicode", () {
	assert(text("\\w+", "--汉字!") == "汉字")
	()
})

test("not_word", () {
	assert(text("\\W+", "word !ok") == " !")
	()
})

test("space", () {
	assert(text("\\s+", "a\t \nb") == "\t \n")
	()
})

test("space_unicode", () {
	assert(text("\\s+", "a  b") == "  ")
	()
})

test("not_space", () {
	assert(text("\\S+", " \na! ") == "a!")
	()
})

test("boundary", () {
	assert(text("\\bcat\\b", "scat cat!") == "cat")
	()
})

test("not_boundary", () {
	assert(text("\\Bcat", "scat cat") == "cat")
	()
})

test("unicode_boundary", () {
	assert(text("\\bélan\\b", "élan!") == "élan")
	()
})

test("greedy_star", () {
	assert(text("a*", "aaab") == "aaa")
	()
})

test("lazy_star", () {
	assert(text("a*?", "aaab") == "")
	()
})

test("greedy_plus", () {
	assert(text("a+", "baaa") == "aaa")
	()
})

test("lazy_plus", () {
	assert(text("a+?", "baaa") == "a")
	()
})

test("greedy_optional", () {
	assert(text("a?", "a") == "a")
	()
})

test("lazy_optional", () {
	assert(text("a??", "a") == "")
	()
})

test("exact", () {
	assert(text("a{3}", "aaaa") == "aaa")
	()
})

test("bounded", () {
	assert(text("a{2,4}", "aaaaa") == "aaaa")
	()
})

test("bounded_lazy", () {
	assert(text("a{2,4}?", "aaaaa") == "aa")
	()
})

test("open_bound", () {
	assert(text("a{2,}", "aaaa") == "aaaa")
	()
})

test("zero", () {
	assert(text("a{0}", "bbb") == "")
	()
})

test("zero_optional", () {
	assert(text("a{0,2}", "aaa") == "aa")
	()
})

test("alternate_first", () {
	assert(text("a|ab", "ab") == "a")
	()
})

test("alternate_second", () {
	assert(text("ab|a", "ab") == "ab")
	()
})

test("leftmost", () {
	assert(text("b|a", "ab") == "a")
	()
})

test("group", () {
	assert(text("(ab)+", "ababx") == "abab")
	()
})

test("noncapture", () {
	assert(text("(?:ab)+", "ababx") == "abab")
	()
})

test("start", () {
	assert(text("^a", "ba") == "<no match>")
	()
})

test("end", () {
	assert(text("a$", "ab") == "<no match>")
	()
})

test("dollar_strict", () {
	assert(text("a$", "a\n") == "<no match>")
	()
})

test("multiline", () {
	assert(text("(?m)^a$", "b\na\nc") == "a")
	()
})

test("absolute_start", () {
	assert(text("\\Aa", "a\na") == "a")
	()
})

test("absolute_end", () {
	assert(text("a\\z", "ba") == "a")
	()
})

test("case", () {
	assert(text("(?i)abc", "ABC") == "ABC")
	()
})

test("case_unicode", () {
	assert(text("(?i)é", "É") == "É")
	()
})

test("case_kelvin", () {
	assert(text("(?i)k", "K") == "K")
	()
})

test("case_longs", () {
	assert(text("(?i)[a-z]", "ſ") == "ſ")
	()
})

test("case_sigma", () {
	assert(text("(?i)σ", "ς") == "ς")
	()
})

test("case_negation", () {
	assert(text("(?i)[^a]+", "AaB") == "B")
	()
})

test("flag_scope", () {
	assert(text("((?i)a)b", "Ab") == "Ab")
	()
})

test("flag_restore", () {
	assert(text("((?i)a)b", "AB") == "<no match>")
	()
})

test("scoped_flags", () {
	assert(text("(?i:a)b", "Ab") == "Ab")
	()
})

test("unset_flag", () {
	assert(text("(?i)a(?-i)b", "Ab") == "Ab")
	()
})

test("class_escape", () {
	assert(text("[\\dA-F]+", "Z1AF!") == "1AF")
	()
})

test("class_negated_escape", () {
	assert(text("[^\\D]+", "a123b") == "123")
	()
})

test("escaped_dot", () {
	assert(text("\\.", "x.y") == ".")
	()
})

test("escaped_tab", () {
	assert(text("\\t+", "\t\tx") == "\t\t")
	()
})

test("hex", () {
	assert(text("\\x41+", "AAA") == "AAA")
	()
})

test("unicode_hex", () {
	assert(text("\\u{1f980}", "🦀") == "🦀")
	()
})

test("empty_alt", () {
	assert(text("|a", "a") == "")
	()
})

test("empty_group", () {
	assert(text("()", "a") == "")
	()
})

test("nested_empty", () {
	assert(text("(a*)*b", "aaab") == "aaab")
	()
})

test("greedy_follow", () {
	assert(text("a*ab", "aaaab") == "aaaab")
	()
})

test("lazy_follow", () {
	assert(text("a*?b", "aaab") == "aaab")
	()
})

test("branch_backtrack", () {
	assert(text("(ab|a)b", "ab") == "ab")
	()
})

test("class_dash", () {
	assert(text("[-a]+", "--aa") == "--aa")
	()
})

test("class_end_dash", () {
	assert(text("[a-]+", "aa--") == "aa--")
	()
})

test("astral_range", () {
	assert(text("[😀-🙏]+", "x😀🙏!") == "😀🙏")
	()
})

test("error_lookahead", () {
	fail.catch(() { regex.compile("(?=a)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_negative_lookahead", () {
	fail.catch(() { regex.compile("(?!a)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_lookbehind", () {
	fail.catch(() { regex.compile("(?<=a)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_negative_lookbehind", () {
	fail.catch(() { regex.compile("(?<!a)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_backreference", () {
	fail.catch(() { regex.compile("(a)\\1") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_named_backreference", () {
	fail.catch(() { regex.compile("\\k<a>") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_unclosed_group", () {
	fail.catch(() { regex.compile("(a") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_unclosed_class", () {
	fail.catch(() { regex.compile("[a") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_reverse_range", () {
	fail.catch(() { regex.compile("[z-a]") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_empty_class", () {
	fail.catch(() { regex.compile("[]") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_unknown_escape", () {
	fail.catch(() { regex.compile("\\q") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_trailing_escape", () {
	fail.catch(() { regex.compile("a\\") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_repeated_quantifier", () {
	fail.catch(() { regex.compile("a**") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_leading_quantifier", () {
	fail.catch(() { regex.compile("*a") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_reversed_bounds", () {
	fail.catch(() { regex.compile("a{3,2}") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_large_count", () {
	fail.catch(() { regex.compile("a{1001}") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_duplicate_name", () {
	fail.catch(() { regex.compile("(?<x>a)(?<x>b)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_invalid_name", () {
	fail.catch(() { regex.compile("(?<1x>a)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_empty_name", () {
	fail.catch(() { regex.compile("(?<>a)") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_unclosed_count", () {
	fail.catch(() { regex.compile("a{2") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_surrogate", () {
	fail.catch(() { regex.compile("\\x{d800}") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_large_scalar", () {
	fail.catch(() { regex.compile("\\u{110000}") }) |>
		(Error(regex.RegexError(position = position, reason = reason))) {
			assert(position >= 0)
			assert(reason != "")
			()
		}
		& (Ok(_)) { panic("expected compile error") }
	()
})

test("error_position", () {
	assert(
		fail.catch(() { regex.compile("a[") }) ==
			Error(
				regex.RegexError(
					position = 2,
					reason = "unclosed character class",
				),
			),
	)
	()
})

test("capture_index_name", () {
	c = regex.captures(compile("(?<first>a+)(b)?"), "xaa") |>
		(Some(c)) { c }
		& (None()) { panic("capture") }
	assert(
		regex.group(c, 0) == Some(regex.Match(start = 1, end = 3, text = "aa")),
	)
	assert(
		regex.group(c, 1) == Some(regex.Match(start = 1, end = 3, text = "aa")),
	)
	assert(regex.named(c, "first") == regex.group(c, 1))
	assert(regex.group(c, 2) == None())
	assert(regex.group(c, 3) == None())
	assert(regex.group(c, -1) == None())
	assert(regex.named(c, "unknown") == None())
	()
})

test("capture_last_iteration", () {
	c = regex.captures(compile("(a|b)+"), "ab") |>
		(Some(c)) { c }
		& (None()) { panic("capture") }
	assert(
		regex.group(c, 1) == Some(regex.Match(start = 1, end = 2, text = "b")),
	)
	()
})

test("capture_alternation", () {
	c = regex.captures(compile("(a)|(b)"), "b") |>
		(Some(c)) { c }
		& (None()) { panic("capture") }
	assert(regex.group(c, 1) == None())
	assert(
		regex.group(c, 2) == Some(regex.Match(start = 0, end = 1, text = "b")),
	)
	()
})

test("unicode_scalar_offsets", () {
	assert(
		regex.find(compile("🦀é"), "中🦀é!") ==
			Some(regex.Match(start = 1, end = 3, text = "🦀é")),
	)
	()
})

test("find_all", () {
	assert(
		regex.find_all(compile("a+"), "aa ba") ==
			[
				regex.Match(start = 0, end = 2, text = "aa"),
				regex.Match(start = 4, end = 5, text = "a"),
			],
	)
	()
})

test("find_all_empty", () {
	assert(
		regex.find_all(compile(""), "é🦀") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
			],
	)
	()
})

test("find_all_suppress_adjacent_empty", () {
	assert(
		regex.find_all(compile("a*"), "ab") ==
			[
				regex.Match(start = 0, end = 1, text = "a"),
				regex.Match(start = 2, end = 2, text = ""),
			],
	)
	()
})

test("find_all_lazy", () {
	assert(
		list.length(
			regex.find_all(compile("a*?"), "aa"),
		) ==
			3,
	)
	()
})

test("replace_first", () {
	assert(
		regex.replace(compile("(?<x>a)(b)"), "ab ab", "${x}-$2-$0-$$") ==
			"a-b-ab-$ ab",
	)
	()
})

test("replace_all", () {
	assert(
		regex.replace_all(compile("(a)"), "a a", "<$1>") == "<a> <a>",
	)
	()
})

test("replace_missing", () {
	assert(
		regex.replace(compile("(a)?b"), "b", "$1/$2/${missing}") == "//",
	)
	()
})

test("replace_braces", () {
	assert(
		regex.replace(compile("(a)"), "a", "${1}x") == "ax",
	)
	()
})

test("replace_longest_name", () {
	assert(
		regex.replace(compile("(a)"), "a", "$1x") == "",
	)
	()
})

test("replace_empty", () {
	assert(
		regex.replace_all(compile(""), "é🦀", "-") == "-é-🦀-",
	)
	()
})

test("replace_literal_dollar", () {
	assert(
		regex.replace(compile("a"), "a", "${x $!") == "${x $!",
	)
	()
})

test("replace_absent", () {
	assert(
		regex.replace_all(compile("z"), "abc", "x") == "abc",
	)
	()
})

test("split", () {
	assert(
		regex.split(compile("[,;]"), ",a;b,") == ["", "a", "b", ""],
	)
	()
})

test("split_empty", () {
	assert(
		regex.split(compile(""), "é🦀") == ["", "é", "🦀", ""],
	)
	()
})

test("split_absent", () {
	assert(
		regex.split(compile("z"), "abc") == ["abc"],
	)
	()
})

test("empty_input", () {
	assert(
		regex.is_match(compile(""), ""),
	)
	assert(
		regex.find(compile("a"), "") == None(),
	)
	()
})

test("catastrophic_5000", () {
	assert(
		regex.is_match(compile("(a*)*b"), string.repeat("a", 5000)) == False(),
	)
	()
})

// Rust regex testdata/empty.toml, 100.
test("rust_empty_100", () {
	assert(
		regex.find_all(compile("|b"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 110.
test("rust_empty_110", () {
	assert(
		regex.find_all(compile("b|"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 2, text = "b"),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 120.
test("rust_empty_120", () {
	assert(
		regex.find_all(compile("|z"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 130.
test("rust_empty_130", () {
	assert(
		regex.find_all(compile("z|"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 200.
test("rust_empty_200", () {
	assert(
		regex.find_all(compile("|"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 210.
test("rust_empty_210", () {
	assert(
		regex.find_all(compile("||"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 220.
test("rust_empty_220", () {
	assert(
		regex.find_all(compile("||b"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 230.
test("rust_empty_230", () {
	assert(
		regex.find_all(compile("b||"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 2, text = "b"),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 240.
test("rust_empty_240", () {
	assert(
		regex.find_all(compile("||z"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 300.
test("rust_empty_300", () {
	assert(
		regex.find_all(compile("(?:)|b"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 310.
test("rust_empty_310", () {
	assert(
		regex.find_all(compile("b|(?:)"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 2, text = "b"),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 320.
test("rust_empty_320", () {
	assert(
		regex.find_all(compile("(?:|)"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 330.
test("rust_empty_330", () {
	assert(
		regex.find_all(compile("(?:|)|z"), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 400.
test("rust_empty_400", () {
	assert(
		regex.find_all(compile("a(?:)|b"), "abc") ==
			[
				regex.Match(start = 0, end = 1, text = "a"),
				regex.Match(start = 1, end = 2, text = "b"),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 500.
test("rust_empty_500", () {
	assert(
		regex.find_all(compile(""), "") ==
			[regex.Match(start = 0, end = 0, text = "")],
	)
	()
})

// Rust regex testdata/empty.toml, 510.
test("rust_empty_510", () {
	assert(
		regex.find_all(compile(""), "a") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 520.
test("rust_empty_520", () {
	assert(
		regex.find_all(compile(""), "abc") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 600.
test("rust_empty_600", () {
	assert(
		regex.find_all(compile("(?:|a)*"), "aaa") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})

// Rust regex testdata/empty.toml, 610.
test("rust_empty_610", () {
	assert(
		regex.find_all(compile("(?:|a)+"), "aaa") ==
			[
				regex.Match(start = 0, end = 0, text = ""),
				regex.Match(start = 1, end = 1, text = ""),
				regex.Match(start = 2, end = 2, text = ""),
				regex.Match(start = 3, end = 3, text = ""),
			],
	)
	()
})
