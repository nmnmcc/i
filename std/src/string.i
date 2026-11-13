//// Functions for `String`, the type of lists of Unicode scalar values kept
//// as UTF-8.
////
//// Lengths, indexes, and slices count **codepoints** (Unicode scalar
//// values), not bytes and not graphemes: `"é"` written as `e` and a
//// combining accent is two codepoints. `byte_size` counts UTF-8 bytes.
////
//// Access and searches count scalars. Negative `at` indexes count from the
//// end, as in `slice`; search bounds must be between zero and length.
//// Access, taking and traversals decode once, costing O(bytes) time and
//// O(scalars) cells, even for early access. Searches decode both strings and
//// use naive matching: O(n*m) comparisons and O(n+m) cells. Int arithmetic
//// and callbacks add their costs. No grapheme or normalization policy.
////
//// `to_upper`, `to_lower`, and `capitalise` use the simple case mappings of
//// Unicode (`core.to_upper` and `core.to_lower`): each codepoint changes to
//// one codepoint, with no locale, so `"ß"` stays `"ß"`. `trim` and
//// the other trim functions take out the characters of Unicode
//// Pattern_White_Space: U+0009 to U+000D, U+0020, U+0085, U+200E, U+200F,
//// U+2028, and U+2029.
(
	i/core
	list = std/list
	option = (Option, Some, None) = std/option
	order = (Order, Less, Equal, Greater) = std/order
)

(
	length
	byte_size
	is_empty
	concat
	append
	join
	repeat
	split
	split_once
	lines
	trim
	trim_start
	trim_end
	starts_with
	ends_with
	contains
	replace
	slice
	drop_start
	drop_end
	first
	last
	pop_codepoint
	crop
	remove_prefix
	remove_suffix
	to_upper
	to_lower
	capitalise
	reverse
	pad_left
	pad_right
	to_codepoints
	from_codepoints
	to_codepoint_strings
	utf_codepoint
	utf_codepoint_to_int
	compare
	inspect
	to_option
	at
	index_of
	last_index_of
	count
	take_start
	take_end
	split_once_last
	replace_n
	split_when
	words
	map
	filter
	fold
)

/// The number of codepoints.
///
/// ```i
/// string.length("héllo")  // 5
/// ```
length = (string: String) -> Int { list.length(core.codepoints(string)) }

/// The number of UTF-8 bytes.
///
/// ```i
/// string.byte_size("héllo")  // 6
/// ```
byte_size = (string: String) -> Int { core.byte_size(string) }

/// `True` for `""`.
is_empty = (string: String) -> Bool { string == "" }

/// The strings one after the other.
concat = (strings: List(String)) -> String { core.concat(strings) }

/// `to` and then `suffix`, as a function.
append = (to: String, suffix: String) -> String { to <> suffix }

/// The strings with `with` between every two of them.
///
/// ```i
/// string.join(["a", "b", "c"], ", ")  // "a, b, c"
/// ```
join = (strings: List(String), with: String) -> String {
	core.concat(list.intersperse(strings, with))
}

/// The string `times` times, one after the other. `""` when `times` is 0 or
/// less.
repeat = (string: String, times: Int) -> String {
	core.concat(list.repeat(string, times))
}

/// Cuts a string at every place where `on` is, and takes out `on`. With
/// `on` as `""`, it cuts between every two codepoints.
///
/// ```i
/// string.split("a,b,,c", ",")  // ["a", "b", "", "c"]
/// string.split("abc", "")      // ["a", "b", "c"]
/// ```
split = (string: String, on: String) -> List(String) {
	on |>
		("") { to_codepoint_strings(string) }
		| (_) {
			n = core.byte_size(on)
			go = (from, acc) {
				core.index_of(string, on, from) |>
					(Ok(at)) {
						go(at + n, [byte_range(string, from, at), ..acc])
					}
					| (Error(_)) {
						list.reverse(
							[
								byte_range(
									string,
									from,
									core.byte_size(string),
								),
								..acc,
							],
						)
					}
			}
			go(0, [])
		}
}

from_reversed = (codepoints: List(UtfCodepoint)) -> String {
	core.from_codepoints(list.reverse(codepoints))
}

// The bytes of `string` from byte `start` to byte `end`, which are
// character boundaries, as `core.index_of` gives them.
byte_range = (string: String, start: Int, end: Int) -> String {
	core.byte_slice(string, start, end - start) |>
		(Ok(part)) { part }
		| (Error(_)) { "" }
}

/// Cuts a string in two at the first place where `on` is, and takes out
/// `on`. `None` when `on` is not in the string, or is `""`.
///
/// ```i
/// string.split_once("key=value=x", "=")  // Some(("key", "value=x"))
/// ```
split_once = (string: String, on: String) -> Option((String, String)) {
	on |>
		("") { None }
		| (_) {
			core.index_of(string, on, 0) |>
				(Ok(at)) {
					Some(
						(
							byte_range(string, 0, at),
							byte_range(
								string,
								at + core.byte_size(on),
								core.byte_size(string),
							),
						),
					)
				}
				| (Error(_)) { None }
		}
}

/// Cuts a string into lines, at `"\n"` and at `"\r\n"`.
///
/// ```i
/// string.lines("a\nb\r\nc")  // ["a", "b", "c"]
/// ```
lines = (string: String) -> List(String) {
	list.map(split(string, "\n"), (line) {
		ends_with(line, "\r") |> (True) { drop_end(line, 1) } | (False) { line }
	})
}

is_space = (c: UtfCodepoint) -> Bool {
	n = core.codepoint_to_int(c)
	{ n >= 9 && n <= 13 } ||
		n == 32 ||
		n == 133 ||
		n == 8206 ||
		n == 8207 ||
		n == 8232 ||
		n == 8233
}

/// The string without white space at the start and at the end.
///
/// ```i
/// string.trim("  hi \n")  // "hi"
/// ```
trim = (string: String) -> String { trim_end(trim_start(string)) }

/// The string without white space at the start.
trim_start = (string: String) -> String {
	core.from_codepoints(
		list.drop_while(core.codepoints(string), is_space),
	)
}

/// The string without white space at the end.
trim_end = (string: String) -> String {
	from_reversed(
		list.drop_while(
			list.reverse(core.codepoints(string)),
			is_space,
		),
	)
}

/// `True` when the string starts with `prefix`.
starts_with = (string: String, prefix: String) -> Bool {
	core.byte_slice(string, 0, core.byte_size(prefix)) == Ok(prefix)
}

/// `True` when the string ends with `suffix`.
ends_with = (string: String, suffix: String) -> Bool {
	n = core.byte_size(suffix)
	core.byte_slice(string, core.byte_size(string) - n, n) == Ok(suffix)
}

/// `True` when `contain` is somewhere in the string. Every string contains
/// `""`.
///
/// ```i
/// string.contains("theory", "ory")  // True
/// ```
contains = (does: String, contain: String) -> Bool {
	core.index_of(does, contain, 0) |> (Ok(_)) { True } | (Error(_)) { False }
}

/// Puts `with` in the place of every `each` in the string. When `each` is
/// `""`, the string is the same.
///
/// ```i
/// string.replace("www.example.com", ".", "-")  // "www-example-com"
/// ```
replace = (in: String, each: String, with: String) -> String {
	each |> ("") { in } | (_) { join(split(in, each), with) }
}

/// `length` codepoints from the codepoint at `at_index`, counting from 0.
/// A negative index counts from the end: `-1` is the last codepoint. The
/// slice stops at the end of the string. A negative length gives `""`.
///
/// ```i
/// string.slice("hello", 1, 2)   // "el"
/// string.slice("hello", -3, 2)  // "ll"
/// ```
slice = (string: String, at_index: Int, length: Int) -> String {
	{ length <= 0 } |>
		(True) { "" }
		| (False) {
			codepoints = core.codepoints(string)
			start = { at_index < 0 } |>
				(True) { list.length(codepoints) + at_index }
				| (False) { at_index }
			{ start < 0 } |>
				(True) { "" }
				| (False) {
					core.from_codepoints(
						list.take(list.drop(codepoints, start), length),
					)
				}
		}
}

/// The string without its first `up_to` codepoints.
drop_start = (from: String, up_to: Int) -> String {
	core.from_codepoints(
		list.drop(core.codepoints(from), up_to),
	)
}

/// The string without its last `up_to` codepoints.
drop_end = (from: String, up_to: Int) -> String {
	from_reversed(
		list.drop(
			list.reverse(core.codepoints(from)),
			up_to,
		),
	)
}

/// The first codepoint, as a string, or `None` for `""`.
first = (string: String) -> Option(String) {
	core.codepoints(string) |>
		([]) { None }
		| ([c, ..]) { Some(core.from_codepoints([c])) }
}

/// The last codepoint, as a string, or `None` for `""`.
last = (string: String) -> Option(String) {
	list.last(core.codepoints(string)) |>
		(Some(c)) { Some(core.from_codepoints([c])) }
		| (None) { None }
}

/// The first codepoint, as a string, and the rest, or `None` for `""`.
///
/// ```i
/// string.pop_codepoint("abc")  // Some(("a", "bc"))
/// ```
pop_codepoint = (string: String) -> Option((String, String)) {
	core.codepoints(string) |>
		([]) { None }
		| ([c, ..cs]) {
			Some((core.from_codepoints([c]), core.from_codepoints(cs)))
		}
}

/// The string from the first place where `before` is. When `before` is not
/// there, the string is the same.
///
/// ```i
/// string.crop("The Lone Ranger", "Lone")  // "Lone Ranger"
/// ```
crop = (from: String, before: String) -> String {
	core.index_of(from, before, 0) |>
		(Ok(at)) { byte_range(from, at, core.byte_size(from)) }
		| (Error(_)) { from }
}

/// The string without `prefix` at its start. When it does not start with
/// `prefix`, the string is the same.
///
/// ```i
/// string.remove_prefix("@lpil", "@")  // "lpil"
/// ```
remove_prefix = (from: String, matching: String) -> String {
	n = core.byte_size(matching)
	starts_with(from, matching) |>
		(True) {
			core.byte_slice(from, n, core.byte_size(from) - n) |>
				(Ok(rest)) { rest }
				| (Error(_)) { from }
		}
		| (False) { from }
}

/// The string without `suffix` at its end. When it does not end with
/// `suffix`, the string is the same.
remove_suffix = (from: String, matching: String) -> String {
	n = core.byte_size(from) - core.byte_size(matching)
	ends_with(from, matching) |>
		(True) {
			core.byte_slice(from, 0, n) |>
				(Ok(rest)) { rest }
				| (Error(_)) { from }
		}
		| (False) { from }
}

/// The string with every codepoint in upper case, by the simple uppercase
/// mapping of Unicode. A codepoint with no such mapping, such as `ß`, stays
/// as it is.
///
/// ```i
/// string.to_upper("héllo")  // "HÉLLO"
/// string.to_upper("straße")  // "STRAßE"
/// ```
to_upper = (string: String) -> String { core.to_upper(string) }

/// The string with every codepoint in lower case, by the simple lowercase
/// mapping of Unicode.
///
/// ```i
/// string.to_lower("ΑΒΓ")  // "αβγ"
/// ```
to_lower = (string: String) -> String { core.to_lower(string) }

/// The first codepoint in upper case and the rest in lower case.
///
/// ```i
/// string.capitalise("mamouna")  // "Mamouna"
/// ```
capitalise = (string: String) -> String {
	pop_codepoint(string) |>
		(Some((head, tail))) { to_upper(head) <> to_lower(tail) }
		| (None) { "" }
}

/// The codepoints in the other order.
///
/// ```i
/// string.reverse("abc")  // "cba"
/// ```
reverse = (string: String) -> String { from_reversed(core.codepoints(string)) }

// `length` codepoints of `with`, repeated as needed.
padding = (length: Int, with: String) -> String {
	pad = core.codepoints(with)
	pad |>
		([]) { "" }
		| (_) {
			go = (n, rest, acc) {
				{ n <= 0 } |>
					(True) { from_reversed(acc) }
					| (False) {
						rest |>
							([]) { go(n, pad, acc) }
							| ([c, ..cs]) { go(n - 1, cs, [c, ..acc]) }
					}
			}
			go(length, pad, [])
		}
}

/// Puts `with`, repeated, before the string until it has `to` codepoints.
/// The last copy of `with` may be cut. A string that is long enough stays.
///
/// ```i
/// string.pad_left("121", 5, ".")  // "..121"
/// string.pad_left("121", 6, "XY")  // "XYX121"
/// ```
pad_left = (string: String, to: Int, with: String) -> String {
	padding(to - length(string), with) <> string
}

/// Puts `with`, repeated, after the string until it has `to` codepoints.
/// The last copy of `with` may be cut. A string that is long enough stays.
pad_right = (string: String, to: Int, with: String) -> String {
	string <> padding(to - length(string), with)
}

/// The codepoints of the string, in order.
to_codepoints = (string: String) -> List(UtfCodepoint) {
	core.codepoints(string)
}

/// The string of these codepoints.
from_codepoints = (codepoints: List(UtfCodepoint)) -> String {
	core.from_codepoints(codepoints)
}

/// Every codepoint as a string of its own.
///
/// ```i
/// string.to_codepoint_strings("hé")  // ["h", "é"]
/// ```
to_codepoint_strings = (string: String) -> List(String) {
	list.map(core.codepoints(string), (c) { core.from_codepoints([c]) })
}

/// The codepoint with this number, or `Error(Nil)` when it is not a Unicode
/// scalar value (0 to 0x10FFFF, but not 0xD800 to 0xDFFF).
utf_codepoint = (value: Int) -> Result(UtfCodepoint, Nil) {
	core.int_to_codepoint(value)
}

/// The number of a codepoint.
utf_codepoint_to_int = (codepoint: UtfCodepoint) -> Int {
	core.codepoint_to_int(codepoint)
}

/// Compares two strings by their codepoints, one by one. This is also the
/// order of their UTF-8 bytes.
///
/// ```i
/// string.compare("Anthony", "Anthony")  // Equal
/// string.compare("A", "B")              // Less
/// ```
compare = (a: String, b: String) -> Order {
	{ a == b } |>
		(True) { Equal }
		| (False) { compare_bytes(<<a:utf8>>, <<b:utf8>>) }
}

compare_bytes = (a: BitArray, b: BitArray) -> Order {
	(a, b) |>
		((<<x, xs:bytes>>, <<y, ys:bytes>>)) {
			{ x == y } |>
				(True) { compare_bytes(xs, ys) }
				| (False) { { x < y } |> (True) { Less } | (False) { Greater } }
		}
		| ((<<>>, <<>>)) { Equal }
		| ((<<>>, _)) { Less }
		| (_) { Greater }
}

/// The text form of any value, as i source (§11.8).
///
/// ```i
/// string.inspect([1, 2])  // "[1, 2]"
/// string.inspect("a")     // "\"a\""
/// ```
inspect = <A>(term: A) -> String { core.inspect(term) }

/// `None` for `""`, and `Some(string)` for any other string.
to_option = (string: String) -> Option(String) {
	string |> ("") { None } | (_) { Some(string) }
}

/// One scalar as a string. Negative indexes count from the end; indexes
/// outside the string, including very negative ones, give None.
at = (string: String, index: Int) -> Option(String) {
	cs = core.codepoints(string)
	i = { index < 0 } |> (True) { list.length(cs) + index } | (False) { index }
	list.at(cs, i) |>
		(None) { None }
		| (Some(c)) { Some(core.from_codepoints([c])) }
}

scalar_prefix = (
	haystack: List(UtfCodepoint),
	needle: List(UtfCodepoint)
) -> Bool {
	needle |>
		([]) { True }
		| ([c, ..rest]) {
			haystack |>
				([]) { False }
				| ([h, ..tail]) { h == c && scalar_prefix(tail, rest) }
		}
}

search_scalars = (
	cs: List(UtfCodepoint),
	needle: List(UtfCodepoint),
	from: Int,
	last: Bool
) -> Option(Int) {
	n = list.length(cs)
	{ from < 0 || from > n } |>
		(True) { None }
		| (False) {
			needle |>
				([]) { Some(from) }
				| (_) {
					go = (
						rest: List(UtfCodepoint),
						i: Int,
						found: Option(Int)
					) -> Option(Int) {
						{ last && i > from } |>
							(True) { found }
							| (False) {
								hit = { last || i >= from } &&
									scalar_prefix(rest, needle)
								{ hit && core.not(last) } |>
									(True) { Some(i) }
									| (False) {
										next = hit |>
											(True) { Some(i) }
											| (False) { found }
										rest |>
											([]) { next }
											| ([_, ..tail]) {
												go(tail, i + 1, next)
											}
									}
							}
					}
					go(cs, 0, None)
				}
		}
}

/// Least full starting scalar index >= from; bounds must be in [0,length].
/// Empty find matches from. Overlaps are eligible.
index_of = (string: String, find: String, from: Int) -> Option(Int) {
	search_scalars(core.codepoints(string), core.codepoints(find), from, False)
}

/// Greatest full starting scalar index <= from; the match may end after from.
/// Empty find matches from; negative or past-end bounds give None.
last_index_of = (string: String, find: String, from: Int) -> Option(Int) {
	search_scalars(core.codepoints(string), core.codepoints(find), from, True)
}

/// Left-to-right nonoverlapping matches. Empty find has count zero.
count = (string: String, find: String) -> Int {
	needle = core.codepoints(find)
	needle |>
		([]) { 0 }
		| (_) {
			width = list.length(needle)
			go = (rest: List(UtfCodepoint), n: Int) -> Int {
				rest |>
					([]) { n }
					| ([_, ..tail]) {
						scalar_prefix(rest, needle) |>
							(True) { go(list.drop(rest, width), n + 1) }
							| (False) { go(tail, n) }
					}
			}
			go(core.codepoints(string), 0)
		}
}

/// First up_to scalars, clamping the count to [0,length].
take_start = (from: String, up_to: Int) -> String {
	core.from_codepoints(
		list.take(core.codepoints(from), up_to),
	)
}

/// Last up_to scalars, clamping the count to [0,length].
take_end = (from: String, up_to: Int) -> String {
	from_reversed(
		list.take(
			list.reverse(core.codepoints(from)),
			up_to,
		),
	)
}

/// Cuts at the last full separator, omitting it. Missing or empty on gives
/// None. Separators at either end leave an empty component.
split_once_last = (string: String, on: String) -> Option((String, String)) {
	needle = core.codepoints(on)
	needle |>
		([]) { None }
		| (_) {
			cs = core.codepoints(string)
			search_scalars(cs, needle, list.length(cs), True) |>
				(None) { None }
				| (Some(i)) {
					Some(
						(
							core.from_codepoints(list.take(cs, i)),
							core.from_codepoints(
								list.drop(cs, i + list.length(needle)),
							),
						),
					)
				}
		}
}

/// Replaces the first max(0,up_to) nonoverlapping matches. Empty each is a
/// no-op, and inserted text is never searched. Collects and concatenates once.
replace_n = (in: String, each: String, with: String, up_to: Int) -> String {
	{ up_to <= 0 || each == "" } |>
		(True) { in }
		| (False) {
			needle = core.codepoints(each)
			width = list.length(needle)
			go = (
				rest: List(UtfCodepoint),
				left: Int,
				fragments: List(String)
			) -> String {
				{ left == 0 } |>
					(True) {
						core.concat(
							list.reverse(
								[core.from_codepoints(rest), ..fragments],
							),
						)
					}
					| (False) {
						rest |>
							([]) { core.concat(list.reverse(fragments)) }
							| ([c, ..tail]) {
								scalar_prefix(rest, needle) |>
									(True) {
										go(
											list.drop(rest, width),
											left - 1,
											[with, ..fragments],
										)
									}
									| (False) {
										go(
											tail,
											left,
											[
												core.from_codepoints([c]),
												..fragments,
											],
										)
									}
							}
					}
			}
			go(core.codepoints(in), up_to, [])
		}
}

/// Calls on once per scalar, left to right. True separates pieces; separator
/// runs collapse and empty pieces are omitted. Errors stop traversal at once.
split_when = <E, N>(
	string: String,
	on: (UtfCodepoint) <- (Fail(E), ..N) -> Bool
) -> List(String) {
	go = (
		rest: List(UtfCodepoint),
		piece: List(UtfCodepoint),
		acc: List(String)
	) -> List(String) {
		rest |>
			([]) {
				piece |>
					([]) { list.reverse(acc) }
					| (_) { list.reverse([from_reversed(piece), ..acc]) }
			}
			| ([c, ..tail]) {
				on(c) |>
					(False) { go(tail, [c, ..piece], acc) }
					| (True) {
						piece |>
							([]) { go(tail, [], acc) }
							| (_) {
								go(tail, [], [from_reversed(piece), ..acc])
							}
					}
			}
	}
	go(core.codepoints(string), [], [])
}

/// Splits on Pattern_White_Space, as trim does. NBSP is not a separator.
words = (string: String) -> List(String) { split_when(string, is_space) }

/// Calls with once per scalar in order, producing one scalar per input.
map = <N>(string: String, with: (UtfCodepoint) <- N -> UtfCodepoint) -> String {
	core.from_codepoints(
		list.map(core.codepoints(string), with),
	)
}

/// Calls keeping once per scalar in order. Errors stop later calls.
filter = <N>(string: String, keeping: (UtfCodepoint) <- N -> Bool) -> String {
	core.from_codepoints(
		list.filter(core.codepoints(string), keeping),
	)
}

/// Folds left to right with (accumulator,scalar); empty input gives from.
fold = <A, N>(string: String, from: A, with: (A, UtfCodepoint) <- N -> A) -> A {
	list.fold(core.codepoints(string), from, with)
}
