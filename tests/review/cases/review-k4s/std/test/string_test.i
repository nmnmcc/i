// syntax: syn1
(
	i/core
	dict = std/dict
	result = std/result
	int = std/int
	list = std/list
	string = std/string
	order = (Less, Equal, Greater) = std/order
	option = (Some, None) = std/option
)

@test
length_unicode_test = () {
	// Codepoints, not graphemes: the last character is `e` and a combining ring.
	assert(string.length("ß↑e̊") == 4)
	Nil
}

@test
length_basic_test = () {
	assert(string.length("Gleam") == 5)
	Nil
}

@test
length_empty_test = () {
	assert(string.length("") == 0)
	Nil
}

@test
to_lower_test = () {
	assert(string.to_lower("Gleam") == "gleam")
	Nil
}

@test
to_upper_test = () {
	assert(string.to_upper("Gleam") == "GLEAM")
	Nil
}

@test
reverse_basic_test = () {
	assert(string.reverse("Gleam") == "maelG")
	Nil
}

@test
reverse_with_space_test = () {
	assert(string.reverse(" Gleam") == "maelG ")
	Nil
}

@test
reverse_emoji_test = () {
	assert(string.reverse("👍 OK") == "KO 👍")
	Nil
}

@test
reverse_single_emoji_test = () {
	assert(string.reverse("👍") == "👍")
	Nil
}

@test
reverse_unicode_test = () {
	assert(string.reverse("ÅÄÖ") == "ÖÄÅ")
	Nil
}

@test
reverse_skin_tone_emoji_test = () {
	// Codepoints, not graphemes: the skin tone comes first.
	assert(string.reverse("👶🏿") == "\u{1F3FF}\u{1F476}")
	Nil
}

@test
reverse_skin_tone_roundtrip_test = () {
	assert("👶🏿" |> string.reverse |> string.reverse == "👶🏿")
	Nil
}

@test
split_comma_test = () {
	assert(
		string.split("Gleam,Erlang,Elixir", ",") ==
			["Gleam", "Erlang", "Elixir"]
	)
	Nil
}

@test
split_comma_space_test = () {
	assert(
		string.split("Gleam, Erlang,Elixir", ", ") == ["Gleam", "Erlang,Elixir"]
	)
	Nil
}

@test
split_empty_test = () {
	assert(
		string.split("Gleam On Beam", "") ==
			["G", "l", "e", "a", "m", " ", "O", "n", " ", "B", "e", "a", "m"]
	)
	Nil
}

@test
split_once_ok_test = () {
	assert(
		string.split_once("Gleam,Erlang,Elixir", ",") ==
			Some(("Gleam", "Erlang,Elixir"))
	)
	Nil
}

@test
split_once_not_found_test = () {
	assert(string.split_once("Gleam", ",") == None)
	Nil
}

@test
split_once_empty_test = () {
	assert(string.split_once("", ",") == None)
	Nil
}

@test
replace_test = () {
	assert(
		string.replace("Gleam,Erlang,Elixir", ",", "++") ==
			"Gleam++Erlang++Elixir"
	)
	Nil
}

@test
append_test = () {
	assert(string.append("Test", " Me") == "Test Me")
	Nil
}

@test
compare_equal_test = () {
	assert(string.compare("", "") == order.Equal)
	Nil
}

@test
compare_a_gt_empty_test = () {
	assert(string.compare("a", "") == order.Greater)
	Nil
}

@test
compare_lowercase_a_gt_uppercase_a_test = () {
	assert(string.compare("a", "A") == order.Greater)
	Nil
}

@test
compare_uppercase_a_lt_uppercase_b_test = () {
	assert(string.compare("A", "B") == order.Less)
	Nil
}

@test
compare_t_gt_uppercase_abc_test = () {
	assert(string.compare("t", "ABC") == order.Greater)
	Nil
}

@test
contains_present_test = () {
	assert(string.contains("gleam", "ea"))
	Nil
}

@test
contains_absent_test = () {
	assert(core.not(string.contains("gleam", "x")))
	Nil
}

@test
contains_labelled_test = () {
	assert(string.contains(does: "bellwether", contain: "bell"))
	Nil
}

@test
concat_test = () {
	assert(string.concat(["Hello", ", ", "world!"]) == "Hello, world!")
	Nil
}

@test
concat_emoji_test = () {
	assert(
		string.concat(["💃🏿", "💇🏼‍♀️", "🧔‍♂️", "🧑‍🦼‍➡️"]) == "💃🏿💇🏼‍♀️🧔‍♂️🧑‍🦼‍➡️"
	)
	Nil
}

@test
repeat_one_test = () {
	assert(string.repeat("hi", times: 1) == "hi")
	Nil
}

@test
repeat_two_test = () {
	assert(string.repeat("hi", times: 2) == "hihi")
	Nil
}

@test
repeat_three_test = () {
	assert(string.repeat("hi", times: 3) == "hihihi")
	Nil
}

@test
repeat_large_test = () {
	assert(string.repeat("a", times: 10_001) |> string.length == 10_001)
	Nil
}

@test
repeat_zero_test = () {
	assert(string.repeat("hi", 0) == "")
	Nil
}

@test
repeat_negative_test = () {
	assert(string.repeat("hi", -1) == "")
	Nil
}

@test
join_0_test = () {
	assert(string.join([], with: ", ") == "")
	Nil
}

@test
join_0_dash_test = () {
	assert(string.join([], with: "-") == "")
	Nil
}

@test
join_1_test = () {
	assert(string.join(["Hello"], with: ", ") == "Hello")
	Nil
}

@test
join_1_dash_test = () {
	assert(string.join(["Hello"], with: "-") == "Hello")
	Nil
}

@test
join_2_test = () {
	assert(string.join(["Hello", "world!"], with: ", ") == "Hello, world!")
	Nil
}

@test
join_2_dash_test = () {
	assert(string.join(["Hello", "world!"], with: "-") == "Hello-world!")
	Nil
}

@test
join_3_test = () {
	assert(
		string.join(["Hello", "there", "world!"], with: ", ") ==
			"Hello, there, world!"
	)
	Nil
}

@test
join_3_dash_test = () {
	assert(
		string.join(["Hello", "there", "world!"], with: "-") ==
			"Hello-there-world!"
	)
	Nil
}

@test
trim_test = () {
	assert(string.trim("  hats  \n") == "hats")
	Nil
}

@test
trim2_test = () {
	assert(string.trim("k\r1=v2") == "k\r1=v2")
	Nil
}

@test
trim3_test = () {
	assert(string.trim("  \nhello\nworld\n  ") == "hello\nworld")
	Nil
}

@test
trim_start_test = () {
	assert(string.trim_start("  hats  \n") == "hats  \n")
	Nil
}

@test
trim_end_test = () {
	assert(string.trim_end("  hats  \n") == "  hats")
	Nil
}

@test
trim_whole_string_start_test = () {
	s = "\u{0020}\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0085}\u{2028}\u{2029}"

	assert(string.trim_start(s) == "")
	Nil
}

@test
trim_whole_string_end_test = () {
	s = "\u{0020}\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0085}\u{2028}\u{2029}"

	assert(string.trim_end(s) == "")
	Nil
}

@test
trim_whole_string_test = () {
	s = "\u{0020}\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0085}\u{2028}\u{2029}"

	assert(string.trim(s) == "")
	Nil
}

// unicode whitespaces
@test
trim_horizontal_tab_test = () {
	assert(string.trim("hats\u{0009}") == "hats")
	Nil
}

@test
trim_newline_test = () {
	assert(string.trim("hats\u{000A}") == "hats")
	Nil
}

@test
trim_vertical_tab_test = () {
	assert(string.trim("hats\u{000B}") == "hats")
	Nil
}

@test
trim_form_feed_test = () {
	assert(string.trim("hats\u{000C}") == "hats")
	Nil
}

@test
trim_carriage_return_test = () {
	assert(string.trim("hats\u{000D}") == "hats")
	Nil
}

@test
trim_space_test = () {
	assert(string.trim("hats\u{0020}") == "hats")
	Nil
}

@test
trim_no_break_space_test = () {
	assert(string.trim("hats\u{00A0}") == "hats\u{00A0}")
	Nil
}

@test
trim_ogham_space_mark_test = () {
	assert(string.trim("hats\u{1680}") == "hats\u{1680}")
	Nil
}

@test
trim_en_quad_test = () {
	assert(string.trim("hats\u{2000}") == "hats\u{2000}")
	Nil
}

@test
trim_em_quad_test = () {
	assert(string.trim("hats\u{2001}") == "hats\u{2001}")
	Nil
}

@test
trim_en_space_test = () {
	assert(string.trim("hats\u{2002}") == "hats\u{2002}")
	Nil
}

@test
trim_em_space_test = () {
	assert(string.trim("hats\u{2003}") == "hats\u{2003}")
	Nil
}

@test
trim_three_per_em_space_test = () {
	assert(string.trim("hats\u{2004}") == "hats\u{2004}")
	Nil
}

@test
trim_four_per_em_space_test = () {
	assert(string.trim("hats\u{2005}") == "hats\u{2005}")
	Nil
}

@test
trim_six_per_em_space_test = () {
	assert(string.trim("hats\u{2006}") == "hats\u{2006}")
	Nil
}

@test
trim_figure_space_test = () {
	assert(string.trim("hats\u{2007}") == "hats\u{2007}")
	Nil
}

@test
trim_punctuation_space_test = () {
	assert(string.trim("hats\u{2008}") == "hats\u{2008}")
	Nil
}

@test
trim_thin_space_test = () {
	assert(string.trim("hats\u{2009}") == "hats\u{2009}")
	Nil
}

@test
trim_hair_space_test = () {
	assert(string.trim("hats\u{200A}") == "hats\u{200A}")
	Nil
}

@test
trim_line_separator_test = () {
	assert(string.trim("hats\u{2028}") == "hats")
	Nil
}

@test
trim_paragraph_separator_test = () {
	assert(string.trim("hats\u{2029}") == "hats")
	Nil
}

@test
trim_narrow_no_break_space_test = () {
	assert(string.trim("hats\u{202F}") == "hats\u{202F}")
	Nil
}

@test
trim_medium_mathematical_space_test = () {
	assert(string.trim("hats\u{205F}") == "hats\u{205F}")
	Nil
}

@test
trim_ideographic_space_test = () {
	assert(string.trim("hats\u{3000}") == "hats\u{3000}")
	Nil
}

// related unicode non-whitespaces
@test
trim_mongolian_vowel_separator_test = () {
	assert(string.trim("hats\u{180E}") == "hats\u{180E}")
	Nil
}

@test
trim_zero_width_space_test = () {
	assert(string.trim("hats\u{200B}") == "hats\u{200B}")
	Nil
}

@test
trim_zero_width_non_joiner_test = () {
	assert(string.trim("hats\u{200C}") == "hats\u{200C}")
	Nil
}

@test
trim_zero_width_joiner_test = () {
	assert(string.trim("hats\u{200D}") == "hats\u{200D}")
	Nil
}

@test
trim_word_joiner_test = () {
	assert(string.trim("hats\u{2060}") == "hats\u{2060}")
	Nil
}

@test
trim_zero_width_non_breaking_space_test = () {
	assert(string.trim("hats\u{FEFF}") == "hats\u{FEFF}")
	Nil
}

@test
trim_comma_test = () {
	assert(string.trim("hats,") == "hats,")
	Nil
}

@test
starts_with_empty_test = () {
	assert(string.starts_with("theory", ""))
	Nil
}

@test
starts_with_match_test = () {
	assert(string.starts_with("theory", "the"))
	Nil
}

@test
starts_with_no_match_test = () {
	assert(core.not(string.starts_with("theory", "ory")))
	Nil
}

@test
starts_with_longer_test = () {
	assert(core.not(string.starts_with("theory", "theory2")))
	Nil
}

@test
ends_with_empty_test = () {
	assert(string.ends_with("theory", ""))
	Nil
}

@test
ends_with_match_test = () {
	assert(string.ends_with("theory", "ory"))
	Nil
}

@test
ends_with_no_match_test = () {
	assert(core.not(string.ends_with("theory", "the")))
	Nil
}

@test
ends_with_longer_test = () {
	assert(core.not(string.ends_with("theory", "theory2")))
	Nil
}

@test
slice_basic_test = () {
	assert(string.slice("gleam", at_index: 1, length: 2) == "le")
	Nil
}

@test
slice_beyond_length_test = () {
	assert(string.slice("gleam", at_index: 1, length: 10) == "leam")
	Nil
}

@test
slice_beyond_string_test = () {
	assert(string.slice("gleam", at_index: 10, length: 3) == "")
	Nil
}

@test
slice_negative_index_test = () {
	assert(string.slice("gleam", at_index: -2, length: 2) == "am")
	Nil
}

@test
slice_negative_beyond_test = () {
	assert(string.slice("gleam", at_index: -12, length: 2) == "")
	Nil
}

@test
slice_negative_length_test = () {
	assert(string.slice("gleam", at_index: 2, length: -3) == "")
	Nil
}

@test
slice_zero_length_test = () {
	assert(string.slice("gleam", at_index: 2, length: 0) == "")
	Nil
}

@test
slice_emoji_test = () {
	assert(string.slice("👶🏿", at_index: 0, length: 3) == "👶🏿")
	Nil
}

@test
crop_start_test = () {
	assert(string.crop("gleam", "gl") == "gleam")
	Nil
}

@test
crop_middle_test = () {
	assert(string.crop("gleam", "le") == "leam")
	Nil
}

@test
crop_labelled_test = () {
	assert(string.crop(from: "gleam", before: "ea") == "eam")
	Nil
}

@test
crop_empty_test = () {
	assert(string.crop("gleam", "") == "gleam")
	Nil
}

@test
crop_not_found_test = () {
	assert(string.crop("gleam", "!") == "gleam")
	Nil
}

@test
drop_start_basic_test = () {
	assert(string.drop_start("gleam", up_to: 2) == "eam")
	Nil
}

@test
drop_start_beyond_length_test = () {
	assert(string.drop_start("gleam", up_to: 6) == "")
	Nil
}

@test
drop_start_negative_test = () {
	assert(string.drop_start("gleam", up_to: -2) == "gleam")
	Nil
}

@test
drop_start_zero_test = () {
	assert(string.drop_start("gleam", up_to: 0) == "gleam")
	Nil
}

@test
drop_start_3499_test = () {
	// https://github.com/gleam-lang/gleam/issues/3499
	assert(string.drop_start("\r]", 1) == "]")
	Nil
}

@test
drop_end_basic_test = () {
	assert(string.drop_end("gleam", up_to: 2) == "gle")
	Nil
}

@test
drop_end_all_test = () {
	assert(string.drop_end("gleam", up_to: 5) == "")
	Nil
}

@test
drop_end_negative_test = () {
	assert(string.drop_end("gleam", up_to: -2) == "gleam")
	Nil
}

@test
drop_end_zero_test = () {
	assert(string.drop_end("gleam", up_to: 0) == "gleam")
	Nil
}

@test
pad_left_basic_test = () {
	assert(string.pad_left("121", to: 5, with: ".") == "..121")
	Nil
}

@test
pad_left_exact_test = () {
	assert(string.pad_left("121", to: 3, with: ".") == "121")
	Nil
}

@test
pad_left_shorter_test = () {
	assert(string.pad_left("121", to: 2, with: ".") == "121")
	Nil
}

@test
pad_left_partial_multi_test = () {
	assert(string.pad_left("121", to: 4, with: "XY") == "X121")
	Nil
}

@test
pad_left_exact_multi_test = () {
	assert(string.pad_left("121", to: 5, with: "XY") == "XY121")
	Nil
}

@test
pad_left_extra_multi_test = () {
	assert(string.pad_left("121", to: 6, with: "XY") == "XYX121")
	Nil
}

@test
pad_right_basic_test = () {
	assert(string.pad_right("121", to: 5, with: ".") == "121..")
	Nil
}

@test
pad_right_exact_test = () {
	assert(string.pad_right("121", to: 3, with: ".") == "121")
	Nil
}

@test
pad_right_shorter_test = () {
	assert(string.pad_right("121", to: 2, with: ".") == "121")
	Nil
}

@test
pad_right_partial_multi_test = () {
	assert(string.pad_right("121", to: 4, with: "XY") == "121X")
	Nil
}

@test
pad_right_exact_multi_test = () {
	assert(string.pad_right("121", to: 5, with: "XY") == "121XY")
	Nil
}

@test
pad_right_extra_multi_test = () {
	assert(string.pad_right("121", to: 6, with: "XY") == "121XYX")
	Nil
}

@test
pop_codepoint_ok_test = () {
	assert(string.pop_codepoint("gleam") == Some(("g", "leam")))
	Nil
}

@test
pop_codepoint_single_test = () {
	assert(string.pop_codepoint("g") == Some(("g", "")))
	Nil
}

@test
pop_codepoint_empty_test = () {
	assert(string.pop_codepoint("") == None)
	Nil
}

@test
to_codepoint_strings_empty_test = () {
	assert(string.to_codepoint_strings("") == [])
	Nil
}

@test
to_codepoint_strings_escape_chars_test = () {
	assert(
		string.to_codepoint_strings("\n\t\r\"\\") ==
			["\n", "\t", "\r", "\"", "\\"]
	)
	Nil
}

@test
to_codepoint_strings_single_test = () {
	assert(string.to_codepoint_strings("a") == ["a"])
	Nil
}

@test
to_codepoint_strings_basic_test = () {
	assert(string.to_codepoint_strings("abc") == ["a", "b", "c"])
	Nil
}

@test
to_codepoint_strings_korean_test = () {
	assert(string.to_codepoint_strings("뎌쉐") == ["뎌", "쉐"])
	Nil
}

@test
to_codepoint_strings_katakana_test = () {
	assert(string.to_codepoint_strings("パピプペポ") == ["パ", "ピ", "プ", "ペ", "ポ"])
	Nil
}

@test
to_utf_codepoints_empty_test = () {
	assert(string.to_codepoints("") == [])
	Nil
}

@test
to_utf_codepoints_basic_test = () {
	assert(string.to_codepoints("gleam") == {
		(Ok(g), Ok(l), Ok(e), Ok(a), Ok(m)) = (
			string.utf_codepoint(103)
			string.utf_codepoint(108)
			string.utf_codepoint(101)
			string.utf_codepoint(97)
			string.utf_codepoint(109)
		)
		[g, l, e, a, m]
	})
	Nil
}

@test
to_utf_codepoints_rainbow_flag_test = () {
	// ["🏳", "️", "‍", "🌈"]
	expected = {
		(
			Ok(waving_white_flag)
			Ok(variant_selector_16)
			Ok(zero_width_joiner)
			Ok(rainbow)
		) = (
			string.utf_codepoint(127_987)
			string.utf_codepoint(65_039)
			string.utf_codepoint(8205)
			string.utf_codepoint(127_752)
		)
		[waving_white_flag, variant_selector_16, zero_width_joiner, rainbow]
	}

	assert(string.to_codepoints("🏳️‍🌈") == expected)
	Nil
}

@test
from_utf_codepoints_empty_test = () {
	assert("" |> string.to_codepoints |> string.from_codepoints == "")
	Nil
}

@test
from_utf_codepoints_basic_test = () {
	assert("gleam" |> string.to_codepoints |> string.from_codepoints == "gleam")
	Nil
}

@test
from_utf_codepoints_emoji_test = () {
	assert("🏳️‍🌈" |> string.to_codepoints |> string.from_codepoints == "🏳️‍🌈")
	Nil
}

@test
from_utf_codepoints_abc_test = () {
	assert(string.from_codepoints({
		(Ok(a), Ok(b), Ok(c)) = (
			string.utf_codepoint(97)
			string.utf_codepoint(98)
			string.utf_codepoint(99)
		)
		[a, b, c]
	}) == "abc")
	Nil
}

@test
utf_codepoint_negative_test = () {
	// Less than the lower bound on valid codepoints
	Error(_) = string.utf_codepoint(-1)
	Nil
}

@test
utf_codepoint_zero_test = () {
	// The lower bound on valid codepoints
	Ok(_) = string.utf_codepoint(0)
	Nil
}

@test
utf_codepoint_max_test = () {
	// The upper bound for valid code points
	Ok(_) = string.utf_codepoint(1_114_111)
	Nil
}

@test
utf_codepoint_above_max_test = () {
	// Greater than the upper bound on valid codepoints
	Error(_) = string.utf_codepoint(1_114_112)
	Nil
}

@test
utf_codepoint_nonchar_fffe_test = () {
	// Non-characters U+FFFE and U+FFFF are valid codepoints.  See (#778).
	Ok(_) = string.utf_codepoint(65_534)
	Nil
}

@test
utf_codepoint_nonchar_ffff_test = () {
	Ok(_) = string.utf_codepoint(65_535)
	Nil
}

@test
utf_codepoint_below_high_surrogate_test = () {
	// One less than the lowest "High-surrogate code point"
	Ok(_) = string.utf_codepoint(55_295)
	Nil
}

@test
utf_codepoint_high_surrogate_test = () {
	// Lowest value of the "High-surrogate code point" (U+D800 to U+DBFF)
	Error(_) = string.utf_codepoint(55_296)
	Nil
}

@test
utf_codepoint_low_surrogate_test = () {
	// Highest value of the "Low-surrogate code point" (U+DC00 to U+DFFF)
	Error(_) = string.utf_codepoint(57_343)
	Nil
}

@test
utf_codepoint_above_low_surrogate_test = () {
	// One greater than the highest "Low-surrogate code point"
	Ok(_) = string.utf_codepoint(57_344)
	Nil
}

@test
bit_array_utf_codepoint_test = () {
	Ok(snake) = string.utf_codepoint(128_013)
	expected = <<"🐍":utf8>>
	assert(<<snake:utf8_codepoint>> == expected)
	Nil
}

@test
utf_codepoint_to_int_test = () {
	assert(string.utf_codepoint_to_int({
		Ok(ordinal_value) = string.utf_codepoint(128_013)
		ordinal_value
	}) == 128_013)
	Nil
}

@test
to_option_empty_test = () {
	assert(string.to_option("") == None)
	Nil
}

@test
to_option_non_empty_test = () {
	assert(string.to_option("ok") == Some("ok"))
	Nil
}

@test
first_empty_test = () {
	None = string.first("")
	Nil
}

@test
first_basic_test = () {
	assert(string.first("gleam") == Some("g"))
	Nil
}

@test
first_emoji_test = () {
	// The first codepoint, without the variation selector.
	assert(string.first("⭐️ Gleam") == Some("\u{2B50}"))
	Nil
}

@test
first_single_test = () {
	assert(string.first("a") == Some("a"))
	Nil
}

@test
last_empty_test = () {
	None = string.last("")
	Nil
}

@test
last_basic_test = () {
	assert(string.last("gleam") == Some("m"))
	Nil
}

@test
last_with_space_test = () {
	assert(string.last("gleam ") == Some(" "))
	Nil
}

@test
last_unicode_test = () {
	assert(string.last("եdelays") == Some("s"))
	Nil
}

@test
last_single_test = () {
	assert(string.last("a") == Some("a"))
	Nil
}

@test
capitalise_empty_test = () {
	assert(string.capitalise("") == "")
	Nil
}

@test
capitalise_lowercase_test = () {
	assert(string.capitalise("gleam") == "Gleam")
	Nil
}

@test
capitalise_uppercase_test = () {
	assert(string.capitalise("GLEAM") == "Gleam")
	Nil
}

@test
capitalise_with_spaces_test = () {
	assert(string.capitalise("g l e a m") == "G l e a m")
	Nil
}

@test
capitalise_leading_number_test = () {
	assert(string.capitalise("1GLEAM") == "1gleam")
	Nil
}

@test
capitalise_leading_underscore_test = () {
	assert(string.capitalise("_gLeAm1") == "_gleam1")
	Nil
}

@test
capitalise_leading_space_test = () {
	assert(string.capitalise(" gLeAm1") == " gleam1")
	Nil
}

@test
capitalise_unicode_test = () {
	assert(string.capitalise("る") == "る")
	Nil
}

InspectType(A, B) {
	InspectTypeZero
	InspectTypeOne(A)
	InspectTypeTwo(A, B)
}

@test
inspect_0_test = () {
	assert(string.inspect(True) == "True")
	Nil
}

@test
inspect_4_test = () {
	assert(string.inspect(False) == "False")
	Nil
}

@test
inspect_8_test = () {
	assert(string.inspect([True, False]) == "[True, False]")
	Nil
}

@test
inspect_12_test = () {
	assert(string.inspect([False, False]) == "[False, False]")
	Nil
}

@test
inspect_16_test = () {
	assert(string.inspect([True, True]) == "[True, True]")
	Nil
}

@test
inspect_20_test = () {
	assert(string.inspect([Nil, Nil]) == "[Nil, Nil]")
	Nil
}

@test
inspect_24_test = () {
	assert(string.inspect((True, False)) == "(True, False)")
	Nil
}

@test
inspect_28_test = () {
	assert(string.inspect((False, False)) == "(False, False)")
	Nil
}

@test
inspect_32_test = () {
	assert(string.inspect((True, True)) == "(True, True)")
	Nil
}

@test
inspect_36_test = () {
	assert(string.inspect((Nil, True)) == "(Nil, True)")
	Nil
}

@test
inspect_40_test = () {
	assert(string.inspect((Nil, False)) == "(Nil, False)")
	Nil
}

@test
inspect_44_test = () {
	assert(string.inspect((True, Nil)) == "(True, Nil)")
	Nil
}

@test
inspect_48_test = () {
	assert(string.inspect((False, Nil)) == "(False, Nil)")
	Nil
}

@test
inspect_52_test = () {
	assert(string.inspect(-1) == "-1")
	Nil
}

@test
inspect_56_test = () {
	assert(string.inspect(0) == "0")
	Nil
}

@test
inspect_60_test = () {
	assert(string.inspect(1) == "1")
	Nil
}

@test
inspect_64_test = () {
	assert(string.inspect([]) == "[]")
	Nil
}

@test
inspect_68_test = () {
	assert(string.inspect([1]) == "[1]")
	Nil
}

@test
inspect_72_test = () {
	assert(string.inspect([1, 2]) == "[1, 2]")
	Nil
}

@test
inspect_76_test = () {
	assert(string.inspect([[1], [1]]) == "[[1], [1]]")
	Nil
}

@test
inspect_80_test = () {
	assert(string.inspect(-.1.5) == "-1.5")
	Nil
}

@test
inspect_84_test = () {
	assert(string.inspect(5.0e-26) == "5.0e-26")
	Nil
}

@test
inspect_88_test = () {
	assert(string.inspect(1.5) == "1.5")
	Nil
}

@test
inspect_92_test = () {
	assert(string.inspect(-.5.0e-26) == "-5.0e-26")
	Nil
}

@test
inspect_96_test = () {
	assert(string.inspect([1.5]) == "[1.5]")
	Nil
}

@test
inspect_100_test = () {
	assert(string.inspect("") == "\"\"")
	Nil
}

@test
inspect_104_test = () {
	assert(string.inspect("\\") == "\"\\\\\"")
	Nil
}

@test
inspect_108_test = () {
	assert(string.inspect("\\\\") == "\"\\\\\\\\\"")
	Nil
}

@test
inspect_112_test = () {
	assert(string.inspect("\\\\\\") == "\"\\\\\\\\\\\\\"")
	Nil
}

@test
inspect_116_test = () {
	assert(string.inspect("\"") == "\"\\\"\"")
	Nil
}

@test
inspect_120_test = () {
	assert(string.inspect("\"\"") == "\"\\\"\\\"\"")
	Nil
}

@test
inspect_124_test = () {
	assert(string.inspect("\r") == "\"\\r\"")
	Nil
}

@test
inspect_128_test = () {
	assert(string.inspect("\n") == "\"\\n\"")
	Nil
}

@test
inspect_132_test = () {
	assert(string.inspect("\t") == "\"\\t\"")
	Nil
}

@test
inspect_136_test = () {
	assert(string.inspect("\f") == "\"\\f\"")
	Nil
}

@test
inspect_140_test = () {
	assert(string.inspect("\u{0008}") == "\"\\u{0008}\"")
	Nil
}

@test
inspect_144_test = () {
	assert(string.inspect("\u{000B}") == "\"\\u{000B}\"")
	Nil
}

@test
inspect_148_test = () {
	assert(string.inspect("\u{001B}") == "\"\\u{001B}\"")
	Nil
}

@test
inspect_152_test = () {
	assert(string.inspect("\u{0015}") == "\"\\u{0015}\"")
	Nil
}

@test
inspect_156_test = () {
	assert(string.inspect("\u{001F}") == "\"\\u{001F}\"")
	Nil
}

@test
inspect_160_test = () {
	assert(string.inspect("\u{0020}") == "\" \"")
	Nil
}

@test
inspect_164_test = () {
	assert(string.inspect("\u{007F}") == "\"\\u{007F}\"")
	Nil
}

@test
inspect_168_test = () {
	assert(string.inspect("\u{009F}") == "\"\\u{009F}\"")
	Nil
}

@test
inspect_172_test = () {
	assert(string.inspect("\u{00A0}") == "\"\u{00A0}\"")
	Nil
}

@test
inspect_176_test = () {
	assert(string.inspect("\r\r") == "\"\\r\\r\"")
	Nil
}

@test
inspect_180_test = () {
	assert(string.inspect("\n\n") == "\"\\n\\n\"")
	Nil
}

@test
inspect_184_test = () {
	assert(string.inspect("\r\n") == "\"\\r\\n\"")
	Nil
}

@test
inspect_188_test = () {
	assert(string.inspect("\n\r") == "\"\\n\\r\"")
	Nil
}

@test
inspect_192_test = () {
	assert(string.inspect("\t\t") == "\"\\t\\t\"")
	Nil
}

@test
inspect_196_test = () {
	assert(string.inspect("\t\n") == "\"\\t\\n\"")
	Nil
}

@test
inspect_200_test = () {
	assert(string.inspect("\n\t") == "\"\\n\\t\"")
	Nil
}

@test
inspect_204_test = () {
	assert(string.inspect("\t\r") == "\"\\t\\r\"")
	Nil
}

@test
inspect_208_test = () {
	assert(string.inspect("\r\t") == "\"\\r\\t\"")
	Nil
}

@test
inspect_212_test = () {
	assert(string.inspect("\t\f") == "\"\\t\\f\"")
	Nil
}

@test
inspect_216_test = () {
	assert(string.inspect("\f\t") == "\"\\f\\t\"")
	Nil
}

@test
inspect_220_test = () {
	assert(string.inspect("\t\u{0008}") == "\"\\t\\u{0008}\"")
	Nil
}

@test
inspect_224_test = () {
	assert(string.inspect("\u{0008}\t") == "\"\\u{0008}\\t\"")
	Nil
}

@test
inspect_228_test = () {
	assert(string.inspect("\t\u{000B}") == "\"\\t\\u{000B}\"")
	Nil
}

@test
inspect_232_test = () {
	assert(string.inspect("\u{000B}\t") == "\"\\u{000B}\\t\"")
	Nil
}

@test
inspect_236_test = () {
	assert(string.inspect("\t\u{001B}") == "\"\\t\\u{001B}\"")
	Nil
}

@test
inspect_240_test = () {
	assert(string.inspect("\u{001B}\t") == "\"\\u{001B}\\t\"")
	Nil
}

@test
inspect_244_test = () {
	assert(string.inspect("\\\n\\") == "\"\\\\\\n\\\\\"")
	Nil
}

@test
inspect_248_test = () {
	assert(string.inspect("\\\"\\") == "\"\\\\\\\"\\\\\"")
	Nil
}

@test
inspect_252_test = () {
	assert(string.inspect("\\\"\"\\") == "\"\\\\\\\"\\\"\\\\\"")
	Nil
}

@test
inspect_256_test = () {
	assert(string.inspect("'") == "\"'\"")
	Nil
}

@test
inspect_260_test = () {
	assert(string.inspect("''") == "\"''\"")
	Nil
}

@test
inspect_264_test = () {
	assert(
		string.inspect("around-single-quotes'around-single-quotes") ==
			"\"around-single-quotes'around-single-quotes\""
	)
	Nil
}

@test
inspect_269_test = () {
	assert(
		string.inspect("'between-single-quotes'") ==
			"\"'between-single-quotes'\""
	)
	Nil
}

@test
inspect_274_test = () {
	assert(string.inspect("0") == "\"0\"")
	Nil
}

@test
inspect_278_test = () {
	assert(string.inspect("1") == "\"1\"")
	Nil
}

@test
inspect_282_test = () {
	assert(string.inspect("2") == "\"2\"")
	Nil
}

@test
inspect_286_test = () {
	assert(string.inspect("Hello Joe!") == "\"Hello Joe!\"")
	Nil
}

@test
inspect_290_test = () {
	assert(string.inspect("Hello \"Manuel\"!") == "\"Hello \\\"Manuel\\\"!\"")
	Nil
}

@test
inspect_294_test = () {
	assert(string.inspect("👨‍👩‍👦‍👦 💜 Gleam") == "\"👨‍👩‍👦‍👦 💜 Gleam\"")
	Nil
}

@test
inspect_298_test = () {
	assert(string.inspect("✨") == "\"✨\"")
	Nil
}

@test
inspect_302_test = () {
	assert(string.inspect("🏳️‍⚧️") == "\"🏳️‍⚧️\"")
	Nil
}

@test
inspect_306_test = () {
	assert(string.inspect("True") == "\"True\"")
	Nil
}

@test
inspect_310_test = () {
	assert(string.inspect("False") == "\"False\"")
	Nil
}

@test
inspect_314_test = () {
	assert(string.inspect("Nil") == "\"Nil\"")
	Nil
}

@test
inspect_318_test = () {
	assert(string.inspect(["1"]) == "[\"1\"]")
	Nil
}

@test
inspect_322_test = () {
	assert(string.inspect(()) == "()")
	Nil
}

@test
inspect_326_test = () {
	assert(string.inspect((1)) == "(1)")
	Nil
}

@test
inspect_330_test = () {
	assert(string.inspect(("1")) == "(\"1\")")
	Nil
}

@test
inspect_334_test = () {
	assert(string.inspect((1.5)) == "(1.5)")
	Nil
}

@test
inspect_338_test = () {
	assert(string.inspect([(1, 2, 3), (1, 2, 3)]) == "[(1, 2, 3), (1, 2, 3)]")
	Nil
}

@test
inspect_342_test = () {
	assert(
		string.inspect(([1, 2, 3], "🌈", "🏳️‍🌈", (1, "1", True))) ==
			"([1, 2, 3], \"🌈\", \"🏳️‍🌈\", (1, \"1\", True))"
	)
	Nil
}

@test
inspect_347_test = () {
	assert(string.inspect(Nil) == "Nil")
	Nil
}

@test
inspect_351_test = () {
	assert(string.inspect(Ok(1)) == "Ok(1)")
	Nil
}

@test
inspect_355_test = () {
	assert(string.inspect(Ok(True)) == "Ok(True)")
	Nil
}

@test
inspect_359_test = () {
	assert(string.inspect(Ok(False)) == "Ok(False)")
	Nil
}

@test
inspect_363_test = () {
	assert(string.inspect(Ok(Nil)) == "Ok(Nil)")
	Nil
}

@test
inspect_367_test = () {
	assert(string.inspect(Error(2)) == "Error(2)")
	Nil
}

@test
inspect_371_test = () {
	assert(string.inspect(Error(True)) == "Error(True)")
	Nil
}

@test
inspect_375_test = () {
	assert(string.inspect(Error(False)) == "Error(False)")
	Nil
}

@test
inspect_379_test = () {
	assert(string.inspect(Error(Nil)) == "Error(Nil)")
	Nil
}

@test
inspect_383_test = () {
	assert(string.inspect(InspectTypeZero) == "InspectTypeZero")
	Nil
}

@test
inspect_387_test = () {
	assert(string.inspect(InspectTypeOne(1)) == "InspectTypeOne(1)")
	Nil
}

@test
inspect_391_test = () {
	assert(string.inspect(InspectTypeTwo(1, 2)) == "InspectTypeTwo(1, 2)")
	Nil
}

@test
inspect_395_test = () {
	assert(string.inspect(InspectTypeOne([1])) == "InspectTypeOne([1])")
	Nil
}

@test
inspect_399_test = () {
	assert(string.inspect(InspectTypeOne("1")) == "InspectTypeOne(\"1\")")
	Nil
}

@test
inspect_403_test = () {
	assert(string.inspect(InspectTypeOne(["1"])) == "InspectTypeOne([\"1\"])")
	Nil
}

@test
inspect_407_test = () {
	assert(
		string.inspect(InspectTypeOne(([1], "a"))) ==
			"InspectTypeOne(([1], \"a\"))"
	)
	Nil
}

@test
inspect_412_test = () {
	assert(string.inspect(Ok) == "//fn(a) { ... }")
	Nil
}

@test
inspect_416_test = () {
	assert(string.inspect(Error) == "//fn(a) { ... }")
	Nil
}

@test
inspect_420_test = () {
	assert(string.inspect(() { Nil }) == "//fn() { ... }")
	Nil
}

@test
inspect_424_test = () {
	assert(string.inspect((_) { Nil }) == "//fn(a) { ... }")
	Nil
}

@test
inspect_428_test = () {
	assert(string.inspect((_, _) { Nil }) == "//fn(a, b) { ... }")
	Nil
}

@test
inspect_432_test = () {
	assert(string.inspect((_, _) { Nil }) == "//fn(a, b) { ... }")
	Nil
}

@test
inspect_436_test = () {
	assert(
		string.inspect((_: Int, _: String) -> Bool { False }) ==
			"//fn(a, b) { ... }"
	)
	Nil
}

@test
inspect_441_test = () {
	assert(
		string.inspect((InspectTypeOne, InspectTypeTwo)) ==
			"(//fn(a) { ... }, //fn(a, b) { ... })"
	)
	Nil
}

@test
inspect_446_test = () {
	assert(
		string.inspect(InspectTypeOne(InspectTypeZero)) ==
			"InspectTypeOne(InspectTypeZero)"
	)
	Nil
}

@test
inspect_451_test = () {
	assert(string.inspect(<<255, 2, 0>>) == "<<255, 2, 0>>")
	Nil
}

@test
inspect_charlist_test = () {
	list = [
		70
		97
		105
		108
		101
		100
		32
		116
		111
		32
		108
		111
		97
		100
		32
		78
		73
		70
		32
		108
		105
		98
		114
		97
		114
		121
		58
		32
		39
		47
		114
		117
		110
		47
		99
		117
		114
		114
		101
		110
		116
		45
		115
		121
		115
		116
		101
		109
		47
		115
		119
		47
		115
		104
		97
		114
		101
		47
		110
		105
		120
		45
		108
		100
		47
		108
		105
		98
		47
		108
		105
		98
		99
		114
		121
		112
		116
		111
		46
		115
		111
		46
		51
	]
	// A Wasm List(Int) remains a list even when every value is printable.
	assert(
		string.inspect(list) ==
			"[70, 97, 105, 108, 101, 100, 32, 116, 111, 32, 108, 111, 97, 100, 32, 78, 73, 70, 32, 108, 105, 98, 114, 97, 114, 121, 58, 32, 39, 47, 114, 117, 110, 47, 99, 117, 114, 114, 101, 110, 116, 45, 115, 121, 115, 116, 101, 109, 47, 115, 119, 47, 115, 104, 97, 114, 101, 47, 110, 105, 120, 45, 108, 100, 47, 108, 105, 98, 47, 108, 105, 98, 99, 114, 121, 112, 116, 111, 46, 115, 111, 46, 51]"
	)
	Nil
}

// WasmGC carries distinct runtime tags for tuples, constructors, floats,
// strings and bit arrays; these assertions replace duplicated host branches.
@test
target_inspect_tuple_test = () {
	assert(
		string.inspect((InspectTypeZero, InspectTypeZero)) ==
			"(InspectTypeZero, InspectTypeZero)"
	)
	Nil
}

@test
target_inspect_float_negative_test = () {
	assert(string.inspect(-.1.0) == "-1.0")
	Nil
}

@test
target_inspect_float_zero_test = () {
	assert(string.inspect(0.0) == "0.0")
	Nil
}

@test
target_inspect_float_positive_test = () {
	assert(string.inspect(1.0) == "1.0")
	Nil
}

@test
target_inspect_float_list_test = () {
	assert(string.inspect([1.0]) == "[1.0]")
	Nil
}

@test
target_inspect_float_tuple_test = () {
	assert(string.inspect((1.0)) == "(1.0)")
	Nil
}

@test
target_inspect_bit_array_test = () {
	assert(string.inspect(<<"abc":utf8>>) == "<<97, 98, 99>>")
	Nil
}

@test
inspect_fifteen_bit_int_test = () {
	assert(string.inspect(<<2, 3:u7>>) == "<<2, 3:u7>>")
	Nil
}

@test
byte_size_empty_test = () {
	0 = string.byte_size("")
	Nil
}

@test
byte_size_one_test = () {
	1 = string.byte_size("a")
	Nil
}

@test
byte_size_two_test = () {
	2 = string.byte_size("ab")
	Nil
}

@test
byte_size_three_test = () {
	3 = string.byte_size("abc")
	Nil
}

@test
byte_size_unicode_a_test = () {
	// Unicode graphemes. These will be multiple bytes.
	1 = string.byte_size("a")
	Nil
}

@test
byte_size_unicode_umlaut_test = () {
	2 = string.byte_size("ä")
	Nil
}

@test
byte_size_unicode_emoji_test = () {
	4 = string.byte_size("👩")
	Nil
}

@test
byte_size_unicode_skin_tone_test = () {
	8 = string.byte_size("👩🏾")
	Nil
}

@test
byte_size_unicode_complex_test = () {
	15 = string.byte_size("👩🏾‍🦰")
	Nil
}

@test
remove_prefix_test = () {
	assert(string.remove_prefix("gleamlucy", "gleam") == "lucy")
	assert(string.remove_prefix("lucygleam", "gleam") == "lucygleam")
	assert(string.remove_prefix("lucygleam", "") == "lucygleam")
	assert(string.remove_prefix("", "gleam") == "")
	assert(string.remove_prefix("gleam", "gleam") == "")
	assert(string.remove_prefix("🦑lucy", "🦑") == "lucy")
	assert(string.remove_prefix("🦑🦑", "🦑") == "🦑")
	assert(string.remove_prefix("abc", "abcde") == "abc")
	assert(string.remove_prefix(from: "abc", matching: "ab") == "c")
	Nil
}

@test
remove_suffix_test = () {
	assert(string.remove_suffix("lucygleam", "gleam") == "lucy")
	assert(string.remove_suffix("gleamlucy", "gleam") == "gleamlucy")
	assert(string.remove_suffix("lucygleam", "") == "lucygleam")
	assert(string.remove_suffix("", "gleam") == "")
	assert(string.remove_suffix("gleam", "gleam") == "")
	assert(string.remove_suffix("lucy🦑", "🦑") == "lucy")
	assert(string.remove_suffix("🦑🦑", "🦑") == "🦑")
	assert(string.remove_suffix(from: "abc", matching: "bc") == "a")
	Nil
}

// Tests for functions that are new in std, or that changed from the old
// library.

@test
split_more_test = () {
	assert(string.split("a,b,,c", on: ",") == ["a", "b", "", "c"])
	assert(string.split("", on: ",") == [""])
	assert(string.split(",", on: ",") == ["", ""])
	assert(string.split("abc", on: "") == ["a", "b", "c"])
	assert(string.split("a--b---c", on: "--") == ["a", "b", "-c"])
	assert(string.split("héllo wörld", on: "ö") == ["héllo w", "rld"])
	assert(string.split("😀x😀", on: "😀") == ["", "x", ""])
}

@test
split_long_test = () {
	text = string.repeat("ab,", 100_000)
	parts = string.split(text, ",")
	assert(list.length(parts) == 100_001)
	assert(list.first(parts) == Some("ab"))
	assert(list.last(parts) == Some(""))
}

@test
split_unicode_needle_test = () {
	assert(string.split("aéébé", on: "é") == ["a", "", "b", ""])
	assert(string.split("abababc", on: "abc") == ["abab", ""])
	assert(string.split("aaaa", on: "aa") == ["", "", ""])
	assert(string.split("日本語", on: "本") == ["日", "語"])
}

@test
split_once_more_test = () {
	assert(
		string.split_once("key=value=x", on: "=") == Some(("key", "value=x"))
	)
	assert(string.split_once("novalue", on: "=") == None)
	assert(string.split_once("abc", on: "") == None)
	assert(string.split_once("=", on: "=") == Some(("", "")))
}

@test
lines_test = () {
	assert(string.lines("a\nb\r\nc") == ["a", "b", "c"])
	assert(string.lines("") == [""])
	assert(string.lines("a\n") == ["a", ""])
	assert(string.lines("one line") == ["one line"])
}

@test
replace_more_test = () {
	assert(
		string.replace(in: "www.example.com", each: ".", with: "-") ==
			"www-example-com"
	)
	assert(string.replace("aaa", "a", "bb") == "bbbbbb")
	assert(string.replace("abc", "", "x") == "abc")
	assert(string.replace("héllo", "é", "e") == "hello")
}

@test
contains_more_test = () {
	assert(string.contains(does: "theory", contain: "ory"))
	assert(string.contains("abc", ""))
	assert(string.contains("", ""))
	assert(core.not(string.contains("", "a")))
	assert(core.not(string.contains("ab", "abc")))
	assert(string.contains("héllo", "é"))
	assert(string.contains("aé", "é"))
}

@test
contains_and_crop_test = () {
	assert(string.contains("aaaaaaab", "aaab"))
	assert(core.not(string.contains("aaaaaaa", "aaab")))
	assert(string.contains("𐐨𐐩", "𐐩"))
	assert(string.crop("héllo wörld", before: "wö") == "wörld")
	assert(string.crop("aaab", before: "ab") == "ab")
	assert(string.crop("abc", before: "") == "abc")
}

@test
search_long_test = () {
	text = string.repeat("ab", 50_000) <> "abc"
	assert(string.contains(text, "abc"))
	assert(
		string.split_once(text, "abc") ==
			Some((string.repeat("ab", 50_000), ""))
	)
	assert(
		string.replace(string.repeat("x,", 50_000), ",", "") ==
			string.repeat("x", 50_000)
	)
}

@test
starts_ends_test = () {
	assert(string.starts_with("héllo", "hé"))
	assert(core.not(string.starts_with("héllo", "e")))
	assert(string.starts_with("abc", ""))
	assert(string.ends_with("héllo", "llo"))
	assert(string.ends_with("abc", ""))
	assert(core.not(string.ends_with("bc", "abc")))
}

@test
slice_more_test = () {
	assert(string.slice("gleam", at_index: 1, length: 2) == "le")
	assert(string.slice("gleam", at_index: -3, length: 2) == "ea")
	assert(string.slice("gleam", at_index: 1, length: 10) == "leam")
	assert(string.slice("gleam", at_index: 10, length: 2) == "")
	assert(string.slice("gleam", at_index: -10, length: 2) == "")
	assert(string.slice("gleam", at_index: 1, length: -1) == "")
	assert(string.slice("héllo", at_index: 1, length: 3) == "éll")
}

@test
case_unicode_test = () {
	assert(string.to_upper("héllo, wörld 1") == "HÉLLO, WÖRLD 1")
	assert(string.to_lower("HÉLLO, WORLD") == "héllo, world")
	assert(string.capitalise("hELLO") == "Hello")
	assert(string.capitalise("") == "")
}

@test
case_greek_and_cyrillic_test = () {
	assert(string.to_upper("ωμέγα") == "ΩΜΈΓΑ")
	assert(string.to_lower("ΩΜΈΓΑ") == "ωμέγα")
	assert(string.to_upper("ς") == "Σ")
	assert(string.to_upper("привет") == "ПРИВЕТ")
	assert(string.to_lower("ПРИВЕТ") == "привет")
	assert(string.capitalise("éCOLE") == "École")
	assert(string.capitalise("ΑΘΗΝΑ") == "Αθηνα")
}

@test
case_one_codepoint_for_one_test = () {
	// ß has no simple uppercase mapping, so it stays.
	assert(string.to_upper("straße") == "STRAßE")
	assert(string.to_lower("ẞ") == "ß")
	// No locale: dotless ı goes up to I, and I comes down to i.
	assert(string.to_upper("ı") == "I")
	assert(string.to_lower("I") == "i")
	assert(string.to_lower("İ") == "i")
	assert(string.length(string.to_upper("ßŉǰ")) == 3)
}

@test
case_outside_the_basic_plane_test = () {
	// Deseret.
	assert(string.to_upper("𐐨𐐩") == "𐐀𐐁")
	assert(string.to_lower("𐐀𐐁") == "𐐨𐐩")
	assert(string.to_upper("a😀b") == "A😀B")
}

@test
trim_unicode_test = () {
	assert(string.trim("\u{2028} \t hi \u{85}\n") == "hi")
	// U+00A0 (no-break space) is not Pattern_White_Space.
	assert(string.trim("\u{A0}hi") == "\u{A0}hi")
	assert(string.trim_start("  a  ") == "a  ")
	assert(string.trim_end("  a  ") == "  a")
	assert(string.trim("") == "")
	assert(string.trim("   ") == "")
}

@test
pad_test = () {
	assert(string.pad_left("121", to: 5, with: ".") == "..121")
	assert(string.pad_left("121", to: 6, with: "XY") == "XYX121")
	assert(string.pad_left("121", to: 2, with: ".") == "121")
	assert(string.pad_right("121", to: 5, with: ".") == "121..")
	assert(string.pad_right("121", to: 6, with: "XY") == "121XYX")
	assert(string.pad_right("é", to: 3, with: "é") == "ééé")
	assert(string.pad_left("a", to: 3, with: "") == "a")
}

@test
reverse_more_test = () {
	assert(string.reverse("héllo") == "olléh")
	assert(string.reverse("") == "")
}

@test
codepoints_test = () {
	cps = string.to_codepoints("hé😀")
	assert(list.map(cps, string.utf_codepoint_to_int) == [104, 233, 128512])
	assert(string.from_codepoints(cps) == "hé😀")
	assert(string.to_codepoint_strings("hé😀") == ["h", "é", "😀"])
	assert(result.is_error(string.utf_codepoint(0xD800)))
	assert(result.is_ok(string.utf_codepoint(0x10FFFF)))
}

@test
compare_more_test = () {
	assert(string.compare("a", "b") == Less)
	assert(string.compare("b", "a") == Greater)
	assert(string.compare("abc", "abc") == Equal)
	assert(string.compare("ab", "abc") == Less)
	assert(string.compare("", "a") == Less)
	assert(string.compare("é", "z") == Greater)
	assert(string.compare("Z", "a") == Less)
	assert(
		list.sort(["pear", "apple", "fig", "Apple"], string.compare) ==
			["Apple", "apple", "fig", "pear"]
	)
}

@test
remove_prefix_suffix_test = () {
	assert(string.remove_prefix("@lpil", "@") == "lpil")
	assert(string.remove_prefix("hello", "@") == "hello")
	assert(string.remove_prefix("héllo", "hé") == "llo")
	assert(string.remove_suffix("hello!", "!") == "hello")
	assert(string.remove_suffix("hello", "!") == "hello")
	assert(string.remove_suffix("aé", "é") == "a")
}

@test
drop_more_test = () {
	assert(string.drop_start("héllo", 2) == "llo")
	assert(string.drop_end("héllo", 4) == "h")
	assert(string.drop_start("abc", 10) == "")
	assert(string.drop_end("abc", -1) == "abc")
}

@test
length_long_test = () {
	assert(string.length(string.repeat("é", 1_000_000)) == 1_000_000)
	assert(string.byte_size(string.repeat("é", 1_000_000)) == 2_000_000)
}

@test
join_long_test = () {
	joined = string.join(list.repeat("a", 100_000), with: ",")
	assert(string.byte_size(joined) == 199_999)
}
