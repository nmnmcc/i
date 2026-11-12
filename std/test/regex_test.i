(
	testlib = (test) = std/test
	i/core
	std/regex
	std/list
	std/string
	(Some, None) = std/option
)

compile = (pattern: String) -> regex.Regex {
	regex.compile(pattern) |> {
		Ok(re)   -> re
		Error(_) -> panic("test pattern")
	}
}

text = (pattern: String, input: String) -> String {
	regex.find(compile(pattern), input) |> {
		Some(m) -> m.text
		None    -> "<no match>"
	}
}

test("literal", () {
	assert(text("abc", "--abc--") == "abc")
	Nil
})

test("missing", () {
	assert(text("abc", "ab") == "<no match>")
	Nil
})

test("dot_unicode", () {
	assert(text(".", "🦀") == "🦀")
	Nil
})

test("dot_no_lf", () {
	assert(text(".", "\n") == "<no match>")
	Nil
})

test("dotall", () {
	assert(text("(?s).", "\n") == "\n")
	Nil
})

test("range", () {
	assert(text("[a-z]+", "123hello!") == "hello")
	Nil
})

test("negated", () {
	assert(text("[^a-c]+", "abcXYZ") == "XYZ")
	Nil
})

test("digit", () {
	assert(text("\\d+", "x123z") == "123")
	Nil
})

test("digit_unicode", () {
	assert(text("\\d+", "x١٢٣!") == "١٢٣")
	Nil
})

test("not_digit", () {
	assert(text("\\D+", "12abc3") == "abc")
	Nil
})

test("word", () {
	assert(text("\\w+", "--café_2!") == "café_2")
	Nil
})

test("word_unicode", () {
	assert(text("\\w+", "--汉字!") == "汉字")
	Nil
})

test("not_word", () {
	assert(text("\\W+", "word !ok") == " !")
	Nil
})

test("space", () {
	assert(text("\\s+", "a\t \nb") == "\t \n")
	Nil
})

test("space_unicode", () {
	assert(text("\\s+", "a  b") == "  ")
	Nil
})

test("not_space", () {
	assert(text("\\S+", " \na! ") == "a!")
	Nil
})

test("boundary", () {
	assert(text("\\bcat\\b", "scat cat!") == "cat")
	Nil
})

test("not_boundary", () {
	assert(text("\\Bcat", "scat cat") == "cat")
	Nil
})

test("unicode_boundary", () {
	assert(text("\\bélan\\b", "élan!") == "élan")
	Nil
})

test("greedy_star", () {
	assert(text("a*", "aaab") == "aaa")
	Nil
})

test("lazy_star", () {
	assert(text("a*?", "aaab") == "")
	Nil
})

test("greedy_plus", () {
	assert(text("a+", "baaa") == "aaa")
	Nil
})

test("lazy_plus", () {
	assert(text("a+?", "baaa") == "a")
	Nil
})

test("greedy_optional", () {
	assert(text("a?", "a") == "a")
	Nil
})

test("lazy_optional", () {
	assert(text("a??", "a") == "")
	Nil
})

test("exact", () {
	assert(text("a{3}", "aaaa") == "aaa")
	Nil
})

test("bounded", () {
	assert(text("a{2,4}", "aaaaa") == "aaaa")
	Nil
})

test("bounded_lazy", () {
	assert(text("a{2,4}?", "aaaaa") == "aa")
	Nil
})

test("open_bound", () {
	assert(text("a{2,}", "aaaa") == "aaaa")
	Nil
})

test("zero", () {
	assert(text("a{0}", "bbb") == "")
	Nil
})

test("zero_optional", () {
	assert(text("a{0,2}", "aaa") == "aa")
	Nil
})

test("alternate_first", () {
	assert(text("a|ab", "ab") == "a")
	Nil
})

test("alternate_second", () {
	assert(text("ab|a", "ab") == "ab")
	Nil
})

test("leftmost", () {
	assert(text("b|a", "ab") == "a")
	Nil
})

test("group", () {
	assert(text("(ab)+", "ababx") == "abab")
	Nil
})

test("noncapture", () {
	assert(text("(?:ab)+", "ababx") == "abab")
	Nil
})

test("start", () {
	assert(text("^a", "ba") == "<no match>")
	Nil
})

test("end", () {
	assert(text("a$", "ab") == "<no match>")
	Nil
})

test("dollar_strict", () {
	assert(text("a$", "a\n") == "<no match>")
	Nil
})

test("multiline", () {
	assert(text("(?m)^a$", "b\na\nc") == "a")
	Nil
})

test("absolute_start", () {
	assert(text("\\Aa", "a\na") == "a")
	Nil
})

test("absolute_end", () {
	assert(text("a\\z", "ba") == "a")
	Nil
})

test("case", () {
	assert(text("(?i)abc", "ABC") == "ABC")
	Nil
})

test("case_unicode", () {
	assert(text("(?i)é", "É") == "É")
	Nil
})

test("case_kelvin", () {
	assert(text("(?i)k", "K") == "K")
	Nil
})

test("case_longs", () {
	assert(text("(?i)[a-z]", "ſ") == "ſ")
	Nil
})

test("case_sigma", () {
	assert(text("(?i)σ", "ς") == "ς")
	Nil
})

test("case_negation", () {
	assert(text("(?i)[^a]+", "AaB") == "B")
	Nil
})

test("flag_scope", () {
	assert(text("((?i)a)b", "Ab") == "Ab")
	Nil
})

test("flag_restore", () {
	assert(text("((?i)a)b", "AB") == "<no match>")
	Nil
})

test("scoped_flags", () {
	assert(text("(?i:a)b", "Ab") == "Ab")
	Nil
})

test("unset_flag", () {
	assert(text("(?i)a(?-i)b", "Ab") == "Ab")
	Nil
})

test("class_escape", () {
	assert(text("[\\dA-F]+", "Z1AF!") == "1AF")
	Nil
})

test("class_negated_escape", () {
	assert(text("[^\\D]+", "a123b") == "123")
	Nil
})

test("escaped_dot", () {
	assert(text("\\.", "x.y") == ".")
	Nil
})

test("escaped_tab", () {
	assert(text("\\t+", "\t\tx") == "\t\t")
	Nil
})

test("hex", () {
	assert(text("\\x41+", "AAA") == "AAA")
	Nil
})

test("unicode_hex", () {
	assert(text("\\u{1f980}", "🦀") == "🦀")
	Nil
})

test("empty_alt", () {
	assert(text("|a", "a") == "")
	Nil
})

test("empty_group", () {
	assert(text("()", "a") == "")
	Nil
})

test("nested_empty", () {
	assert(text("(a*)*b", "aaab") == "aaab")
	Nil
})

test("greedy_follow", () {
	assert(text("a*ab", "aaaab") == "aaaab")
	Nil
})

test("lazy_follow", () {
	assert(text("a*?b", "aaab") == "aaab")
	Nil
})

test("branch_backtrack", () {
	assert(text("(ab|a)b", "ab") == "ab")
	Nil
})

test("class_dash", () {
	assert(text("[-a]+", "--aa") == "--aa")
	Nil
})

test("class_end_dash", () {
	assert(text("[a-]+", "aa--") == "aa--")
	Nil
})

test("astral_range", () {
	assert(text("[😀-🙏]+", "x😀🙏!") == "😀🙏")
	Nil
})

test("error_lookahead", () {
	regex.compile("(?=a)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_negative_lookahead", () {
	regex.compile("(?!a)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_lookbehind", () {
	regex.compile("(?<=a)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_negative_lookbehind", () {
	regex.compile("(?<!a)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_backreference", () {
	regex.compile("(a)\\1") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_named_backreference", () {
	regex.compile("\\k<a>") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_unclosed_group", () {
	regex.compile("(a") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_unclosed_class", () {
	regex.compile("[a") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_reverse_range", () {
	regex.compile("[z-a]") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_empty_class", () {
	regex.compile("[]") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_unknown_escape", () {
	regex.compile("\\q") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_trailing_escape", () {
	regex.compile("a\\") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_repeated_quantifier", () {
	regex.compile("a**") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_leading_quantifier", () {
	regex.compile("*a") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_reversed_bounds", () {
	regex.compile("a{3,2}") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_large_count", () {
	regex.compile("a{1001}") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_duplicate_name", () {
	regex.compile("(?<x>a)(?<x>b)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_invalid_name", () {
	regex.compile("(?<1x>a)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_empty_name", () {
	regex.compile("(?<>a)") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_unclosed_count", () {
	regex.compile("a{2") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_surrogate", () {
	regex.compile("\\x{d800}") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_large_scalar", () {
	regex.compile("\\u{110000}") |> {
		Error(regex.RegexError(position, reason)) -> {
			assert(position >= 0)
			assert(reason != "")
			Nil
		}
		Ok(_) -> panic("expected compile error")
	}
	Nil
})

test("error_position", () {
	assert(
		regex.compile("a[") ==
			Error(regex.RegexError(2, "unclosed character class"))
	)
	Nil
})

test("capture_index_name", () {
	c = regex.captures(compile("(?<first>a+)(b)?"), "xaa") |> {
		Some(c) -> c
		None    -> panic("capture")
	}
	assert(regex.group(c, 0) == Some(regex.Match(1, 3, "aa")))
	assert(regex.group(c, 1) == Some(regex.Match(1, 3, "aa")))
	assert(regex.named(c, "first") == regex.group(c, 1))
	assert(regex.group(c, 2) == None)
	assert(regex.group(c, 3) == None)
	assert(regex.group(c, -1) == None)
	assert(regex.named(c, "unknown") == None)
	Nil
})

test("capture_last_iteration", () {
	c = regex.captures(compile("(a|b)+"), "ab") |> {
		Some(c) -> c
		None    -> panic("capture")
	}
	assert(regex.group(c, 1) == Some(regex.Match(1, 2, "b")))
	Nil
})

test("capture_alternation", () {
	c = regex.captures(compile("(a)|(b)"), "b") |> {
		Some(c) -> c
		None    -> panic("capture")
	}
	assert(regex.group(c, 1) == None)
	assert(regex.group(c, 2) == Some(regex.Match(0, 1, "b")))
	Nil
})

test("unicode_scalar_offsets", () {
	assert(regex.find(compile("🦀é"), "中🦀é!") == Some(regex.Match(1, 3, "🦀é")))
	Nil
})

test("find_all", () {
	assert(
		regex.find_all(compile("a+"), "aa ba") ==
			[regex.Match(0, 2, "aa"), regex.Match(4, 5, "a")]
	)
	Nil
})

test("find_all_empty", () {
	assert(
		regex.find_all(compile(""), "é🦀") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
			]
	)
	Nil
})

test("find_all_suppress_adjacent_empty", () {
	assert(
		regex.find_all(compile("a*"), "ab") ==
			[regex.Match(0, 1, "a"), regex.Match(2, 2, "")]
	)
	Nil
})

test("find_all_lazy", () {
	assert(list.length(regex.find_all(compile("a*?"), "aa")) == 3)
	Nil
})

test("replace_first", () {
	assert(
		regex.replace(compile("(?<x>a)(b)"), "ab ab", "${x}-$2-$0-$$") ==
			"a-b-ab-$ ab"
	)
	Nil
})

test("replace_all", () {
	assert(regex.replace_all(compile("(a)"), "a a", "<$1>") == "<a> <a>")
	Nil
})

test("replace_missing", () {
	assert(regex.replace(compile("(a)?b"), "b", "$1/$2/${missing}") == "//")
	Nil
})

test("replace_braces", () {
	assert(regex.replace(compile("(a)"), "a", "${1}x") == "ax")
	Nil
})

test("replace_longest_name", () {
	assert(regex.replace(compile("(a)"), "a", "$1x") == "")
	Nil
})

test("replace_empty", () {
	assert(regex.replace_all(compile(""), "é🦀", "-") == "-é-🦀-")
	Nil
})

test("replace_literal_dollar", () {
	assert(regex.replace(compile("a"), "a", "${x $!") == "${x $!")
	Nil
})

test("replace_absent", () {
	assert(regex.replace_all(compile("z"), "abc", "x") == "abc")
	Nil
})

test("split", () {
	assert(regex.split(compile("[,;]"), ",a;b,") == ["", "a", "b", ""])
	Nil
})

test("split_empty", () {
	assert(regex.split(compile(""), "é🦀") == ["", "é", "🦀", ""])
	Nil
})

test("split_absent", () {
	assert(regex.split(compile("z"), "abc") == ["abc"])
	Nil
})

test("empty_input", () {
	assert(regex.is_match(compile(""), ""))
	assert(regex.find(compile("a"), "") == None)
	Nil
})

test("catastrophic_5000", () {
	assert(regex.is_match(compile("(a*)*b"), string.repeat("a", 5000)) == False)
	Nil
})

// Rust regex testdata/empty.toml, 100.
test("rust_empty_100", () {
	assert(
		regex.find_all(compile("|b"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 110.
test("rust_empty_110", () {
	assert(
		regex.find_all(compile("b|"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 2, "b")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 120.
test("rust_empty_120", () {
	assert(
		regex.find_all(compile("|z"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 130.
test("rust_empty_130", () {
	assert(
		regex.find_all(compile("z|"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 200.
test("rust_empty_200", () {
	assert(
		regex.find_all(compile("|"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 210.
test("rust_empty_210", () {
	assert(
		regex.find_all(compile("||"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 220.
test("rust_empty_220", () {
	assert(
		regex.find_all(compile("||b"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 230.
test("rust_empty_230", () {
	assert(
		regex.find_all(compile("b||"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 2, "b")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 240.
test("rust_empty_240", () {
	assert(
		regex.find_all(compile("||z"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 300.
test("rust_empty_300", () {
	assert(
		regex.find_all(compile("(?:)|b"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 310.
test("rust_empty_310", () {
	assert(
		regex.find_all(compile("b|(?:)"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 2, "b")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 320.
test("rust_empty_320", () {
	assert(
		regex.find_all(compile("(?:|)"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 330.
test("rust_empty_330", () {
	assert(
		regex.find_all(compile("(?:|)|z"), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 400.
test("rust_empty_400", () {
	assert(
		regex.find_all(compile("a(?:)|b"), "abc") ==
			[regex.Match(0, 1, "a"), regex.Match(1, 2, "b")]
	)
	Nil
})

// Rust regex testdata/empty.toml, 500.
test("rust_empty_500", () {
	assert(regex.find_all(compile(""), "") == [regex.Match(0, 0, "")])
	Nil
})

// Rust regex testdata/empty.toml, 510.
test("rust_empty_510", () {
	assert(
		regex.find_all(compile(""), "a") ==
			[regex.Match(0, 0, ""), regex.Match(1, 1, "")]
	)
	Nil
})

// Rust regex testdata/empty.toml, 520.
test("rust_empty_520", () {
	assert(
		regex.find_all(compile(""), "abc") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 600.
test("rust_empty_600", () {
	assert(
		regex.find_all(compile("(?:|a)*"), "aaa") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})

// Rust regex testdata/empty.toml, 610.
test("rust_empty_610", () {
	assert(
		regex.find_all(compile("(?:|a)+"), "aaa") ==
			[
				regex.Match(0, 0, "")
				regex.Match(1, 1, "")
				regex.Match(2, 2, "")
				regex.Match(3, 3, "")
			]
	)
	Nil
})
