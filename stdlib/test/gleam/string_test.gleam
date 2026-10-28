import gleam/dict
import gleam/option.{None, Some}
import gleam/order
import gleam/string

pub fn length_unicode_test() -> Nil {
  assert string.length("ß↑e̊") == 3
  Nil
}

pub fn length_basic_test() -> Nil {
  assert string.length("Gleam") == 5
  Nil
}

pub fn length_empty_test() -> Nil {
  assert string.length("") == 0
  Nil
}

pub fn lowercase_test() -> Nil {
  assert string.lowercase("Gleam") == "gleam"
  Nil
}

pub fn uppercase_test() -> Nil {
  assert string.uppercase("Gleam") == "GLEAM"
  Nil
}

pub fn reverse_basic_test() -> Nil {
  assert string.reverse("Gleam") == "maelG"
  Nil
}

pub fn reverse_with_space_test() -> Nil {
  assert string.reverse(" Gleam") == "maelG "
  Nil
}

pub fn reverse_emoji_test() -> Nil {
  assert string.reverse("👍 OK") == "KO 👍"
  Nil
}

pub fn reverse_single_emoji_test() -> Nil {
  assert string.reverse("👍") == "👍"
  Nil
}

pub fn reverse_unicode_test() -> Nil {
  assert string.reverse("ÅÄÖ") == "ÖÄÅ"
  Nil
}

pub fn reverse_skin_tone_emoji_test() -> Nil {
  assert string.reverse("👶🏿") == "👶🏿"
  Nil
}

pub fn reverse_skin_tone_roundtrip_test() -> Nil {
  assert "👶🏿"
    |> string.reverse
    |> string.reverse
    == "👶🏿"
  Nil
}

pub fn split_comma_test() -> Nil {
  assert string.split("Gleam,Erlang,Elixir", ",")
    == ["Gleam", "Erlang", "Elixir"]
  Nil
}

pub fn split_comma_space_test() -> Nil {
  assert string.split("Gleam, Erlang,Elixir", ", ")
    == ["Gleam", "Erlang,Elixir"]
  Nil
}

pub fn split_empty_test() -> Nil {
  assert string.split("Gleam On Beam", "")
    == ["G", "l", "e", "a", "m", " ", "O", "n", " ", "B", "e", "a", "m"]
  Nil
}

pub fn split_once_ok_test() -> Nil {
  assert string.split_once("Gleam,Erlang,Elixir", ",")
    == Ok(#("Gleam", "Erlang,Elixir"))
  Nil
}

pub fn split_once_not_found_test() -> Nil {
  assert string.split_once("Gleam", ",") == Error(Nil)
  Nil
}

pub fn split_once_empty_test() -> Nil {
  assert string.split_once("", ",") == Error(Nil)
  Nil
}

pub fn replace_test() -> Nil {
  assert string.replace("Gleam,Erlang,Elixir", ",", "++")
    == "Gleam++Erlang++Elixir"
  Nil
}

pub fn append_test() -> Nil {
  assert string.append("Test", " Me") == "Test Me"
  Nil
}

pub fn compare_equal_test() -> Nil {
  assert string.compare("", "") == order.Eq
  Nil
}

pub fn compare_a_gt_empty_test() -> Nil {
  assert string.compare("a", "") == order.Gt
  Nil
}

pub fn compare_lowercase_a_gt_uppercase_a_test() -> Nil {
  assert string.compare("a", "A") == order.Gt
  Nil
}

pub fn compare_uppercase_a_lt_uppercase_b_test() -> Nil {
  assert string.compare("A", "B") == order.Lt
  Nil
}

pub fn compare_t_gt_uppercase_abc_test() -> Nil {
  assert string.compare("t", "ABC") == order.Gt
  Nil
}

pub fn contains_present_test() -> Nil {
  assert string.contains("gleam", "ea")
  Nil
}

pub fn contains_absent_test() -> Nil {
  assert !string.contains("gleam", "x")
  Nil
}

pub fn contains_labelled_test() -> Nil {
  assert string.contains(does: "bellwether", contain: "bell")
  Nil
}

pub fn concat_test() -> Nil {
  assert string.concat(["Hello", ", ", "world!"]) == "Hello, world!"
  Nil
}

pub fn concat_emoji_test() -> Nil {
  assert string.concat(["💃🏿", "💇🏼‍♀️", "🧔‍♂️", "🧑‍🦼‍➡️"]) == "💃🏿💇🏼‍♀️🧔‍♂️🧑‍🦼‍➡️"
  Nil
}

pub fn repeat_one_test() -> Nil {
  assert string.repeat("hi", times: 1) == "hi"
  Nil
}

pub fn repeat_two_test() -> Nil {
  assert string.repeat("hi", times: 2) == "hihi"
  Nil
}

pub fn repeat_three_test() -> Nil {
  assert string.repeat("hi", times: 3) == "hihihi"
  Nil
}

pub fn repeat_large_test() -> Nil {
  assert string.repeat("a", times: 10_001) |> string.length == 10_001
  Nil
}

pub fn repeat_zero_test() -> Nil {
  assert string.repeat("hi", 0) == ""
  Nil
}

pub fn repeat_negative_test() -> Nil {
  assert string.repeat("hi", -1) == ""
  Nil
}

pub fn join_0_test() -> Nil {
  assert string.join([], with: ", ") == ""
  Nil
}

pub fn join_0_dash_test() -> Nil {
  assert string.join([], with: "-") == ""
  Nil
}

pub fn join_1_test() -> Nil {
  assert string.join(["Hello"], with: ", ") == "Hello"
  Nil
}

pub fn join_1_dash_test() -> Nil {
  assert string.join(["Hello"], with: "-") == "Hello"
  Nil
}

pub fn join_2_test() -> Nil {
  assert string.join(["Hello", "world!"], with: ", ") == "Hello, world!"
  Nil
}

pub fn join_2_dash_test() -> Nil {
  assert string.join(["Hello", "world!"], with: "-") == "Hello-world!"
  Nil
}

pub fn join_3_test() -> Nil {
  assert string.join(["Hello", "there", "world!"], with: ", ")
    == "Hello, there, world!"
  Nil
}

pub fn join_3_dash_test() -> Nil {
  assert string.join(["Hello", "there", "world!"], with: "-")
    == "Hello-there-world!"
  Nil
}

pub fn trim_test() -> Nil {
  assert string.trim("  hats  \n") == "hats"
  Nil
}

pub fn trim2_test() -> Nil {
  assert string.trim("k\r1=v2") == "k\r1=v2"
  Nil
}

pub fn trim3_test() -> Nil {
  assert string.trim("  \nhello\nworld\n  ") == "hello\nworld"
  Nil
}

pub fn trim_start_test() -> Nil {
  assert string.trim_start("  hats  \n") == "hats  \n"
  Nil
}

pub fn trim_end_test() -> Nil {
  assert string.trim_end("  hats  \n") == "  hats"
  Nil
}

pub fn trim_whole_string_start_test() -> Nil {
  let s =
    "\u{0020}\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0085}\u{2028}\u{2029}"

  assert string.trim_start(s) == ""
  Nil
}

pub fn trim_whole_string_end_test() -> Nil {
  let s =
    "\u{0020}\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0085}\u{2028}\u{2029}"

  assert string.trim_end(s) == ""
  Nil
}

pub fn trim_whole_string_test() -> Nil {
  let s =
    "\u{0020}\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0085}\u{2028}\u{2029}"

  assert string.trim(s) == ""
  Nil
}

// unicode whitespaces
pub fn trim_horizontal_tab_test() -> Nil {
  assert string.trim("hats\u{0009}") == "hats"
  Nil
}

pub fn trim_newline_test() -> Nil {
  assert string.trim("hats\u{000A}") == "hats"
  Nil
}

pub fn trim_vertical_tab_test() -> Nil {
  assert string.trim("hats\u{000B}") == "hats"
  Nil
}

pub fn trim_form_feed_test() -> Nil {
  assert string.trim("hats\u{000C}") == "hats"
  Nil
}

pub fn trim_carriage_return_test() -> Nil {
  assert string.trim("hats\u{000D}") == "hats"
  Nil
}

pub fn trim_space_test() -> Nil {
  assert string.trim("hats\u{0020}") == "hats"
  Nil
}

pub fn trim_no_break_space_test() -> Nil {
  assert string.trim("hats\u{00A0}") == "hats\u{00A0}"
  Nil
}

pub fn trim_ogham_space_mark_test() -> Nil {
  assert string.trim("hats\u{1680}") == "hats\u{1680}"
  Nil
}

pub fn trim_en_quad_test() -> Nil {
  assert string.trim("hats\u{2000}") == "hats\u{2000}"
  Nil
}

pub fn trim_em_quad_test() -> Nil {
  assert string.trim("hats\u{2001}") == "hats\u{2001}"
  Nil
}

pub fn trim_en_space_test() -> Nil {
  assert string.trim("hats\u{2002}") == "hats\u{2002}"
  Nil
}

pub fn trim_em_space_test() -> Nil {
  assert string.trim("hats\u{2003}") == "hats\u{2003}"
  Nil
}

pub fn trim_three_per_em_space_test() -> Nil {
  assert string.trim("hats\u{2004}") == "hats\u{2004}"
  Nil
}

pub fn trim_four_per_em_space_test() -> Nil {
  assert string.trim("hats\u{2005}") == "hats\u{2005}"
  Nil
}

pub fn trim_six_per_em_space_test() -> Nil {
  assert string.trim("hats\u{2006}") == "hats\u{2006}"
  Nil
}

pub fn trim_figure_space_test() -> Nil {
  assert string.trim("hats\u{2007}") == "hats\u{2007}"
  Nil
}

pub fn trim_punctuation_space_test() -> Nil {
  assert string.trim("hats\u{2008}") == "hats\u{2008}"
  Nil
}

pub fn trim_thin_space_test() -> Nil {
  assert string.trim("hats\u{2009}") == "hats\u{2009}"
  Nil
}

pub fn trim_hair_space_test() -> Nil {
  assert string.trim("hats\u{200A}") == "hats\u{200A}"
  Nil
}

pub fn trim_line_separator_test() -> Nil {
  assert string.trim("hats\u{2028}") == "hats"
  Nil
}

pub fn trim_paragraph_separator_test() -> Nil {
  assert string.trim("hats\u{2029}") == "hats"
  Nil
}

pub fn trim_narrow_no_break_space_test() -> Nil {
  assert string.trim("hats\u{202F}") == "hats\u{202F}"
  Nil
}

pub fn trim_medium_mathematical_space_test() -> Nil {
  assert string.trim("hats\u{205F}") == "hats\u{205F}"
  Nil
}

pub fn trim_ideographic_space_test() -> Nil {
  assert string.trim("hats\u{3000}") == "hats\u{3000}"
  Nil
}

// related unicode non-whitespaces
pub fn trim_mongolian_vowel_separator_test() -> Nil {
  assert string.trim("hats\u{180E}") == "hats\u{180E}"
  Nil
}

pub fn trim_zero_width_space_test() -> Nil {
  assert string.trim("hats\u{200B}") == "hats\u{200B}"
  Nil
}

pub fn trim_zero_width_non_joiner_test() -> Nil {
  assert string.trim("hats\u{200C}") == "hats\u{200C}"
  Nil
}

pub fn trim_zero_width_joiner_test() -> Nil {
  assert string.trim("hats\u{200D}") == "hats\u{200D}"
  Nil
}

pub fn trim_word_joiner_test() -> Nil {
  assert string.trim("hats\u{2060}") == "hats\u{2060}"
  Nil
}

pub fn trim_zero_width_non_breaking_space_test() -> Nil {
  assert string.trim("hats\u{FEFF}") == "hats\u{FEFF}"
  Nil
}

pub fn trim_comma_test() -> Nil {
  assert string.trim("hats,") == "hats,"
  Nil
}

pub fn starts_with_empty_test() -> Nil {
  assert string.starts_with("theory", "")
  Nil
}

pub fn starts_with_match_test() -> Nil {
  assert string.starts_with("theory", "the")
  Nil
}

pub fn starts_with_no_match_test() -> Nil {
  assert !string.starts_with("theory", "ory")
  Nil
}

pub fn starts_with_longer_test() -> Nil {
  assert !string.starts_with("theory", "theory2")
  Nil
}

pub fn ends_with_empty_test() -> Nil {
  assert string.ends_with("theory", "")
  Nil
}

pub fn ends_with_match_test() -> Nil {
  assert string.ends_with("theory", "ory")
  Nil
}

pub fn ends_with_no_match_test() -> Nil {
  assert !string.ends_with("theory", "the")
  Nil
}

pub fn ends_with_longer_test() -> Nil {
  assert !string.ends_with("theory", "theory2")
  Nil
}

pub fn slice_basic_test() -> Nil {
  assert string.slice("gleam", at_index: 1, length: 2) == "le"
  Nil
}

pub fn slice_beyond_length_test() -> Nil {
  assert string.slice("gleam", at_index: 1, length: 10) == "leam"
  Nil
}

pub fn slice_beyond_string_test() -> Nil {
  assert string.slice("gleam", at_index: 10, length: 3) == ""
  Nil
}

pub fn slice_negative_index_test() -> Nil {
  assert string.slice("gleam", at_index: -2, length: 2) == "am"
  Nil
}

pub fn slice_negative_beyond_test() -> Nil {
  assert string.slice("gleam", at_index: -12, length: 2) == ""
  Nil
}

pub fn slice_negative_length_test() -> Nil {
  assert string.slice("gleam", at_index: 2, length: -3) == ""
  Nil
}

pub fn slice_zero_length_test() -> Nil {
  assert string.slice("gleam", at_index: 2, length: 0) == ""
  Nil
}

pub fn slice_emoji_test() -> Nil {
  assert string.slice("👶🏿", at_index: 0, length: 3) == "👶🏿"
  Nil
}

pub fn crop_start_test() -> Nil {
  assert string.crop("gleam", "gl") == "gleam"
  Nil
}

pub fn crop_middle_test() -> Nil {
  assert string.crop("gleam", "le") == "leam"
  Nil
}

pub fn crop_labelled_test() -> Nil {
  assert string.crop(from: "gleam", before: "ea") == "eam"
  Nil
}

pub fn crop_empty_test() -> Nil {
  assert string.crop("gleam", "") == "gleam"
  Nil
}

pub fn crop_not_found_test() -> Nil {
  assert string.crop("gleam", "!") == "gleam"
  Nil
}

pub fn drop_start_basic_test() -> Nil {
  assert string.drop_start("gleam", up_to: 2) == "eam"
  Nil
}

pub fn drop_start_beyond_length_test() -> Nil {
  assert string.drop_start("gleam", up_to: 6) == ""
  Nil
}

pub fn drop_start_negative_test() -> Nil {
  assert string.drop_start("gleam", up_to: -2) == "gleam"
  Nil
}

pub fn drop_start_zero_test() -> Nil {
  assert string.drop_start("gleam", up_to: 0) == "gleam"
  Nil
}

pub fn drop_start_3499_test() -> Nil {
  // https://github.com/gleam-lang/gleam/issues/3499
  assert string.drop_start("\r]", 1) == "]"
  Nil
}

pub fn drop_end_basic_test() -> Nil {
  assert string.drop_end("gleam", up_to: 2) == "gle"
  Nil
}

pub fn drop_end_all_test() -> Nil {
  assert string.drop_end("gleam", up_to: 5) == ""
  Nil
}

pub fn drop_end_negative_test() -> Nil {
  assert string.drop_end("gleam", up_to: -2) == "gleam"
  Nil
}

pub fn drop_end_zero_test() -> Nil {
  assert string.drop_end("gleam", up_to: 0) == "gleam"
  Nil
}

pub fn pad_start_basic_test() -> Nil {
  assert string.pad_start("121", to: 5, with: ".") == "..121"
  Nil
}

pub fn pad_start_exact_test() -> Nil {
  assert string.pad_start("121", to: 3, with: ".") == "121"
  Nil
}

pub fn pad_start_shorter_test() -> Nil {
  assert string.pad_start("121", to: 2, with: ".") == "121"
  Nil
}

pub fn pad_start_partial_multi_test() -> Nil {
  assert string.pad_start("121", to: 4, with: "XY") == "X121"
  Nil
}

pub fn pad_start_exact_multi_test() -> Nil {
  assert string.pad_start("121", to: 5, with: "XY") == "XY121"
  Nil
}

pub fn pad_start_extra_multi_test() -> Nil {
  assert string.pad_start("121", to: 6, with: "XY") == "XYX121"
  Nil
}

pub fn pad_end_basic_test() -> Nil {
  assert string.pad_end("121", to: 5, with: ".") == "121.."
  Nil
}

pub fn pad_end_exact_test() -> Nil {
  assert string.pad_end("121", to: 3, with: ".") == "121"
  Nil
}

pub fn pad_end_shorter_test() -> Nil {
  assert string.pad_end("121", to: 2, with: ".") == "121"
  Nil
}

pub fn pad_end_partial_multi_test() -> Nil {
  assert string.pad_end("121", to: 4, with: "XY") == "121X"
  Nil
}

pub fn pad_end_exact_multi_test() -> Nil {
  assert string.pad_end("121", to: 5, with: "XY") == "121XY"
  Nil
}

pub fn pad_end_extra_multi_test() -> Nil {
  assert string.pad_end("121", to: 6, with: "XY") == "121XYX"
  Nil
}

pub fn pop_grapheme_ok_test() -> Nil {
  assert string.pop_grapheme("gleam") == Ok(#("g", "leam"))
  Nil
}

pub fn pop_grapheme_single_test() -> Nil {
  assert string.pop_grapheme("g") == Ok(#("g", ""))
  Nil
}

pub fn pop_grapheme_empty_test() -> Nil {
  assert string.pop_grapheme("") == Error(Nil)
  Nil
}

pub fn to_graphemes_empty_test() -> Nil {
  assert string.to_graphemes("") == []
  Nil
}

pub fn to_graphemes_escape_chars_test() -> Nil {
  assert string.to_graphemes("\n\t\r\"\\") == ["\n", "\t", "\r", "\"", "\\"]
  Nil
}

pub fn to_graphemes_single_test() -> Nil {
  assert string.to_graphemes("a") == ["a"]
  Nil
}

pub fn to_graphemes_basic_test() -> Nil {
  assert string.to_graphemes("abc") == ["a", "b", "c"]
  Nil
}

pub fn to_graphemes_emoji_test() -> Nil {
  assert string.to_graphemes("🌷🎁💩😜👍🏳️‍🌈") == ["🌷", "🎁", "💩", "😜", "👍", "🏳️‍🌈"]
  Nil
}

pub fn to_graphemes_combining_test() -> Nil {
  assert string.to_graphemes("Ĺo͂řȩm̅") == ["Ĺ", "o͂", "ř", "ȩ", "m̅"]
  Nil
}

pub fn to_graphemes_korean_test() -> Nil {
  assert string.to_graphemes("뎌쉐") == ["뎌", "쉐"]
  Nil
}

pub fn to_graphemes_family_emoji_test() -> Nil {
  assert string.to_graphemes("👨‍👩‍👦‍👦") == ["👨‍👩‍👦‍👦"]
  Nil
}

pub fn to_graphemes_japanese_test() -> Nil {
  assert string.to_graphemes("ごん゙に゙ぢば") == ["ご", "ん゙", "に゙", "ぢ", "ば"]
  Nil
}

pub fn to_graphemes_katakana_test() -> Nil {
  assert string.to_graphemes("パピプペポ") == ["パ", "ピ", "プ", "ペ", "ポ"]
  Nil
}

pub fn to_graphemes_zalgo_test() -> Nil {
  assert string.to_graphemes("Z͑ͫ̓ͪ̂ͫ̽͏̴̙̤̞͉͚̯̞̠͍A̴̵̜̰͔ͫ͗͢L̠ͨͧͩ͘G̴̻͈͍͔̹̑͗̎̅͛́Ǫ̵̹̻̝̳͂̌̌͘!͖̬̰̙̗̿̋ͥͥ̂ͣ̐́́͜͞") == ["Z͑ͫ̓ͪ̂ͫ̽͏̴̙̤̞͉͚̯̞̠͍", "A̴̵̜̰͔ͫ͗͢", "L̠ͨͧͩ͘", "G̴̻͈͍͔̹̑͗̎̅͛́", "Ǫ̵̹̻̝̳͂̌̌͘", "!͖̬̰̙̗̿̋ͥͥ̂ͣ̐́́͜͞"]
  Nil
}

pub fn to_utf_codepoints_empty_test() -> Nil {
  assert string.to_utf_codepoints("") == []
  Nil
}

pub fn to_utf_codepoints_basic_test() -> Nil {
  assert string.to_utf_codepoints("gleam")
    == {
      let assert #(Ok(g), Ok(l), Ok(e), Ok(a), Ok(m)) = #(
        string.utf_codepoint(103),
        string.utf_codepoint(108),
        string.utf_codepoint(101),
        string.utf_codepoint(97),
        string.utf_codepoint(109),
      )
      [g, l, e, a, m]
    }
  Nil
}

pub fn to_utf_codepoints_rainbow_flag_test() -> Nil {
  // ["🏳", "️", "‍", "🌈"]
  let expected = {
    let assert #(
      Ok(waving_white_flag),
      Ok(variant_selector_16),
      Ok(zero_width_joiner),
      Ok(rainbow),
    ) = #(
      string.utf_codepoint(127_987),
      string.utf_codepoint(65_039),
      string.utf_codepoint(8205),
      string.utf_codepoint(127_752),
    )
    [waving_white_flag, variant_selector_16, zero_width_joiner, rainbow]
  }

  assert string.to_utf_codepoints("🏳️‍🌈") == expected
  Nil
}

pub fn from_utf_codepoints_empty_test() -> Nil {
  assert ""
    |> string.to_utf_codepoints
    |> string.from_utf_codepoints
    == ""
  Nil
}

pub fn from_utf_codepoints_basic_test() -> Nil {
  assert "gleam"
    |> string.to_utf_codepoints
    |> string.from_utf_codepoints
    == "gleam"
  Nil
}

pub fn from_utf_codepoints_emoji_test() -> Nil {
  assert "🏳️‍🌈"
    |> string.to_utf_codepoints
    |> string.from_utf_codepoints
    == "🏳️‍🌈"
  Nil
}

pub fn from_utf_codepoints_abc_test() -> Nil {
  assert string.from_utf_codepoints({
      let assert #(Ok(a), Ok(b), Ok(c)) = #(
        string.utf_codepoint(97),
        string.utf_codepoint(98),
        string.utf_codepoint(99),
      )
      [a, b, c]
    })
    == "abc"
  Nil
}

pub fn utf_codepoint_negative_test() -> Nil {
  // Less than the lower bound on valid codepoints
  let assert Error(_) = string.utf_codepoint(-1)
  Nil
}

pub fn utf_codepoint_zero_test() -> Nil {
  // The lower bound on valid codepoints
  let assert Ok(_) = string.utf_codepoint(0)
  Nil
}

pub fn utf_codepoint_max_test() -> Nil {
  // The upper bound for valid code points
  let assert Ok(_) = string.utf_codepoint(1_114_111)
  Nil
}

pub fn utf_codepoint_above_max_test() -> Nil {
  // Greater than the upper bound on valid codepoints
  let assert Error(_) = string.utf_codepoint(1_114_112)
  Nil
}

pub fn utf_codepoint_nonchar_fffe_test() -> Nil {
  // Non-characters U+FFFE and U+FFFF are valid codepoints.  See (#778).
  let assert Ok(_) = string.utf_codepoint(65_534)
  Nil
}

pub fn utf_codepoint_nonchar_ffff_test() -> Nil {
  let assert Ok(_) = string.utf_codepoint(65_535)
  Nil
}

pub fn utf_codepoint_below_high_surrogate_test() -> Nil {
  // One less than the lowest "High-surrogate code point"
  let assert Ok(_) = string.utf_codepoint(55_295)
  Nil
}

pub fn utf_codepoint_high_surrogate_test() -> Nil {
  // Lowest value of the "High-surrogate code point" (U+D800 to U+DBFF)
  let assert Error(_) = string.utf_codepoint(55_296)
  Nil
}

pub fn utf_codepoint_low_surrogate_test() -> Nil {
  // Highest value of the "Low-surrogate code point" (U+DC00 to U+DFFF)
  let assert Error(_) = string.utf_codepoint(57_343)
  Nil
}

pub fn utf_codepoint_above_low_surrogate_test() -> Nil {
  // One greater than the highest "Low-surrogate code point"
  let assert Ok(_) = string.utf_codepoint(57_344)
  Nil
}

pub fn bit_array_utf_codepoint_test() -> Nil {
  let assert Ok(snake) = string.utf_codepoint(128_013)
  let expected = <<"🐍":utf8>>
  assert <<snake:utf8_codepoint>> == expected
  Nil
}

pub fn utf_codepoint_to_int_test() -> Nil {
  assert string.utf_codepoint_to_int({
      let assert Ok(ordinal_value) = string.utf_codepoint(128_013)
      ordinal_value
    })
    == 128_013
  Nil
}

pub fn to_option_empty_test() -> Nil {
  assert string.to_option("") == None
  Nil
}

pub fn to_option_non_empty_test() -> Nil {
  assert string.to_option("ok") == Some("ok")
  Nil
}

pub fn first_empty_test() -> Nil {
  let assert Error(_) = string.first("")
  Nil
}

pub fn first_basic_test() -> Nil {
  assert string.first("gleam") == Ok("g")
  Nil
}

pub fn first_emoji_test() -> Nil {
  assert string.first("⭐️ Gleam") == Ok("⭐️")
  Nil
}

pub fn first_single_test() -> Nil {
  assert string.first("a") == Ok("a")
  Nil
}

pub fn last_empty_test() -> Nil {
  let assert Error(_) = string.last("")
  Nil
}

pub fn last_basic_test() -> Nil {
  assert string.last("gleam") == Ok("m")
  Nil
}

pub fn last_with_space_test() -> Nil {
  assert string.last("gleam ") == Ok(" ")
  Nil
}

pub fn last_unicode_test() -> Nil {
  assert string.last("եdelays") == Ok("s")
  Nil
}

pub fn last_single_test() -> Nil {
  assert string.last("a") == Ok("a")
  Nil
}

pub fn capitalise_empty_test() -> Nil {
  assert string.capitalise("") == ""
  Nil
}

pub fn capitalise_lowercase_test() -> Nil {
  assert string.capitalise("gleam") == "Gleam"
  Nil
}

pub fn capitalise_uppercase_test() -> Nil {
  assert string.capitalise("GLEAM") == "Gleam"
  Nil
}

pub fn capitalise_with_spaces_test() -> Nil {
  assert string.capitalise("g l e a m") == "G l e a m"
  Nil
}

pub fn capitalise_leading_number_test() -> Nil {
  assert string.capitalise("1GLEAM") == "1gleam"
  Nil
}

pub fn capitalise_leading_underscore_test() -> Nil {
  assert string.capitalise("_gLeAm1") == "_gleam1"
  Nil
}

pub fn capitalise_leading_space_test() -> Nil {
  assert string.capitalise(" gLeAm1") == " gleam1"
  Nil
}

pub fn capitalise_unicode_test() -> Nil {
  assert string.capitalise("る") == "る"
  Nil
}

type InspectType(a, b) {
  InspectTypeZero
  InspectTypeOne(a)
  InspectTypeTwo(a, b)
}

pub fn inspect_0_test() -> Nil {
  assert string.inspect(True) == "True"
  Nil
}

pub fn inspect_4_test() -> Nil {
  assert string.inspect(False) == "False"
  Nil
}

pub fn inspect_8_test() -> Nil {
  assert string.inspect([True, False]) == "[True, False]"
  Nil
}

pub fn inspect_12_test() -> Nil {
  assert string.inspect([False, False]) == "[False, False]"
  Nil
}

pub fn inspect_16_test() -> Nil {
  assert string.inspect([True, True]) == "[True, True]"
  Nil
}

pub fn inspect_20_test() -> Nil {
  assert string.inspect([Nil, Nil]) == "[Nil, Nil]"
  Nil
}

pub fn inspect_24_test() -> Nil {
  assert string.inspect(#(True, False)) == "#(True, False)"
  Nil
}

pub fn inspect_28_test() -> Nil {
  assert string.inspect(#(False, False)) == "#(False, False)"
  Nil
}

pub fn inspect_32_test() -> Nil {
  assert string.inspect(#(True, True)) == "#(True, True)"
  Nil
}

pub fn inspect_36_test() -> Nil {
  assert string.inspect(#(Nil, True)) == "#(Nil, True)"
  Nil
}

pub fn inspect_40_test() -> Nil {
  assert string.inspect(#(Nil, False)) == "#(Nil, False)"
  Nil
}

pub fn inspect_44_test() -> Nil {
  assert string.inspect(#(True, Nil)) == "#(True, Nil)"
  Nil
}

pub fn inspect_48_test() -> Nil {
  assert string.inspect(#(False, Nil)) == "#(False, Nil)"
  Nil
}

pub fn inspect_52_test() -> Nil {
  assert string.inspect(-1) == "-1"
  Nil
}

pub fn inspect_56_test() -> Nil {
  assert string.inspect(0) == "0"
  Nil
}

pub fn inspect_60_test() -> Nil {
  assert string.inspect(1) == "1"
  Nil
}

pub fn inspect_64_test() -> Nil {
  assert string.inspect([]) == "[]"
  Nil
}

pub fn inspect_68_test() -> Nil {
  assert string.inspect([1]) == "[1]"
  Nil
}

pub fn inspect_72_test() -> Nil {
  assert string.inspect([1, 2]) == "[1, 2]"
  Nil
}

pub fn inspect_76_test() -> Nil {
  assert string.inspect([[1], [1]]) == "[[1], [1]]"
  Nil
}

pub fn inspect_80_test() -> Nil {
  assert string.inspect(-1.5) == "-1.5"
  Nil
}

pub fn inspect_84_test() -> Nil {
  assert string.inspect(5.0e-26) == "5.0e-26"
  Nil
}

pub fn inspect_88_test() -> Nil {
  assert string.inspect(1.5) == "1.5"
  Nil
}

pub fn inspect_92_test() -> Nil {
  assert string.inspect(-5.0e-26) == "-5.0e-26"
  Nil
}

pub fn inspect_96_test() -> Nil {
  assert string.inspect([1.5]) == "[1.5]"
  Nil
}

pub fn inspect_100_test() -> Nil {
  assert string.inspect("") == "\"\""
  Nil
}

pub fn inspect_104_test() -> Nil {
  assert string.inspect("\\") == "\"\\\\\""
  Nil
}

pub fn inspect_108_test() -> Nil {
  assert string.inspect("\\\\") == "\"\\\\\\\\\""
  Nil
}

pub fn inspect_112_test() -> Nil {
  assert string.inspect("\\\\\\") == "\"\\\\\\\\\\\\\""
  Nil
}

pub fn inspect_116_test() -> Nil {
  assert string.inspect("\"") == "\"\\\"\""
  Nil
}

pub fn inspect_120_test() -> Nil {
  assert string.inspect("\"\"") == "\"\\\"\\\"\""
  Nil
}

pub fn inspect_124_test() -> Nil {
  assert string.inspect("\r") == "\"\\r\""
  Nil
}

pub fn inspect_128_test() -> Nil {
  assert string.inspect("\n") == "\"\\n\""
  Nil
}

pub fn inspect_132_test() -> Nil {
  assert string.inspect("\t") == "\"\\t\""
  Nil
}

pub fn inspect_136_test() -> Nil {
  assert string.inspect("\f") == "\"\\f\""
  Nil
}

pub fn inspect_140_test() -> Nil {
  assert string.inspect("\u{0008}") == "\"\\u{0008}\""
  Nil
}

pub fn inspect_144_test() -> Nil {
  assert string.inspect("\u{000B}") == "\"\\u{000B}\""
  Nil
}

pub fn inspect_148_test() -> Nil {
  assert string.inspect("\u{001B}") == "\"\\u{001B}\""
  Nil
}

pub fn inspect_152_test() -> Nil {
  assert string.inspect("\u{0015}") == "\"\\u{0015}\""
  Nil
}

pub fn inspect_156_test() -> Nil {
  assert string.inspect("\u{001F}") == "\"\\u{001F}\""
  Nil
}

pub fn inspect_160_test() -> Nil {
  assert string.inspect("\u{0020}") == "\" \""
  Nil
}

pub fn inspect_164_test() -> Nil {
  assert string.inspect("\u{007F}") == "\"\\u{007F}\""
  Nil
}

pub fn inspect_168_test() -> Nil {
  assert string.inspect("\u{009F}") == "\"\\u{009F}\""
  Nil
}

pub fn inspect_172_test() -> Nil {
  assert string.inspect("\u{00A0}") == "\"\u{00A0}\""
  Nil
}

pub fn inspect_176_test() -> Nil {
  assert string.inspect("\r\r") == "\"\\r\\r\""
  Nil
}

pub fn inspect_180_test() -> Nil {
  assert string.inspect("\n\n") == "\"\\n\\n\""
  Nil
}

pub fn inspect_184_test() -> Nil {
  assert string.inspect("\r\n") == "\"\\r\\n\""
  Nil
}

pub fn inspect_188_test() -> Nil {
  assert string.inspect("\n\r") == "\"\\n\\r\""
  Nil
}

pub fn inspect_192_test() -> Nil {
  assert string.inspect("\t\t") == "\"\\t\\t\""
  Nil
}

pub fn inspect_196_test() -> Nil {
  assert string.inspect("\t\n") == "\"\\t\\n\""
  Nil
}

pub fn inspect_200_test() -> Nil {
  assert string.inspect("\n\t") == "\"\\n\\t\""
  Nil
}

pub fn inspect_204_test() -> Nil {
  assert string.inspect("\t\r") == "\"\\t\\r\""
  Nil
}

pub fn inspect_208_test() -> Nil {
  assert string.inspect("\r\t") == "\"\\r\\t\""
  Nil
}

pub fn inspect_212_test() -> Nil {
  assert string.inspect("\t\f") == "\"\\t\\f\""
  Nil
}

pub fn inspect_216_test() -> Nil {
  assert string.inspect("\f\t") == "\"\\f\\t\""
  Nil
}

pub fn inspect_220_test() -> Nil {
  assert string.inspect("\t\u{0008}") == "\"\\t\\u{0008}\""
  Nil
}

pub fn inspect_224_test() -> Nil {
  assert string.inspect("\u{0008}\t") == "\"\\u{0008}\\t\""
  Nil
}

pub fn inspect_228_test() -> Nil {
  assert string.inspect("\t\u{000B}") == "\"\\t\\u{000B}\""
  Nil
}

pub fn inspect_232_test() -> Nil {
  assert string.inspect("\u{000B}\t") == "\"\\u{000B}\\t\""
  Nil
}

pub fn inspect_236_test() -> Nil {
  assert string.inspect("\t\u{001B}") == "\"\\t\\u{001B}\""
  Nil
}

pub fn inspect_240_test() -> Nil {
  assert string.inspect("\u{001B}\t") == "\"\\u{001B}\\t\""
  Nil
}

pub fn inspect_244_test() -> Nil {
  assert string.inspect("\\\n\\") == "\"\\\\\\n\\\\\""
  Nil
}

pub fn inspect_248_test() -> Nil {
  assert string.inspect("\\\"\\") == "\"\\\\\\\"\\\\\""
  Nil
}

pub fn inspect_252_test() -> Nil {
  assert string.inspect("\\\"\"\\") == "\"\\\\\\\"\\\"\\\\\""
  Nil
}

pub fn inspect_256_test() -> Nil {
  assert string.inspect("'") == "\"'\""
  Nil
}

pub fn inspect_260_test() -> Nil {
  assert string.inspect("''") == "\"''\""
  Nil
}

pub fn inspect_264_test() -> Nil {
  assert string.inspect("around-single-quotes'around-single-quotes")
    == "\"around-single-quotes'around-single-quotes\""
  Nil
}

pub fn inspect_269_test() -> Nil {
  assert string.inspect("'between-single-quotes'")
    == "\"'between-single-quotes'\""
  Nil
}

pub fn inspect_274_test() -> Nil {
  assert string.inspect("0") == "\"0\""
  Nil
}

pub fn inspect_278_test() -> Nil {
  assert string.inspect("1") == "\"1\""
  Nil
}

pub fn inspect_282_test() -> Nil {
  assert string.inspect("2") == "\"2\""
  Nil
}

pub fn inspect_286_test() -> Nil {
  assert string.inspect("Hello Joe!") == "\"Hello Joe!\""
  Nil
}

pub fn inspect_290_test() -> Nil {
  assert string.inspect("Hello \"Manuel\"!") == "\"Hello \\\"Manuel\\\"!\""
  Nil
}

pub fn inspect_294_test() -> Nil {
  assert string.inspect("👨‍👩‍👦‍👦 💜 Gleam") == "\"👨‍👩‍👦‍👦 💜 Gleam\""
  Nil
}

pub fn inspect_298_test() -> Nil {
  assert string.inspect("✨") == "\"✨\""
  Nil
}

pub fn inspect_302_test() -> Nil {
  assert string.inspect("🏳️‍⚧️") == "\"🏳️‍⚧️\""
  Nil
}

pub fn inspect_306_test() -> Nil {
  assert string.inspect("True") == "\"True\""
  Nil
}

pub fn inspect_310_test() -> Nil {
  assert string.inspect("False") == "\"False\""
  Nil
}

pub fn inspect_314_test() -> Nil {
  assert string.inspect("Nil") == "\"Nil\""
  Nil
}

pub fn inspect_318_test() -> Nil {
  assert string.inspect(["1"]) == "[\"1\"]"
  Nil
}

pub fn inspect_322_test() -> Nil {
  assert string.inspect(#()) == "#()"
  Nil
}

pub fn inspect_326_test() -> Nil {
  assert string.inspect(#(1)) == "#(1)"
  Nil
}

pub fn inspect_330_test() -> Nil {
  assert string.inspect(#("1")) == "#(\"1\")"
  Nil
}

pub fn inspect_334_test() -> Nil {
  assert string.inspect(#(1.5)) == "#(1.5)"
  Nil
}

pub fn inspect_338_test() -> Nil {
  assert string.inspect([#(1, 2, 3), #(1, 2, 3)]) == "[#(1, 2, 3), #(1, 2, 3)]"
  Nil
}

pub fn inspect_342_test() -> Nil {
  assert string.inspect(#([1, 2, 3], "🌈", "🏳️‍🌈", #(1, "1", True)))
    == "#([1, 2, 3], \"🌈\", \"🏳️‍🌈\", #(1, \"1\", True))"
  Nil
}

pub fn inspect_347_test() -> Nil {
  assert string.inspect(Nil) == "Nil"
  Nil
}

pub fn inspect_351_test() -> Nil {
  assert string.inspect(Ok(1)) == "Ok(1)"
  Nil
}

pub fn inspect_355_test() -> Nil {
  assert string.inspect(Ok(True)) == "Ok(True)"
  Nil
}

pub fn inspect_359_test() -> Nil {
  assert string.inspect(Ok(False)) == "Ok(False)"
  Nil
}

pub fn inspect_363_test() -> Nil {
  assert string.inspect(Ok(Nil)) == "Ok(Nil)"
  Nil
}

pub fn inspect_367_test() -> Nil {
  assert string.inspect(Error(2)) == "Error(2)"
  Nil
}

pub fn inspect_371_test() -> Nil {
  assert string.inspect(Error(True)) == "Error(True)"
  Nil
}

pub fn inspect_375_test() -> Nil {
  assert string.inspect(Error(False)) == "Error(False)"
  Nil
}

pub fn inspect_379_test() -> Nil {
  assert string.inspect(Error(Nil)) == "Error(Nil)"
  Nil
}

pub fn inspect_383_test() -> Nil {
  assert string.inspect(InspectTypeZero) == "InspectTypeZero"
  Nil
}

pub fn inspect_387_test() -> Nil {
  assert string.inspect(InspectTypeOne(1)) == "InspectTypeOne(1)"
  Nil
}

pub fn inspect_391_test() -> Nil {
  assert string.inspect(InspectTypeTwo(1, 2)) == "InspectTypeTwo(1, 2)"
  Nil
}

pub fn inspect_395_test() -> Nil {
  assert string.inspect(InspectTypeOne([1])) == "InspectTypeOne([1])"
  Nil
}

pub fn inspect_399_test() -> Nil {
  assert string.inspect(InspectTypeOne("1")) == "InspectTypeOne(\"1\")"
  Nil
}

pub fn inspect_403_test() -> Nil {
  assert string.inspect(InspectTypeOne(["1"])) == "InspectTypeOne([\"1\"])"
  Nil
}

pub fn inspect_407_test() -> Nil {
  assert string.inspect(InspectTypeOne(#([1], "a")))
    == "InspectTypeOne(#([1], \"a\"))"
  Nil
}

pub fn inspect_412_test() -> Nil {
  assert string.inspect(Ok) == "//fn(a) { ... }"
  Nil
}

pub fn inspect_416_test() -> Nil {
  assert string.inspect(Error) == "//fn(a) { ... }"
  Nil
}

pub fn inspect_420_test() -> Nil {
  assert string.inspect(fn() { Nil }) == "//fn() { ... }"
  Nil
}

pub fn inspect_424_test() -> Nil {
  assert string.inspect(fn(_) { Nil }) == "//fn(a) { ... }"
  Nil
}

pub fn inspect_428_test() -> Nil {
  assert string.inspect(fn(_, _) { Nil }) == "//fn(a, b) { ... }"
  Nil
}

pub fn inspect_432_test() -> Nil {
  assert string.inspect(fn(_, _) { Nil }) == "//fn(a, b) { ... }"
  Nil
}

pub fn inspect_436_test() -> Nil {
  assert string.inspect(fn(_: Int, _: String) -> Bool { False })
    == "//fn(a, b) { ... }"
  Nil
}

pub fn inspect_441_test() -> Nil {
  assert string.inspect(#(InspectTypeOne, InspectTypeTwo))
    == "#(//fn(a) { ... }, //fn(a, b) { ... })"
  Nil
}

pub fn inspect_446_test() -> Nil {
  assert string.inspect(InspectTypeOne(InspectTypeZero))
    == "InspectTypeOne(InspectTypeZero)"
  Nil
}

pub fn inspect_451_test() -> Nil {
  assert string.inspect(<<255, 2, 0>>) == "<<255, 2, 0>>"
  Nil
}

pub fn inspect_charlist_test() -> Nil {
  let list = [
    70, 97, 105, 108, 101, 100, 32, 116, 111, 32, 108, 111, 97, 100, 32, 78, 73,
    70, 32, 108, 105, 98, 114, 97, 114, 121, 58, 32, 39, 47, 114, 117, 110, 47,
    99, 117, 114, 114, 101, 110, 116, 45, 115, 121, 115, 116, 101, 109, 47, 115,
    119, 47, 115, 104, 97, 114, 101, 47, 110, 105, 120, 45, 108, 100, 47, 108,
    105, 98, 47, 108, 105, 98, 99, 114, 121, 112, 116, 111, 46, 115, 111, 46, 51,
  ]
  // A Wasm List(Int) remains a list even when every value is printable.
  assert string.inspect(list)
    == "[70, 97, 105, 108, 101, 100, 32, 116, 111, 32, 108, 111, 97, 100, 32, 78, 73, 70, 32, 108, 105, 98, 114, 97, 114, 121, 58, 32, 39, 47, 114, 117, 110, 47, 99, 117, 114, 114, 101, 110, 116, 45, 115, 121, 115, 116, 101, 109, 47, 115, 119, 47, 115, 104, 97, 114, 101, 47, 110, 105, 120, 45, 108, 100, 47, 108, 105, 98, 47, 108, 105, 98, 99, 114, 121, 112, 116, 111, 46, 115, 111, 46, 51]"
  Nil
}

// WasmGC carries distinct runtime tags for tuples, constructors, floats,
// strings and bit arrays; these assertions replace duplicated host branches.
pub fn target_inspect_tuple_test() -> Nil {
  assert string.inspect(#(InspectTypeZero, InspectTypeZero))
    == "#(InspectTypeZero, InspectTypeZero)"
  Nil
}

pub fn target_inspect_float_negative_test() -> Nil {
  assert string.inspect(-1.0) == "-1.0"
  Nil
}

pub fn target_inspect_float_zero_test() -> Nil {
  assert string.inspect(0.0) == "0.0"
  Nil
}

pub fn target_inspect_float_positive_test() -> Nil {
  assert string.inspect(1.0) == "1.0"
  Nil
}

pub fn target_inspect_float_list_test() -> Nil {
  assert string.inspect([1.0]) == "[1.0]"
  Nil
}

pub fn target_inspect_float_tuple_test() -> Nil {
  assert string.inspect(#(1.0)) == "#(1.0)"
  Nil
}

pub fn target_inspect_bit_array_test() -> Nil {
  assert string.inspect(<<"abc":utf8>>) == "<<97, 98, 99>>"
  Nil
}

pub fn inspect_fifteen_bit_int_test() -> Nil {
  assert string.inspect(<<2, 3:size(7)>>) == "<<2, 3:size(7)>>"
  Nil
}

pub fn byte_size_empty_test() -> Nil {
  let assert 0 = string.byte_size("")
  Nil
}

pub fn byte_size_one_test() -> Nil {
  let assert 1 = string.byte_size("a")
  Nil
}

pub fn byte_size_two_test() -> Nil {
  let assert 2 = string.byte_size("ab")
  Nil
}

pub fn byte_size_three_test() -> Nil {
  let assert 3 = string.byte_size("abc")
  Nil
}

pub fn byte_size_unicode_a_test() -> Nil {
  // Unicode graphemes. These will be multiple bytes.
  let assert 1 = string.byte_size("a")
  Nil
}

pub fn byte_size_unicode_umlaut_test() -> Nil {
  let assert 2 = string.byte_size("ä")
  Nil
}

pub fn byte_size_unicode_emoji_test() -> Nil {
  let assert 4 = string.byte_size("👩")
  Nil
}

pub fn byte_size_unicode_skin_tone_test() -> Nil {
  let assert 8 = string.byte_size("👩🏾")
  Nil
}

pub fn byte_size_unicode_complex_test() -> Nil {
  let assert 15 = string.byte_size("👩🏾‍🦰")
  Nil
}

pub fn inspect_map_test() -> Nil {
  assert string.inspect(dict.from_list([#("a", 1), #("b", 2)]))
    == "dict.from_list([#(\"a\", 1), #(\"b\", 2)])"
  Nil
}

pub fn remove_prefix_test() -> Nil {
  assert string.remove_prefix("gleamlucy", "gleam") == "lucy"
  assert string.remove_prefix("lucygleam", "gleam") == "lucygleam"
  assert string.remove_prefix("lucygleam", "") == "lucygleam"
  assert string.remove_prefix("", "gleam") == ""
  assert string.remove_prefix("gleam", "gleam") == ""
  assert string.remove_prefix("🦑lucy", "🦑") == "lucy"
  assert string.remove_prefix("🦑🦑", "🦑") == "🦑"
  assert string.remove_prefix("abc", "abcde") == "abc"
  assert string.remove_prefix(from: "abc", matching: "ab") == "c"
  Nil
}

pub fn remove_suffix_test() -> Nil {
  assert string.remove_suffix("lucygleam", "gleam") == "lucy"
  assert string.remove_suffix("gleamlucy", "gleam") == "gleamlucy"
  assert string.remove_suffix("lucygleam", "") == "lucygleam"
  assert string.remove_suffix("", "gleam") == ""
  assert string.remove_suffix("gleam", "gleam") == ""
  assert string.remove_suffix("lucy🦑", "🦑") == "lucy"
  assert string.remove_suffix("🦑🦑", "🦑") == "🦑"
  assert string.remove_suffix(from: "abc", matching: "bc") == "a"
  Nil
}
